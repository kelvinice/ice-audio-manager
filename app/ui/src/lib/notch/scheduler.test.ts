import assert from 'node:assert/strict';
import test from 'node:test';
import type { DeviceSummary, EngineSnapshot, NowPlaying } from '../ipc/engine.ts';
import {
  DEFAULT_IDLE_MODE,
  formatRgbCss,
  formatRgbGlowCss,
  formatTimecode,
  IDLE_SUSPEND_LINGER_MS,
  isMediaActive,
  mediaProgressPercent,
  NotchActivityScheduler,
  SWITCH_DURATION_MS,
  TRACK_CHANGE_DURATION_MS,
  VOLUME_DURATION_MS,
  type ActivityView,
  type IdleMode,
} from './scheduler.ts';
import { resolveDisplayedView } from './nowPlaying.ts';

function expectActivity<K extends ActivityView['kind']>(
  sched: NotchActivityScheduler,
  kind: K,
): Extract<ActivityView, { kind: K }> {
  assert.equal(sched.state.activity.kind, kind);
  return sched.state.activity as Extract<ActivityView, { kind: K }>;
}

function sampleDevice(overrides: Partial<DeviceSummary> = {}): DeviceSummary {
  return {
    id: 'fake-arctis',
    name: 'Headphones (Arctis Nova 7)',
    flow: 'render',
    state: 'active',
    isDefault: true,
    isDefaultCommunications: true,
    formFactor: 3,
    isVirtual: false,
    spatial: 'dolbyAtmosHeadphones',
    ...overrides,
  };
}

function sampleSnapshot(): EngineSnapshot {
  return {
    revision: 1,
    playback: [
      sampleDevice(),
      sampleDevice({
        id: 'fake-realtek',
        name: 'Speakers (Realtek(R) Audio)',
        isDefault: false,
        isDefaultCommunications: false,
        formFactor: 1,
        spatial: 'off',
      }),
    ],
    recording: [
      sampleDevice({
        id: 'fake-arctis-mic',
        name: 'Microphone (Arctis Nova 7)',
        flow: 'capture',
        formFactor: 4,
        spatial: null,
      }),
    ],
    disconnected: [],
    volume: { percent: 40, muted: false },
    micMuted: false,
    micVolume: 80,
    notices: [],
    settings: {
      defaultRoles: 'default',
      togglePairs: [],
      bindings: [],
      favouritesPlayback: [],
      favouritesRecording: [],
      devices: [],
      performanceProfile: 'instant',
      pinnedMonitor: null,
      knobGestures: false,
      showDisconnected: false,
      idleLingerSecs: 15,
      musicServices: { spotify: true, youtubeMusic: true },
      lyrics: { enabled: false, onlineLookup: false, showOnPill: true, showOnCard: true, wordHighlight: true, romaji: true, cacheMaxMb: 50 },
      visualizerSpectrum: true,
      showNotchHotkey: null,
      appearance: { theme: 'ice', preset: 'smooth', idleStyle: 'mini_pill', nowPlaying: 'brief', movingBackdrop: true },
    },
    media: null,
    lyrics: { status: 'disabled', trackKey: null, document: null },
  };
}

test('initial snapshot keeps idle state and D-03 revision guard rejects stale or gapped envelopes', () => {
  const sched = new NotchActivityScheduler(1000);
  sched.applySnapshot(sampleSnapshot(), 1000);
  assert.equal(sched.state.activity.kind, 'idle');
  assert.equal(sched.state.revision, 1);

  const stale = sched.applyEnvelope(
    { revision: 1, event: { type: 'volumeChanged', volume: { percent: 50, muted: false } } },
    1050,
  );
  assert.equal(stale.status, 'ignored_stale');
  assert.equal(sched.state.activity.kind, 'idle');

  const gap = sched.applyEnvelope(
    { revision: 4, event: { type: 'volumeChanged', volume: { percent: 50, muted: false } } },
    1060,
  );
  assert.equal(gap.status, 'needs_resnapshot');
});

test('repeat volume ticks merge without re-triggering entry pulse and detect 0%/100% bounds', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);

  const r1 = sched.applyEnvelope(
    { revision: 2, event: { type: 'volumeChanged', volume: { percent: 95, muted: false } } },
    100,
  );
  assert.equal(r1.status, 'applied');
  const v1 = expectActivity(sched, 'volume');
  const firstEntrySeq = v1.entrySeq;

  const r2 = sched.applyEnvelope(
    { revision: 3, event: { type: 'volumeChanged', volume: { percent: 100, muted: false } } },
    200,
  );
  assert.equal(r2.status, 'applied');
  const v2 = expectActivity(sched, 'volume');
  assert.equal(v2.entrySeq, firstEntrySeq, 'repeat tick must not replay entry pulse');
  assert.equal(v2.boundBump, 0);

  sched.applyEnvelope(
    { revision: 4, event: { type: 'volumeChanged', volume: { percent: 100, muted: false } } },
    260,
  );
  const v3 = expectActivity(sched, 'volume');
  assert.equal(v3.boundBump, 1);
  assert.equal(v3.expiresAtMs, 260 + VOLUME_DURATION_MS);
});

test('micVolumeChanged uses mic channel and preserves micMuted state', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);
  sched.applyEnvelope({ revision: 2, event: { type: 'micMuteChanged', muted: true } }, 100);
  sched.applyEnvelope({ revision: 3, event: { type: 'micVolumeChanged', percent: 62 } }, 200);
  const v = expectActivity(sched, 'volume');
  assert.equal(v.channel, 'mic');
  assert.equal(v.percent, 62);
  assert.equal(v.muted, true);
  assert.equal(sched.state.micMuted, true);
});

test('deviceArrived and switched for same device merge into a single switch notification', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);

  const usbDac = sampleDevice({
    id: 'fake-usb-dac',
    name: 'Headphones (USB DAC)',
    isDefault: false,
    isDefaultCommunications: false,
    spatial: 'windowsSonic',
  });

  sched.applyEnvelope({ revision: 2, event: { type: 'deviceArrived', device: usbDac } }, 500);
  const p = expectActivity(sched, 'plug');
  assert.equal(p.direction, 'arrived');

  sched.applyEnvelope(
    {
      revision: 3,
      event: {
        type: 'switched',
        device: usbDac,
        roles: 'default',
        relinked: true,
        spatialApplied: 'dolbyAtmosHeadphones',
      },
    },
    900,
  );
  const sw = expectActivity(sched, 'switch');
  assert.equal(sw.mergedPlug, true);
  assert.equal(sw.relinked, true);
  assert.equal(sw.spatialBadge, 'Atmos');
  assert.equal(sw.previousDevice, 'Headphones (Arctis Nova 7)');
  assert.equal(sw.expiresAtMs, 900 + SWITCH_DURATION_MS);

  // Reverse order (switched first, deviceArrived right after) preserves switch card and marks mergedPlug.
  const sched2 = new NotchActivityScheduler(0);
  sched2.applySnapshot(sampleSnapshot(), 0);
  sched2.applyEnvelope(
    {
      revision: 2,
      event: { type: 'switched', device: usbDac, roles: 'default', relinked: false, spatialApplied: null },
    },
    500,
  );
  sched2.applyEnvelope({ revision: 3, event: { type: 'deviceArrived', device: usbDac } }, 700);
  const sw2 = expectActivity(sched2, 'switch');
  assert.equal(sw2.mergedPlug, true);
  assert.equal(sw2.spatialBadge, null);
});

test('deviceLeft removes endpoint and triggers left plug activity', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);
  const removed = sampleDevice({ state: 'not_present', isDefault: false, isDefaultCommunications: false, spatial: null });
  sched.applyEnvelope({ revision: 2, event: { type: 'deviceLeft', device: removed } }, 400);
  assert.equal(sched.state.playback.some((d) => d.id === removed.id), false);
  const left = expectActivity(sched, 'plug');
  assert.equal(left.direction, 'left');
});

test('micMuteChanged leaves persistent static indicator on idle and schedules TrySuspend linger', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);
  assert.equal(sched.tick(IDLE_SUSPEND_LINGER_MS, false).shouldRequestSuspend, true);
  assert.equal(sched.tick(IDLE_SUSPEND_LINGER_MS + 1000, false).shouldRequestSuspend, false);

  sched.applyEnvelope({ revision: 2, event: { type: 'micMuteChanged', muted: true } }, 20_000);
  assert.equal(sched.state.activity.kind, 'mic');
  assert.equal(sched.state.micMuted, true);

  // While hovered/expanded, holdOpen blocks auto-dismiss and suspend.
  assert.equal(sched.tick(25_000, true).dismissedToIdle, false);
  assert.equal(sched.nextTickDelayMs(25_000, true), null);

  // Once released past expiry, dismisses to idle (keeping micMuted) and triggers suspend after linger.
  const exp = sched.tick(25_000, false);
  assert.equal(exp.dismissedToIdle, true);
  assert.equal(exp.shouldRequestSuspend, false);
  assert.equal(sched.state.activity.kind, 'idle');
  assert.equal(sched.state.micMuted, true);
  assert.equal(sched.nextTickDelayMs(25_000, false), IDLE_SUSPEND_LINGER_MS);

  const sus = sched.tick(25_000 + IDLE_SUSPEND_LINGER_MS, false);
  assert.equal(sus.shouldRequestSuspend, true);
});

test('noticesChanged and settingsChanged sync snapshot state silently without waking activity', () => {
  const initialIdleMs = 1000;
  const sched = new NotchActivityScheduler(initialIdleMs);
  const snap = sampleSnapshot();
  snap.settings = {
    ...snap.settings,
    togglePairs: [
      {
        id: 'main',
        hotkey: 'Ctrl+Alt+F12',
        primary: { name: 'Headphones (Arctis Nova 7)', flow: 'render', savedId: 'fake-arctis', deviceId: 'fake-arctis' },
        secondary: { name: 'Speakers (Realtek(R) Audio)', flow: 'render', savedId: 'fake-realtek', deviceId: 'fake-realtek' },
        roles: 'default',
      },
    ],
  };
  snap.notices = [{ type: 'notConnected', name: 'Old Device' }];

  sched.applySnapshot(snap, initialIdleMs);
  assert.deepEqual(sched.state.settings, snap.settings);
  assert.deepEqual(sched.state.notices, snap.notices);

  const noticesRes = sched.applyEnvelope(
    { revision: 2, event: { type: 'noticesChanged', notices: [{ type: 'hotkeyConflict', bindingId: 'main', keys: 'Ctrl+Alt+F12' }] } },
    1500,
  );
  assert.equal(noticesRes.status, 'applied');
  assert.equal(noticesRes.wokeActivity, false);
  assert.equal(sched.state.activity.kind, 'idle');
  assert.equal(sched.state.idleSinceMs, initialIdleMs);
  assert.deepEqual(sched.state.notices, [{ type: 'hotkeyConflict', bindingId: 'main', keys: 'Ctrl+Alt+F12' }]);

  const disconnectedSettings = {
    ...snap.settings,
    togglePairs: [
      {
        ...snap.settings.togglePairs[0],
        primary: { ...snap.settings.togglePairs[0].primary, deviceId: null },
      },
    ],
  };
  const playbackBefore = sched.state.playback;
  const settingsRes = sched.applyEnvelope(
    { revision: 3, event: { type: 'settingsChanged', settings: disconnectedSettings } },
    2000,
  );
  assert.equal(settingsRes.status, 'applied');
  assert.equal(settingsRes.wokeActivity, false);
  assert.equal(sched.state.activity.kind, 'idle');
  assert.equal(sched.state.idleSinceMs, initialIdleMs);
  assert.equal(sched.state.playback, playbackBefore);
  assert.equal(sched.state.settings?.togglePairs[0]?.primary.deviceId, null);
});

function sampleMedia(overrides: Partial<NowPlaying> = {}): NowPlaying {
  return {
    appId: 'Spotify.exe',
    status: 'paused',
    title: 'Viva La Vida',
    artist: 'Coldplay',
    album: 'Viva La Vida or Death and All His Friends',
    positionMs: 52_000n,
    durationMs: 211_000n,
    art: { id: 'fake-art-0', width: 256, height: 256, accent: { r: 226, g: 88, b: 62 } },
    ...overrides,
  };
}

test('snapshot.media populates compact idle state and mediaChanged wakes track activity only on track change', () => {
  const sched = new NotchActivityScheduler(1000);
  const snap = sampleSnapshot();
  snap.media = sampleMedia();
  sched.applySnapshot(snap, 1000);

  // Initial snapshot sets state.media without waking a track-change activity banner.
  assert.equal(sched.state.activity.kind, 'idle');
  assert.equal(isMediaActive(sched.state.media), true);
  assert.equal(sched.state.media?.title, 'Viva La Vida');

  // Same-track play/pause delta updates status in place without waking a track-change banner from idle.
  const playRes = sched.applyEnvelope(
    {
      revision: 2,
      event: {
        type: 'mediaChanged',
        media: sampleMedia({ status: 'playing', positionMs: 55_000n }),
      },
    },
    1200,
  );
  assert.equal(playRes.status, 'applied');
  assert.equal(playRes.wokeActivity, false);
  assert.equal(sched.state.activity.kind, 'idle');
  assert.equal(sched.state.media?.status, 'playing');

  // Next track wakes the track-change activity with previous track metadata for fixed-slot crossfade.
  const nextTrack = sampleMedia({
    status: 'playing',
    title: 'Midnight City',
    artist: 'M83',
    album: "Hurry Up, We're Dreaming",
    positionMs: 0n,
    durationMs: 243_000n,
    art: { id: 'fake-art-1', width: 256, height: 256, accent: { r: 196, g: 82, b: 212 } },
  });
  const trackRes = sched.applyEnvelope(
    { revision: 3, event: { type: 'mediaChanged', media: nextTrack } },
    1500,
  );
  assert.equal(trackRes.status, 'applied');
  assert.equal(trackRes.wokeActivity, true);
  const tr1 = expectActivity(sched, 'track');
  assert.equal(tr1.media.title, 'Midnight City');
  assert.equal(tr1.previousTitle, 'Viva La Vida');
  assert.equal(tr1.previousArtist, 'Coldplay');
  assert.equal(tr1.previousArt?.id, 'fake-art-0');
  assert.equal(tr1.expiresAtMs, 1500 + TRACK_CHANGE_DURATION_MS);
  const firstTrackSeq = tr1.trackSeq;

  // Same-track timeline or pause update while track activity is open keeps trackSeq stable so controls/labels do not replay.
  const pauseRes = sched.applyEnvelope(
    {
      revision: 4,
      event: {
        type: 'mediaChanged',
        media: { ...nextTrack, status: 'paused', positionMs: 12_000n },
      },
    },
    1800,
  );
  assert.equal(pauseRes.status, 'applied');
  assert.equal(pauseRes.wokeActivity, false);
  const tr2 = expectActivity(sched, 'track');
  assert.equal(tr2.trackSeq, firstTrackSeq);
  assert.equal(tr2.media.status, 'paused');

  // Media closing or stopping dismisses an active track banner back to idle.
  const closeRes = sched.applyEnvelope(
    { revision: 5, event: { type: 'mediaChanged', media: null } },
    2000,
  );
  assert.equal(closeRes.status, 'applied');
  assert.equal(closeRes.wokeActivity, false);
  assert.equal(sched.state.activity.kind, 'idle');
  assert.equal(sched.state.media, null);
});

test('media helpers format timecodes, scrubber percentage, and accent colours', () => {
  assert.equal(formatTimecode(52_000n), '0:52');
  assert.equal(formatTimecode(211_000), '3:31');
  assert.equal(formatTimecode(null), '0:00');

  const media = sampleMedia({ positionMs: 52_750n, durationMs: 211_000n });
  assert.equal(Math.round(mediaProgressPercent(media)), 25);
  assert.equal(mediaProgressPercent(sampleMedia({ durationMs: 0n })), 0);

  assert.equal(formatRgbCss({ r: 226, g: 88, b: 62 }), 'rgb(226, 88, 62)');
  assert.equal(formatRgbGlowCss({ r: 226, g: 88, b: 62 }, 0.24), 'rgba(226, 88, 62, 0.24)');
  assert.equal(formatRgbCss(null), null);

  const modes: IdleMode[] = ['mini_pill', 'dot', 'hidden'];
  assert.equal(DEFAULT_IDLE_MODE, 'mini_pill');
  assert.equal(modes.includes(DEFAULT_IDLE_MODE), true);
});

test('now playing pill: off never pops up, while_playing stays while playing and shrinks on pause', () => {
  const withMode = (nowPlaying: 'brief' | 'while_playing' | 'off') => {
    const snap = sampleSnapshot();
    snap.settings.appearance.nowPlaying = nowPlaying;
    snap.media = sampleMedia({ status: 'playing', title: 'A', artist: 'X' });
    const sched = new NotchActivityScheduler(0);
    sched.applySnapshot(snap, 0);
    return sched;
  };
  const next = sampleMedia({ status: 'playing', title: 'B', artist: 'Y' });

  // Off: the title updates but no track activity and nothing to wake.
  const off = withMode('off');
  const offRes = off.applyEnvelope({ revision: 2, event: { type: 'mediaChanged', media: next } }, 100);
  assert.equal(offRes.wokeActivity, false);
  assert.equal(off.state.activity.kind, 'idle');
  assert.equal(off.state.media?.title, 'B');

  // While playing: the pop-up expires as usual, then the idle notch keeps showing the same pill (same trackSeq).
  const wp = withMode('while_playing');
  wp.applyEnvelope({ revision: 2, event: { type: 'mediaChanged', media: next } }, 100);
  const pop = expectActivity(wp, 'track');
  wp.tick(100 + TRACK_CHANGE_DURATION_MS, false);
  assert.equal(wp.state.activity.kind, 'idle');
  const shown = resolveDisplayedView(wp.state.activity, wp.state.media, 'while_playing', wp.lastTrackSeq);
  assert.equal(shown.kind, 'track');
  assert.equal(shown.kind === 'track' && shown.trackSeq, pop.trackSeq, 'no replayed entrance animation');
  const paused = { ...next, status: 'paused' as const };
  assert.equal(resolveDisplayedView(wp.state.activity, paused, 'while_playing', wp.lastTrackSeq).kind, 'idle');

  // Brief (default) shrinks back to idle once the pop-up expires.
  assert.equal(resolveDisplayedView({ kind: 'idle' }, next, 'brief', 1).kind, 'idle');
});

test('idle linger follows settings.idleLingerSecs (D-21.1 L1)', () => {
  const snap = sampleSnapshot();
  snap.settings.idleLingerSecs = 60;
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(snap, 0);
  assert.equal(sched.nextTickDelayMs(0, false), 60_000);
  assert.equal(sched.tick(IDLE_SUSPEND_LINGER_MS, false).shouldRequestSuspend, false, 'the 15 s default no longer applies');
  assert.equal(sched.tick(60_000, false).shouldRequestSuspend, true);
});
