import type { HitRect, NotchSnapshot, SpringPreset } from '../ipc/types.ts';
import { DEFAULT_IDLE_MODE, type ActivityView, type IdleMode } from './scheduler.ts';

// Twin: `NOTCH_CANVAS_W`/`NOTCH_CANVAS_H` in crates/ice-core/src/placement.rs (the side anchors reserve this width, D-20). The canvas keeps ≥ 24 px around the expanded
// pill (+ bottom lyrics extension or Edge Nook side-by-side split + ears) so `--ice-drop-shadow` is never clipped.
export const CANVAS_W = 648;
export const CANVAS_H = 292;
export const TOP_PAD = 6;

/** Expanded card: media block + volume + devices beside a side rail (pin, mic, services, lyrics, settings). */
export const EXPANDED_W = 392;
export const EXPANDED_H_MEDIA = 164;
export const EXPANDED_H_EMPTY = 144;
export const EXPANDED_R = 24;
/** Attached bottom lyrics extension heights for compact music pill and expanded card (`docs/lyrics.md`). */
// Pill: two rows by default, the current line (16 px) and the next line (15 px + 1), plus 9 px padding (owner, 2026-11-24).
// With a Romaji/subtitle row (13 px + 1) under the current line.
export const PILL_LYRICS_H = 42;
export const PILL_LYRICS_SUB_H = 56;
export const CARD_LYRICS_H = 82;

export interface PillBox {
  w: number;
  h: number;
  r: number;
  vol: number;
}

export function effectivePreset(snap: NotchSnapshot | null): SpringPreset {
  if (!snap) return 'bouncy';
  return snap.systemReduceMotion ? 'off' : snap.preset;
}

export function targetGeometry(
  view: ActivityView,
  expandedNow: boolean,
  micMutedNow: boolean,
  idleModeNow: IdleMode = DEFAULT_IDLE_MODE,
  hasMedia = false,
  fallbackVolume = 40,
  lyricsExtraH = 0,
): PillBox {
  const vol = view.kind === 'volume' ? view.percent : fallbackVolume;
  if (expandedNow) {
    return {
      w: EXPANDED_W,
      h: (hasMedia ? EXPANDED_H_MEDIA : EXPANDED_H_EMPTY) + (hasMedia ? lyricsExtraH : 0),
      r: EXPANDED_R,
      vol,
    };
  }
  switch (view.kind) {
    case 'volume':
      return { w: 236, h: 38, r: 19, vol: view.percent };
    case 'switch':
      return { w: 296, h: 44, r: 22, vol };
    case 'plug':
      return { w: 268, h: 44, r: 22, vol };
    case 'mic':
      return { w: 204, h: 38, r: 19, vol };
    case 'track':
      return { w: 292, h: 46 + lyricsExtraH, r: lyricsExtraH > 0 ? 22 : 23, vol };
    case 'idle':
    default:
      return idleGeometry(idleModeNow, hasMedia, micMutedNow, vol);
  }
}

export function idleGeometry(
  idleModeNow: IdleMode,
  hasMedia: boolean,
  micMutedNow: boolean,
  vol: number,
  extraH = 0,
  nookRadius = false,
): PillBox {
  if (idleModeNow === 'hidden') {
    return { w: 96, h: Math.max(8, 10 - (nookRadius ? 2 : 0)), r: nookRadius ? 4 : 5, vol };
  }
  if (idleModeNow === 'dot') {
    const h = 18 + extraH;
    if (hasMedia) {
      return { w: micMutedNow ? 58 : 44, h, r: 9, vol };
    }
    return { w: micMutedNow ? 42 : 36, h, r: 9, vol };
  }
  const h = 28 + extraH;
  const r = nookRadius ? 13 : 14;
  if (hasMedia) {
    return { w: micMutedNow ? 216 : 196, h, r, vol };
  }
  return { w: micMutedNow ? 132 : 116, h, r, vol };
}

export function computeHitRect(target: { w: number; h: number }): HitRect {
  return {
    x: (CANVAS_W - target.w) / 2,
    y: TOP_PAD,
    width: target.w,
    height: target.h,
  };
}

export function computeClipPath(pillSize: { w: number; h: number; r: number }): {
  insetLeft: number;
  insetBottom: number;
  clipPath: string;
} {
  const insetLeft = Math.max(0, (CANVAS_W - pillSize.w) / 2);
  const insetBottom = Math.max(0, CANVAS_H - TOP_PAD - pillSize.h);
  const clipPath = `inset(${TOP_PAD}px ${insetLeft.toFixed(2)}px ${insetBottom.toFixed(2)}px ${insetLeft.toFixed(2)}px round ${pillSize.r.toFixed(2)}px)`;
  return { insetLeft, insetBottom, clipPath };
}

export function resolveRimStroke(
  view: ActivityView,
  isExpanded: boolean,
  effectiveMicMuted: boolean,
): string {
  if (isExpanded) return 'var(--ice-rim)';
  switch (view.kind) {
    case 'mic':
      return view.muted ? 'var(--ice-danger)' : 'var(--ice-ok)';
    case 'volume':
      return view.muted ? 'var(--ice-danger)' : 'var(--ice-rim)';
    case 'plug':
      return view.direction === 'arrived' ? 'var(--ice-ok)' : 'var(--ice-warn)';
    case 'switch':
      return 'var(--ice-rim)';
    case 'idle':
    default:
      return effectiveMicMuted ? 'var(--ice-danger)' : 'var(--ice-rim)';
  }
}

/** Preserves `--bench` driver overrides when `NotchSnapshot` changes outside `EngineEvent`. */
export function deriveBenchOverride(
  prevActivity: NotchSnapshot['activity'] | undefined,
  prevSeq: number,
  next: NotchSnapshot,
  currentOverride: ActivityView | null,
  engineIdle: boolean,
  nowMs: number,
): ActivityView | null {
  if (next.hiddenForBench) return null;
  const changed = next.eventSeq > prevSeq || (next.activity !== prevActivity && next.activity === 'idle');
  if (!changed) return currentOverride;
  if (next.activity === 'idle') return null;
  if (!engineIdle) return currentOverride;

  if (next.activity === 'volume') {
    return {
      kind: 'volume',
      channel: 'playback',
      percent: next.volumePercent,
      muted: next.muted,
      deviceName: next.activeDevice,
      formFactor: null,
      entrySeq: next.eventSeq,
      boundBump: 0,
      bumpSeq: 0,
      expiresAtMs: nowMs,
    };
  }
  if (next.activity === 'mic_mute') {
    return {
      kind: 'mic',
      muted: next.micMuted,
      deviceName: 'Microphone',
      entranceSeq: next.eventSeq,
      expiresAtMs: nowMs,
    };
  }
  return currentOverride;
}
