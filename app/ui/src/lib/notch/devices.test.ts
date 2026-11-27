import assert from 'node:assert/strict';
import test from 'node:test';
import type { DeviceSummary, SavedDevice, SettingsView } from '../ipc/engine.ts';
import {
  buildNotchDeviceRow,
  isDeviceFavourite,
  toggleFavouriteDeviceIds,
} from './devices.ts';

function dev(overrides: Partial<DeviceSummary> & { id: string; name: string }): DeviceSummary {
  return {
    flow: 'render',
    state: 'active',
    isDefault: false,
    isDefaultCommunications: false,
    formFactor: 1,
    isVirtual: false,
    lyricsDelayMs: 0,
    spatial: 'off',
    ...overrides,
  };
}

function saved(name: string, savedId: string, deviceId: string | null = savedId): SavedDevice {
  return { name, flow: 'render', savedId, deviceId };
}

function baseSettings(overrides: Partial<SettingsView> = {}): SettingsView {
  return {
    appearance: { theme: 'ice', preset: 'smooth', idleStyle: 'mini_pill', notchStyle: 'floating', nowPlaying: 'brief', movingBackdrop: true, hibernate: false, hibernateLook: 'plain', hibernateHideFromCapture: true, dragEffects: { landingRipple: false, jellyLanding: false, carryGlow: false }, wavyProgress: false, volumeButton: false, experiments: { beatRim: false, coverTilt: false } },
    performanceProfile: 'instant',
    pinnedMonitor: null,
    notchAnchor: 'top_center',
    notchXRatio: null,
    notchPlacement: 'drag_snap',
    defaultRoles: 'default',
    togglePairs: [],
    bindings: [],
    favouritesPlayback: [],
    favouritesRecording: [],
    devices: [],
    knobGestures: true,
    showDisconnected: false,
    idleLingerSecs: 15,
    musicServices: { spotify: true, youtubeMusic: true },
    lyrics: { enabled: false, onlineLookup: false, showOnPill: true, showOnCard: true, wordHighlight: true, estimatedWordSync: false, romaji: true, cacheMaxMb: 50 },
    visualizerSpectrum: true,

    callMode: { enabled: false, apps: ['Discord.exe'], duckPercent: 30, holdSecs: 3, sensitivity: 'normal' },
    showNotchHotkey: null,
    widget: { enabled: false, buttons: ['pin', 'mic', 'settings'] },
    ...overrides,
  };
}

test('buildNotchDeviceRow follows favourites order, uses playback alias name, and prepends non-favourite current default', () => {
  const playback = [
    dev({ id: 'lg', name: 'LG FULL HD (NVIDIA)', isDefault: true, formFactor: 9 }),
    dev({ id: 'realtek', name: 'Desk Speakers', formFactor: 1 }),
    dev({ id: 'arctis', name: 'Headphones (Arctis Nova 7)', formFactor: 3 }),
  ];
  const settings = baseSettings({
    favouritesPlayback: [
      saved('Speakers (Realtek(R) Audio)', 'realtek', 'realtek'),
      saved('Headphones (Arctis Nova 7)', 'arctis', 'arctis'),
    ],
    devices: [
      { device: saved('Speakers (Realtek(R) Audio)', 'realtek', 'realtek'), spatial: 'off', alias: 'Desk Speakers', hidden: false, lyricsDelayMs: null },
    ],
  });

  const row = buildNotchDeviceRow(playback, settings);
  assert.deepEqual(
    row.chips.map((c) => [c.id, c.label, c.isCurrent, c.disconnected]),
    [
      ['lg', 'LG FULL HD', true, false],
      ['realtek', 'Desk Speakers', false, false],
      ['arctis', 'Arctis Nova 7', false, false],
    ],
  );
});

test('disconnected favourites are hidden unless showDisconnected is on, and hidden devices never appear', () => {
  const playback = [
    dev({ id: 'realtek', name: 'Speakers (Realtek(R) Audio)', isDefault: true }),
    dev({ id: 'hidden-hdmi', name: 'Monitor (Hidden)' }),
  ];
  const disconnected = [dev({ id: 'front-jack', name: 'Headphones (Front Jack)', state: 'unplugged', formFactor: 3 })];
  const baseFavs = [
    saved('Headphones (Front Jack)', 'front-jack', null),
    saved('Monitor (Hidden)', 'hidden-hdmi', 'hidden-hdmi'),
    saved('Speakers (Realtek(R) Audio)', 'realtek', 'realtek'),
  ];
  const prefs = [
    { device: saved('Monitor (Hidden)', 'hidden-hdmi', 'hidden-hdmi'), spatial: 'off' as const, alias: null, hidden: true, lyricsDelayMs: null },
  ];

  const hiddenOff = buildNotchDeviceRow(
    playback,
    baseSettings({ favouritesPlayback: baseFavs, devices: prefs, showDisconnected: false }),
    disconnected,
  );
  assert.deepEqual(
    hiddenOff.chips.map((c) => [c.id, c.disconnected]),
    [['realtek', false]],
  );

  const shownOn = buildNotchDeviceRow(
    playback,
    baseSettings({ favouritesPlayback: baseFavs, devices: prefs, showDisconnected: true }),
    disconnected,
  );
  assert.deepEqual(
    shownOn.chips.map((c) => [c.id, c.fullName, c.disconnected, c.formFactor]),
    [
      ['front-jack', 'Headphones (Front Jack)', true, 3],
      ['realtek', 'Speakers (Realtek(R) Audio)', false, 1],
    ],
  );
});

test('no favourites falls back to current device + togglePairs[0] partner and no picker button, never all devices', () => {
  const playback = [
    dev({ id: 'arctis', name: 'Headphones (Arctis Nova 7)', isDefault: true, formFactor: 3 }),
    dev({ id: 'realtek', name: 'Speakers (Realtek(R) Audio)', formFactor: 1 }),
    dev({ id: 'lg', name: 'LG FULL HD', formFactor: 9 }),
    dev({ id: 'asus', name: 'ASUS VP249', formFactor: 9 }),
  ];
  const settings = baseSettings({
    favouritesPlayback: [],
    togglePairs: [
      {
        id: 'main',
        hotkey: 'Ctrl+Alt+F12',
        primary: saved('Headphones (Arctis Nova 7)', 'arctis', 'arctis'),
        secondary: saved('Speakers (Realtek(R) Audio)', 'realtek', 'realtek'),
        roles: null,
      },
    ],
  });

  const row = buildNotchDeviceRow(playback, settings);
  assert.deepEqual(
    row.chips.map((c) => c.id),
    ['arctis', 'realtek'],
  );

  const noPair = buildNotchDeviceRow(playback, baseSettings({ favouritesPlayback: [], togglePairs: [] }));
  assert.deepEqual(
    noPair.chips.map((c) => c.id),
    ['arctis'],
  );
});

test('caps visible chips at 4 and preserves disconnected savedId order when toggling favourites', () => {
  const playback = [
    dev({ id: 'd1', name: 'Dev 1', isDefault: true }),
    dev({ id: 'd2', name: 'Dev 2' }),
    dev({ id: 'd3', name: 'Dev 3' }),
    dev({ id: 'd4', name: 'Dev 4' }),
    dev({ id: 'd5', name: 'Dev 5' }),
  ];
  const favs = playback.map((d) => saved(d.name, d.id, d.id));
  const row = buildNotchDeviceRow(playback, baseSettings({ favouritesPlayback: favs }));
  assert.equal(row.chips.length, 4);

  const mixedFavs = [
    saved('Arctis', 'saved-arctis', 'live-arctis'),
    saved('Front Jack', 'saved-jack', null),
    saved('Speakers', 'saved-realtek', 'live-realtek'),
  ];
  assert.equal(isDeviceFavourite(mixedFavs, 'live-arctis'), true);
  assert.equal(isDeviceFavourite(mixedFavs, 'lg'), false);
  assert.deepEqual(toggleFavouriteDeviceIds(mixedFavs, 'lg'), [
    'saved-arctis',
    'saved-jack',
    'saved-realtek',
    'lg',
  ]);
  assert.deepEqual(toggleFavouriteDeviceIds(mixedFavs, 'live-arctis'), [
    'saved-jack',
    'saved-realtek',
  ]);
});
