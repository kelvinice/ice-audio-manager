import assert from 'node:assert/strict';
import test from 'node:test';
import type { NowPlaying, Rgb, SettingsView } from '../ipc/engine.ts';
import {
  adaptAccentForSurface,
  capLuminance,
  compositeOver,
  computeArtBackdrop,
  contrastRatio,
  relativeLuminance,
  resolveAmbientMotionState,
  resolveMusicServices,
  type ArtInfoWithPalette,
  WASH_MAX_LUMINANCE,
} from './backdrop.ts';

function parseRgba(css: string): { rgb: Rgb; alpha: number } {
  const match = css.match(/rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([\d.]+)\s*\)/);
  assert.ok(match, `expected rgba(...) string, got: ${css}`);
  return {
    rgb: { r: Number(match[1]), g: Number(match[2]), b: Number(match[3]) },
    alpha: Number(match[4]),
  };
}

const baseSettings: SettingsView = {
  appearance: { theme: 'ice', preset: 'smooth', idleStyle: 'mini_pill', notchStyle: 'floating', nowPlaying: 'brief', movingBackdrop: true, hibernate: false, hibernateLook: 'plain', hibernateHideFromCapture: true },
  performanceProfile: 'instant',
  pinnedMonitor: null,
  defaultRoles: 'both',
  togglePairs: [],
  bindings: [],
  favouritesPlayback: [],
  favouritesRecording: [],
  devices: [],
  knobGestures: true,
  showDisconnected: true,
  idleLingerSecs: 15,
  musicServices: { spotify: true, youtubeMusic: true },
  lyrics: { enabled: false, onlineLookup: false, showOnPill: true, showOnCard: true, wordHighlight: true, romaji: true, cacheMaxMb: 50 },
  visualizerSpectrum: true,
  showNotchHotkey: null,
};

test('computeArtBackdrop derives 2-stop gradient washes and keeps WCAG AA scrim contrast on bright covers', () => {
  const none = computeArtBackdrop(null);
  assert.equal(none.hasArt, false);
  assert.equal(none.artId, null);

  // Dark cover from existing single-accent ArtInfo derives a distinct secondary stop and lighter top scrim.
  const darkArt: ArtInfoWithPalette = {
    id: 'art-dark',
    width: 128,
    height: 128,
    accent: { r: 226, g: 78, b: 64 },
    luminance: 0.08,
  };
  const darkBackdrop = computeArtBackdrop(darkArt);
  assert.equal(darkBackdrop.hasArt, true);
  assert.equal(darkBackdrop.artId, 'art-dark');
  assert.notEqual(darkBackdrop.washPrimaryCss, darkBackdrop.washSecondaryCss);
  assert.notEqual(darkBackdrop.washTertiaryCss, darkBackdrop.washQuaternaryCss);
  assert.ok(darkBackdrop.washBaseCss.startsWith('linear-gradient('), 'provides subtle base wash gradient');

  // Pure white / bright cover raises the top & bottom scrim so muted fg (#98a4b8) stays >= 4.5:1 WCAG AA.
  const brightArt: ArtInfoWithPalette = {
    id: 'art-bright',
    width: 128,
    height: 128,
    accent: { r: 107, g: 197, b: 255 },
    palette: [
      { r: 245, g: 248, b: 252 },
      { r: 188, g: 227, b: 255 },
    ],
    luminance: 0.95,
  };
  const brightBackdrop = computeArtBackdrop(brightArt);
  const topScrim = parseRgba(brightBackdrop.scrimTopCss);
  const darkScrimTop = parseRgba(darkBackdrop.scrimTopCss);
  assert.ok(topScrim.alpha > darkScrimTop.alpha, 'bright covers must strengthen dark scrim');

  const whiteCover: Rgb = { r: 245, g: 248, b: 252 };
  const compositedSurface = compositeOver(topScrim.rgb, topScrim.alpha, whiteCover);
  const mutedFg: Rgb = { r: 152, g: 164, b: 184 };
  assert.ok(
    contrastRatio(mutedFg, compositedSurface) >= 4.5,
    `muted foreground contrast over clamped bright cover should be >= 4.5, got ${contrastRatio(mutedFg, compositedSurface).toFixed(2)}`,
  );

  // V2 wash blobs over dark Ice surface keep muted text >= 4.5:1 even when derived from a bright cover.
  const darkBase: Rgb = { r: 14, g: 17, b: 22 };
  const brightWash = parseRgba(brightBackdrop.washPrimaryCss);
  const compositedWash = compositeOver(brightWash.rgb, brightWash.alpha, darkBase);
  assert.ok(
    contrastRatio(mutedFg, compositedWash) >= 4.5,
    `muted foreground over dark V2 wash should stay >= 4.5, got ${contrastRatio(mutedFg, compositedWash).toFixed(2)}`,
  );

  // Ice Light theme adapts pastel accents and uses a frosted light scrim for dark covers.
  const lightBase: Rgb = { r: 246, g: 247, b: 249 };
  const darkMuted: Rgb = { r: 75, g: 86, b: 104 };
  const lightDarkCover = computeArtBackdrop(darkArt, true);
  const lightScrim = parseRgba(lightDarkCover.scrimTopCss);
  assert.ok(lightScrim.rgb.r > 230, 'light theme uses a frosted light scrim');
  assert.ok(contrastRatio(darkMuted, compositeOver(lightScrim.rgb, lightScrim.alpha, { r: 12, g: 15, b: 20 })) >= 4.5);

  const adapted = adaptAccentForSurface({ r: 253, g: 230, b: 138 }, true);
  assert.ok(contrastRatio(adapted, lightBase) >= 4.0, 'light theme deepens pastel accents for readable controls');
});

test('resolveMusicServices enables Spotify and YouTube Music by default and tracks active playback app', () => {
  const defaultServices = resolveMusicServices(baseSettings, null);
  assert.deepEqual(
    defaultServices.map((s) => ({ id: s.id, active: s.active })),
    [
      { id: 'spotify', active: false },
      { id: 'youtube_music', active: false },
    ],
  );

  const playingSpotify: NowPlaying = {
    appId: 'Spotify.exe',
    title: 'Canopy',
    artist: 'Auralis Sessions',
    album: '',
    status: 'playing',
    positionMs: null,
    durationMs: null,
    art: null,
  };
  const whileSpotify = resolveMusicServices(baseSettings, playingSpotify);
  assert.equal(whileSpotify[0].active, true);
  assert.equal(whileSpotify[1].active, false);

  const withYtmDisabled = resolveMusicServices(
    { ...baseSettings, musicServices: { spotify: true, youtubeMusic: false } },
    { ...playingSpotify, appId: 'chrome.exe' },
  );
  assert.deepEqual(
    withYtmDisabled.map((s) => s.id),
    ['spotify'],
  );
});

test('resolveAmbientMotionState enforces expanded/music-pill, C1, C2, and C3 lifecycle gates', () => {
  const base = {
    hasArt: true,
    isExpanded: true,
    isTrackPill: false,
    isVisible: true,
    isInstant: false,
    isPlaying: true,
    movingBackdrop: true,
  };
  assert.equal(resolveAmbientMotionState(base), 'running');
  assert.equal(resolveAmbientMotionState({ ...base, isExpanded: false, isTrackPill: true }), 'running');
  assert.equal(resolveAmbientMotionState({ ...base, isPlaying: false }), 'paused');
  assert.equal(resolveAmbientMotionState({ ...base, isExpanded: false, isTrackPill: false }), 'stopped');
  assert.equal(resolveAmbientMotionState({ ...base, isVisible: false }), 'stopped');
  assert.equal(resolveAmbientMotionState({ ...base, hasArt: false }), 'stopped');
  assert.equal(resolveAmbientMotionState({ ...base, isInstant: true }), 'stopped');
  assert.equal(resolveAmbientMotionState({ ...base, movingBackdrop: false }), 'stopped');
});

test('capLuminance dims a colour to the cap and keeps its hue', () => {
  const yellow = { r: 210, g: 190, b: 4 };
  const capped = capLuminance(yellow, WASH_MAX_LUMINANCE);
  assert.ok(relativeLuminance(capped) <= WASH_MAX_LUMINANCE, `${JSON.stringify(capped)}`);
  assert.ok(capped.r > capped.b && capped.g > capped.b, 'still yellow');
  const dark = { r: 40, g: 20, b: 60 };
  assert.deepEqual(capLuminance(dark, WASH_MAX_LUMINANCE), dark, 'already dark: unchanged');
});

test('dark-theme lava orbs stay dim enough for text whatever the cover', () => {
  const fg = { r: 237, g: 241, b: 247 };
  const muted = { r: 152, g: 164, b: 184 };
  const covers: Rgb[][] = [
    [{ r: 250, g: 210, b: 80 }, { r: 240, g: 120, b: 200 }, { r: 120, g: 220, b: 255 }, { r: 255, g: 250, b: 240 }],
    [{ r: 90, g: 255, b: 60 }, { r: 255, g: 255, b: 0 }, { r: 0, g: 255, b: 255 }, { r: 255, g: 255, b: 255 }],
  ];
  for (const palette of covers) {
    const art: ArtInfoWithPalette = { id: 'a', width: 128, height: 128, accent: palette[0], palette, luminance: 0.9 };
    const bd = computeArtBackdrop(art, false);
    for (const css of [bd.washPrimaryCss, bd.washSecondaryCss, bd.washTertiaryCss, bd.washQuaternaryCss, bd.washHighlightCss]) {
      // `lighten` stacking tends to the opaque colour: that is the worst case behind text.
      const { rgb } = parseRgba(css);
      assert.ok(relativeLuminance(rgb) <= WASH_MAX_LUMINANCE + 1e-9, css);
      assert.ok(contrastRatio(fg, rgb) >= 4.5, `fg on ${css}`);
      assert.ok(contrastRatio(muted, rgb) >= 3, `muted on ${css}`);
    }
  }
});
