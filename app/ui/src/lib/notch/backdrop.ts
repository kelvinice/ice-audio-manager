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
const DEFAULT_SECONDARY_RGB: Rgb = { r: 88, g: 118, b: 226 };
const DARK_SCRIM_RGB: Rgb = { r: 12, g: 15, b: 20 };
const LIGHT_SCRIM_RGB: Rgb = { r: 246, g: 247, b: 249 };
const LIGHT_FG_RGB: Rgb = { r: 237, g: 241, b: 247 };
const MUTED_FG_RGB: Rgb = { r: 152, g: 164, b: 184 };
const DARK_INK_RGB: Rgb = { r: 17, g: 22, b: 31 };
const DARK_MUTED_RGB: Rgb = { r: 75, g: 86, b: 104 };

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

/** Adapts an album-art accent for readability on light themes (`ice_light`) while keeping dark themes untouched. */
export function adaptAccentForSurface(rgb: Rgb, isLightTheme: boolean): Rgb {
  if (!isLightTheme) return rgb;
  const [h, s, l] = rgbToHsl(rgb);
  let targetL = clamp(Math.min(l, 0.42), 0.22, 0.42);
  const targetS = s < 0.08 ? 0.08 : clamp(Math.max(s, 0.58), 0.58, 0.88);
  let candidate = hslToRgb(h, targetS, targetL);
  while (targetL > 0.16 && contrastRatio(candidate, LIGHT_SCRIM_RGB) < 4.0) {
    targetL = Math.max(0.16, targetL - 0.03);
    candidate = hslToRgb(h, targetS, targetL);
  }
  return candidate;
}

/** Derives a distinct secondary gradient stop (+52° hue shift) when `ArtInfo` provides a single accent. */
export function deriveSecondaryStop(primary: Rgb): Rgb {
  const [h, s, l] = rgbToHsl(primary);
  if (s < 0.08) return DEFAULT_SECONDARY_RGB;
  return hslToRgb(h + 52, clamp(Math.max(s, 0.48), 0.48, 0.9), clamp(l * 0.85, 0.26, 0.58));
}

function tuneWashColor(rgb: Rgb, isLightTheme: boolean): Rgb {
  const [h, s, l] = rgbToHsl(rgb);
  if (isLightTheme) {
    return hslToRgb(h, s < 0.08 ? 0.16 : clamp(Math.max(s, 0.52), 0.52, 0.85), clamp(l, 0.56, 0.72));
  }
  // Keep dark-theme wash blobs vivid but below L=0.44 so moving blobs stay clearly visible while preserving >= 4.5:1 muted text contrast.
  return hslToRgb(h, s < 0.08 ? 0.18 : clamp(Math.max(s, 0.58), 0.58, 0.92), clamp(l, 0.32, 0.44));
}

function scrimAlphasForLuminance(lum: number, isLightTheme: boolean): { top: number; bottom: number } {
  const l = clamp(lum, 0, 1);
  if (isLightTheme) {
    // Dark covers raise the frosted light scrim so dark ink (`#11161f` / `#4b5668`) holds WCAG AA (>= 4.5:1) over V1 blur.
    let top = clamp(0.68 + (1 - l) * 0.22, 0.68, 0.92);
    let bottom = clamp(0.84 + (1 - l) * 0.12, 0.84, 0.96);
    const coverRef: Rgb = { r: Math.round(l * 255), g: Math.round(l * 255), b: Math.round(l * 255) };
    while (top < 0.94 && contrastRatio(DARK_MUTED_RGB, compositeOver(LIGHT_SCRIM_RGB, top, coverRef)) < 4.5) {
      top = Number((top + 0.02).toFixed(3));
    }
    while (bottom < 0.96 && contrastRatio(DARK_INK_RGB, compositeOver(LIGHT_SCRIM_RGB, bottom, coverRef)) < 7.0) {
      bottom = Number((bottom + 0.02).toFixed(3));
    }
    return { top, bottom };
  }

  // Bright/white covers raise the dark scrim so `--ice-fg` and `--ice-fg-muted` hold WCAG AA (>= 4.5:1) over V1 blur.
  let top = clamp(0.52 + l * 0.30, 0.52, 0.82);
  let bottom = clamp(0.80 + l * 0.14, 0.80, 0.94);

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

/** Computes D-23/D-25 V2 gradient stops and V1 luminance-clamped scrim CSS variables from `art`. */
export function computeArtBackdrop(
  art: ArtInfoWithPalette | null | undefined,
  isLightTheme = false,
): ArtBackdropStyle {
  if (!art) {
    return {
      hasArt: false,
      artId: null,
      washPrimaryCss: formatRgbGlowCss(DEFAULT_RGB, isLightTheme ? 0.24 : 0.28) ?? 'rgba(107, 197, 255, 0.28)',
      washSecondaryCss: formatRgbGlowCss(DEFAULT_SECONDARY_RGB, isLightTheme ? 0.18 : 0.22) ?? 'rgba(88, 118, 226, 0.22)',
      washTertiaryCss: null,
      scrimTopCss: isLightTheme ? 'rgba(246, 247, 249, 0.74)' : 'rgba(12, 15, 20, 0.56)',
      scrimBottomCss: isLightTheme ? 'rgba(240, 243, 248, 0.90)' : 'rgba(10, 13, 18, 0.84)',
      coverLuminance: 0.1,
    };
  }

  const rawPrimary = art.palette?.[0] ?? art.accent ?? DEFAULT_RGB;
  const rawSecondary = art.palette?.[1] ?? art.secondary ?? deriveSecondaryStop(rawPrimary);
  const rawTertiary = art.palette?.[2] ?? null;
  const primary = tuneWashColor(rawPrimary, isLightTheme);
  const secondary = tuneWashColor(rawSecondary, isLightTheme);
  const tertiary = rawTertiary ? tuneWashColor(rawTertiary, isLightTheme) : null;
  const coverLuminance = clamp(
    typeof art.luminance === 'number' && Number.isFinite(art.luminance)
      ? art.luminance
      : relativeLuminance(rawPrimary),
    0,
    1,
  );
  const { top, bottom } = scrimAlphasForLuminance(coverLuminance, isLightTheme);
  const scrimRgb = isLightTheme ? LIGHT_SCRIM_RGB : DARK_SCRIM_RGB;
  const scrimBotRgb = isLightTheme ? { r: 240, g: 243, b: 248 } : { r: 10, g: 13, b: 18 };
  const primaryAlpha = isLightTheme ? 0.34 : 0.44;
  const secondaryAlpha = isLightTheme ? 0.28 : 0.36;
  const tertiaryAlpha = isLightTheme ? 0.24 : 0.30;

  return {
    hasArt: true,
    artId: art.id,
    washPrimaryCss: formatRgbGlowCss(primary, primaryAlpha) ?? 'rgba(107, 197, 255, 0.44)',
    washSecondaryCss: formatRgbGlowCss(secondary, secondaryAlpha) ?? 'rgba(88, 118, 226, 0.36)',
    washTertiaryCss: tertiary ? formatRgbGlowCss(tertiary, tertiaryAlpha) : null,
    scrimTopCss: `rgba(${scrimRgb.r}, ${scrimRgb.g}, ${scrimRgb.b}, ${top.toFixed(3)})`,
    scrimBottomCss: `rgba(${scrimBotRgb.r}, ${scrimBotRgb.g}, ${scrimBotRgb.b}, ${bottom.toFixed(3)})`,
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
