import assert from 'node:assert/strict';
import test from 'node:test';
import type { EngineSnapshot, Envelope, SettingsView } from '../ipc/engine.ts';
import { applyEnvelope, lyricsDelayChoice, lyricsDelayFromChoice, moveItem, nextId, recordKeys } from './model.ts';

const settings: SettingsView = {
  appearance: { theme: 'ice', preset: 'smooth', idleStyle: 'mini_pill', notchStyle: 'floating', nowPlaying: 'brief', movingBackdrop: true, hibernate: false, hibernateLook: 'plain', hibernateHideFromCapture: true },
  performanceProfile: 'instant',
  pinnedMonitor: null,
  defaultRoles: 'default',
  togglePairs: [],
  bindings: [],
  favouritesPlayback: [],
  favouritesRecording: [],
  devices: [],
  knobGestures: false,
  showDisconnected: false,
  idleLingerSecs: 15,
  musicServices: { spotify: true, youtubeMusic: true },
  lyrics: { enabled: false, onlineLookup: false, showOnPill: true, showOnCard: true, wordHighlight: true, romaji: true, cacheMaxMb: 50 },
  visualizerSpectrum: true,
  showNotchHotkey: null,
};

const snapshot: EngineSnapshot = {
  revision: 5,
  playback: [],
  recording: [],
  disconnected: [],
  volume: null,
  micMuted: null,
  micVolume: null,
  notices: [],
  settings,
  media: null,
  lyrics: { status: 'disabled', trackKey: null, document: null },
};

const settingsChanged = (revision: number): Envelope => ({
  revision,
  event: { type: 'settingsChanged', settings: { ...settings, knobGestures: true } },
});

test('applies the next revision, ignores stale ones and re-snapshots on a gap', () => {
  const next = applyEnvelope(snapshot, settingsChanged(6));
  assert.equal(next.snapshot.revision, 6);
  assert.equal(next.snapshot.settings.knobGestures, true);
  assert.equal(next.resnapshot, false);

  assert.equal(applyEnvelope(snapshot, settingsChanged(5)).snapshot, snapshot);
  assert.equal(applyEnvelope(snapshot, settingsChanged(8)).resnapshot, true);
});

test('records chords in the engine format and refuses ones that would block typing', () => {
  const key = (code: string, mods: Partial<Record<'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey', boolean>> = {}) => ({
    code,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    ...mods,
  });
  assert.deepEqual(recordKeys(key('F12', { ctrlKey: true, altKey: true })), { kind: 'chord', chord: 'Ctrl+Alt+F12' });
  assert.deepEqual(recordKeys(key('AudioVolumeUp', { ctrlKey: true })), { kind: 'chord', chord: 'Ctrl+VolumeUp' });
  assert.deepEqual(recordKeys(key('ControlLeft', { ctrlKey: true })), { kind: 'partial', parts: ['Ctrl'] });
  assert.equal(recordKeys(key('KeyA', { shiftKey: true })).kind, 'invalid');
  assert.deepEqual(recordKeys(key('F13')), { kind: 'chord', chord: 'F13' });
});

test('moves list items within bounds and picks unused ids', () => {
  assert.deepEqual(moveItem(['a', 'b', 'c'], 2, -1), ['a', 'c', 'b']);
  assert.equal(moveItem(['a', 'b'], 0, -1), null);
  assert.equal(nextId('pair', ['pair-2', 'main']), 'pair-3');
});

test('lyrics delay select: Auto shows the automatic value, a hand-saved value stays listed', () => {
  const auto = lyricsDelayChoice(null, 200);
  assert.equal(auto.value, 'auto');
  assert.equal(auto.options[0]?.label, 'Auto (+200 ms)');
  assert.ok(auto.options.some((o) => o.value === '0' && o.label === 'None (0 ms)'));
  const custom = lyricsDelayChoice(180, 180);
  assert.equal(custom.value, '180');
  assert.equal(custom.options[0]?.label, 'Auto');
  assert.ok(custom.options.some((o) => o.value === '180' && o.label === '+180 ms'), 'kept');
  assert.equal(lyricsDelayFromChoice('auto'), null);
  assert.equal(lyricsDelayFromChoice('-100'), -100);
  assert.equal(lyricsDelayFromChoice('x'), null);
});
