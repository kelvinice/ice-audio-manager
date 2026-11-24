import type { HitRect, NotchSnapshot, SpringPreset } from '../ipc/types.ts';
import { DEFAULT_IDLE_MODE, type ActivityView, type IdleMode } from './scheduler.ts';

// Twin: `NOTCH_CANVAS_W`/`NOTCH_CANVAS_H` in app/src-tauri/src/main.rs. The canvas keeps ≥ 24 px around the expanded
// pill so `--ice-drop-shadow` is never clipped by the window edge.
export const CANVAS_W = 440;
export const CANVAS_H = 210;
export const TOP_PAD = 6;

/** Expanded card: media block + volume + devices beside a side rail (pin, mic, services, settings). */
export const EXPANDED_W = 392;
export const EXPANDED_H_MEDIA = 164;
export const EXPANDED_H_EMPTY = 144;
export const EXPANDED_R = 24;

export interface PillBox {
  w: number;
  h: number;
  r: number;
  vol: number;
}

export function effectivePreset(snap: NotchSnapshot | null): SpringPreset {
  if (!snap) return 'smooth';
  return snap.systemReduceMotion ? 'off' : snap.preset;
}

export function targetGeometry(
  view: ActivityView,
  expandedNow: boolean,
  micMutedNow: boolean,
  idleModeNow: IdleMode = DEFAULT_IDLE_MODE,
  hasMedia = false,
  fallbackVolume = 40,
): PillBox {
  const vol = view.kind === 'volume' ? view.percent : fallbackVolume;
  if (expandedNow) {
    return { w: EXPANDED_W, h: hasMedia ? EXPANDED_H_MEDIA : EXPANDED_H_EMPTY, r: EXPANDED_R, vol };
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
      return { w: 296, h: 48, r: 24, vol };
    case 'idle':
    default:
      return idleGeometry(idleModeNow, hasMedia, micMutedNow, vol);
  }
}

function idleGeometry(
  idleModeNow: IdleMode,
  hasMedia: boolean,
  micMutedNow: boolean,
  vol: number,
): PillBox {
  if (idleModeNow === 'hidden') {
    return { w: 96, h: 10, r: 5, vol };
  }
  if (idleModeNow === 'dot') {
    if (hasMedia) {
      return { w: micMutedNow ? 58 : 44, h: 18, r: 9, vol };
    }
    return { w: micMutedNow ? 42 : 36, h: 18, r: 9, vol };
  }
  if (hasMedia) {
    return { w: micMutedNow ? 216 : 196, h: 28, r: 14, vol };
  }
  return { w: micMutedNow ? 132 : 116, h: 28, r: 14, vol };
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
