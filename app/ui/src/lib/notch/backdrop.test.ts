import assert from 'node:assert/strict';
import test from 'node:test';
import type { NowPlaying, Rgb, SettingsView } from '../ipc/engine.ts';
import {
  compositeOver,
  computeArtBackdrop,
  contrastRatio,
  resolveMusicServices,
  type ArtInfoWithPalette,
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
  appearance: { theme: 'ice', preset: 'smooth', idleStyle: 'mini_pill', nowPlaying: 'brief' },
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
