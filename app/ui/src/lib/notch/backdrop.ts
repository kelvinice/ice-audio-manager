import type { ArtInfo, MusicServiceId, NowPlaying, Rgb, SettingsView } from '../ipc/engine.ts';
import { formatRgbGlowCss, isMediaActive } from './scheduler.ts';

export type { MusicServiceId } from '../ipc/engine.ts';

export interface MusicServiceItem {
  id: MusicServiceId;
  label: string;
  shortLabel: string;
  active: boolean;
}

/**
 * Forward-compatible extension for D-23 art metadata (`palette` and cover `luminance`)
 * until `ice_core::media::ArtInfo` exports them via `ts-rs`.
 */
export interface ArtInfoWithPalette extends ArtInfo {
  palette?: Rgb[] | null;
  secondary?: Rgb | null;
  luminance?: number | null;
}

export interface ArtBackdropStyle {
  hasArt: boolean;
  artId: string | null;
  washPrimaryCss: string;
  washSecondaryCss: string;
  washTertiaryCss: string | null;
  scrimTopCss: string;
  scrimBottomCss: string;
  coverLuminance: number;
}

export type AmbientMotionState = 'running' | 'paused' | 'stopped';

export interface AmbientMotionInput {
  hasArt: boolean;
  isExpanded: boolean;
  isVisible: boolean;
  isInstant: boolean;
  isPlaying: boolean;
  movingBackdrop: boolean;
}

/**
 * Resolves the D-25 ambient backdrop animation lifecycle:
 * - `'stopped'` when collapsed (`B1`), hidden/suspended (`C2`), reduce motion/`Off` (`D-13`), no art, or user disabled (`C3`);
 * - `'paused'` when visible & expanded with playback paused (`C1` freezes phase in place);
 * - `'running'` only while visible, expanded, and playing.
 */
export function resolveAmbientMotionState(input: AmbientMotionInput): AmbientMotionState {
  if (!input.hasArt || !input.isExpanded || !input.isVisible || input.isInstant || !input.movingBackdrop) {
    return 'stopped';
  }
  return input.isPlaying ? 'running' : 'paused';
}

const DEFAULT_RGB: Rgb = { r: 107, g: 197, b: 255 };
const DEFAULT_SECONDARY_RGB: Rgb = { r: 58, g: 132, b: 199 };
const DARK_SCRIM_RGB: Rgb = { r: 12, g: 15, b: 20 };
const LIGHT_FG_RGB: Rgb = { r: 237, g: 241, b: 247 };
const MUTED_FG_RGB: Rgb = { r: 152, g: 164, b: 184 };

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}

function srgbToLinear(channel: number): number {
  const c = clamp(channel / 255, 0, 1);
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

/** WCAG 2.2 relative luminance (`0.0` black .. `1.0` white). */
export function relativeLuminance(rgb: Rgb): number {
  return 0.2126 * srgbToLinear(rgb.r) + 0.7152 * srgbToLinear(rgb.g) + 0.0722 * srgbToLinear(rgb.b);
}

/** WCAG 2.2 contrast ratio (`1.0` .. `21.0`). */
export function contrastRatio(a: Rgb, b: Rgb): number {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  const hi = Math.max(la, lb);
  const lo = Math.min(la, lb);
  return (hi + 0.05) / (lo + 0.05);
}

/** Composites `fg` with alpha `alpha` over opaque `bg`. */
export function compositeOver(fg: Rgb, alpha: number, bg: Rgb): Rgb {
  const a = clamp(alpha, 0, 1);
  return {
    r: Math.round(fg.r * a + bg.r * (1 - a)),
    g: Math.round(fg.g * a + bg.g * (1 - a)),
    b: Math.round(fg.b * a + bg.b * (1 - a)),
  };
}

function rgbToHsl(rgb: Rgb): [number, number, number] {
  const r = clamp(rgb.r / 255, 0, 1);
  const g = clamp(rgb.g / 255, 0, 1);
  const b = clamp(rgb.b / 255, 0, 1);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return [h * 360, s, l];
}

function hslToRgb(h: number, s: number, l: number): Rgb {
  const hue = ((h % 360) + 360) % 360;
  const sat = clamp(s, 0, 1);
  const lig = clamp(l, 0, 1);
  if (sat === 0) {
    const v = Math.round(lig * 255);
    return { r: v, g: v, b: v };
  }
  const c = (1 - Math.abs(2 * lig - 1)) * sat;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = lig - c / 2;
  let r1 = 0;
  let g1 = 0;
  let b1 = 0;
  if (hue < 60) [r1, g1, b1] = [c, x, 0];
  else if (hue < 120) [r1, g1, b1] = [x, c, 0];
  else if (hue < 180) [r1, g1, b1] = [0, c, x];
  else if (hue < 240) [r1, g1, b1] = [0, x, c];
  else if (hue < 300) [r1, g1, b1] = [x, 0, c];
  else [r1, g1, b1] = [c, 0, x];
  return {
    r: Math.round((r1 + m) * 255),
    g: Math.round((g1 + m) * 255),
    b: Math.round((b1 + m) * 255),
  };
}

/** Derives a warm/cool secondary gradient stop when `ArtInfo` provides a single accent. */
export function deriveSecondaryStop(primary: Rgb): Rgb {
  const [h, s, l] = rgbToHsl(primary);
  if (s < 0.08) return DEFAULT_SECONDARY_RGB;
  return hslToRgb(h + 28, clamp(s * 0.9, 0.25, 0.85), clamp(l * 0.72, 0.18, 0.56));
}

function scrimAlphasForLuminance(lum: number): { top: number; bottom: number } {
  const l = clamp(lum, 0, 1);
  // Bright/white covers raise the dark scrim so `--ice-fg` and `--ice-fg-muted` hold WCAG AA (>= 4.5:1).
  let top = clamp(0.52 + l * 0.30, 0.52, 0.82);
  let bottom = clamp(0.80 + l * 0.14, 0.80, 0.94);

  // Even against pure white blurred art beneath `--ice-art-backdrop: 0.85`, verify muted text meets >= 4.5:1.
  const coverRef: Rgb = {
    r: Math.round(l * 255),
    g: Math.round(l * 255),
    b: Math.round(l * 255),
  };
  while (top < 0.94 && contrastRatio(MUTED_FG_RGB, compositeOver(DARK_SCRIM_RGB, top, coverRef)) < 4.5) {
    top = Number((top + 0.02).toFixed(3));
  }
  while (bottom < 0.96 && contrastRatio(LIGHT_FG_RGB, compositeOver(DARK_SCRIM_RGB, bottom, coverRef)) < 7.0) {
    bottom = Number((bottom + 0.02).toFixed(3));
  }
  return { top, bottom };
}

/** Computes D-23 V2 gradient stops and V1 luminance-clamped scrim CSS variables from `art`. */
export function computeArtBackdrop(art: ArtInfoWithPalette | null | undefined): ArtBackdropStyle {
  if (!art) {
    return {
      hasArt: false,
      artId: null,
      washPrimaryCss: formatRgbGlowCss(DEFAULT_RGB, 0.28) ?? 'rgba(107, 197, 255, 0.28)',
      washSecondaryCss: formatRgbGlowCss(DEFAULT_SECONDARY_RGB, 0.2) ?? 'rgba(58, 132, 199, 0.2)',
      washTertiaryCss: null,
      scrimTopCss: 'rgba(12, 15, 20, 0.56)',
      scrimBottomCss: 'rgba(10, 13, 18, 0.84)',
      coverLuminance: 0.1,
    };
  }

  const primary = art.palette?.[0] ?? art.accent ?? DEFAULT_RGB;
  const secondary = art.palette?.[1] ?? art.secondary ?? deriveSecondaryStop(primary);
  const tertiary = art.palette?.[2] ?? null;
  const coverLuminance = clamp(
    typeof art.luminance === 'number' && Number.isFinite(art.luminance)
      ? art.luminance
      : relativeLuminance(primary),
    0,
    1,
  );
  const { top, bottom } = scrimAlphasForLuminance(coverLuminance);

  return {
    hasArt: true,
    artId: art.id,
    washPrimaryCss: formatRgbGlowCss(primary, 0.42) ?? 'rgba(107, 197, 255, 0.42)',
    washSecondaryCss: formatRgbGlowCss(secondary, 0.3) ?? 'rgba(58, 132, 199, 0.3)',
    washTertiaryCss: tertiary ? formatRgbGlowCss(tertiary, 0.26) : null,
    scrimTopCss: `rgba(12, 15, 20, ${top.toFixed(3)})`,
    scrimBottomCss: `rgba(10, 13, 18, ${bottom.toFixed(3)})`,
    coverLuminance,
  };
}

/**
 * Builds enabled D-22 "Music services" quick-launch items. Config `[music_services]` (Settings → Activities);
 * both on by default, and while settings haven't loaded yet.
 */
export function resolveMusicServices(
  settings: SettingsView | null | undefined,
  media: NowPlaying | null | undefined,
): MusicServiceItem[] {
  const spotifyEnabled = settings?.musicServices.spotify ?? true;
  const youtubeMusicEnabled = settings?.musicServices.youtubeMusic ?? true;

  const activeApp = isMediaActive(media) ? media.appId : '';
  const items: MusicServiceItem[] = [];
  if (spotifyEnabled) {
    items.push({
      id: 'spotify',
      label: 'Spotify',
      shortLabel: 'Spotify',
      active: /spotify/i.test(activeApp),
    });
  }
  if (youtubeMusicEnabled) {
    items.push({
      id: 'youtube_music',
      label: 'YouTube Music',
      shortLabel: 'YT Music',
      active: /youtube|ytm/i.test(activeApp),
    });
  }
  return items;
}
