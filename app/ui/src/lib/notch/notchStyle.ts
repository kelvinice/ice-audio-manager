import type { HitRect, NotchStyle } from '../ipc/types.ts';
import {
  BOTTOM_PAD,
  CANVAS_H,
  CANVAS_W,
  computeClipPath,
  computeHitRect as computeFloatingHitRect,
  EXPANDED_H_EMPTY,
  EXPANDED_H_MEDIA,
  EXPANDED_R,
  EXPANDED_W,
  idleGeometry,
  targetGeometry as floatingTargetGeometry,
  TOP_PAD,
  type PillBox,
} from './geometry.ts';
import type { ActivityView, IdleMode } from './scheduler.ts';

// Twin: `frame_metrics` in `crates/ice-win/src/native_dot.rs`.
export const NOOK_EXPANDED_W_SINGLE = 408;
export const NOOK_EXPANDED_W_SPLIT = 572;
export const NOOK_EXPANDED_H_MEDIA = 206;
export const NOOK_EXPANDED_H_SPLIT = 206;
export const NOOK_EXPANDED_H_EMPTY = 166;
export const NOOK_EXPANDED_R = 22;

export interface GeometryContext {
  view: ActivityView;
  expanded: boolean;
  micMuted: boolean;
  idleMode: IdleMode;
  hasMedia: boolean;
  fallbackVolume: number;
  pillLyricsExtraH: number;
  cardLyricsExtraH: number;
  showCardLyrics: boolean;
}

export interface SurfaceShape {
  insetLeft: number;
  insetTop: number;
  topPad: number;
  bottomPad: number;
  isBottom: boolean;
  earRadius: number;
  clipPath: string;
  shadowRadiusCss: string;
  rimKind: 'rect' | 'path';
  rimPathD: string;
}

/**
 * Strategy pattern for notch presentation styles (`Floating` capsule vs flush `Edge Nook`).
 * Implementations are stateless frozen singletons so `SpringGroup` ticks allocate minimally.
 */
export interface NotchStyleStrategy {
  readonly id: NotchStyle;
  readonly topPad: number;
  readonly usesSideBySideLyrics: boolean;
  earRadius(box: { h: number }): number;
  targetGeometry(ctx: GeometryContext): PillBox;
  /** `lift` 0–1: held / carried (Edge Nook becomes a full capsule off the edge; Floating already is one). */
  computeSurfaceShape(pillSize: PillBox, bottom?: boolean, lift?: number): SurfaceShape;
  computeHitRect(target: { w: number; h: number }, bottom?: boolean): HitRect;
}

function clampNookArcs(bodyW: number, h: number, rawR: number, rawEar: number): { left: number; right: number; r: number; e: number } {
  const safeW = Math.max(12, bodyW);
  const safeH = Math.max(4, h);
  const left = (CANVAS_W - safeW) / 2;
  const right = left + safeW;
  const maxSideEar = Math.max(0, (CANVAS_W - safeW) * 0.48);
  const e = Math.max(0, Math.min(rawEar, safeH * 0.46, maxSideEar));
  const r = Math.max(0, Math.min(rawR, safeW * 0.5, Math.max(0, safeH - e)));
  return { left, right, r, e };
}

/** Closed clockwise SVG path anchored flush at `y = 0` (top) or `y = CANVAS_H` (bottom). */
export function buildNookFillPath(left: number, right: number, h: number, r: number, ear: number, bottom = false): string {
  const open = buildNookRimPath(left, right, h, r, ear, bottom);
  return `${open} Z`;
}

/** Open SVG stroke path along ears, side walls, and free rounded corners (omits flushed bezel/taskbar line). */
export function buildNookRimPath(left: number, right: number, h: number, r: number, ear: number, bottom = false): string {
  const safeH = Math.max(4, h);
  const f = (n: number) => n.toFixed(2);
  if (bottom) {
    const baseY = CANVAS_H;
    const topY = Math.max(0, CANVAS_H - safeH);
    if (ear < 0.5) {
      return (
        `M ${f(left)} ${f(baseY)}` +
        ` L ${f(left)} ${f(topY + r)}` +
        ` A ${f(r)} ${f(r)} 0 0 1 ${f(left + r)} ${f(topY)}` +
        ` L ${f(right - r)} ${f(topY)}` +
        ` A ${f(r)} ${f(r)} 0 0 1 ${f(right)} ${f(topY + r)}` +
        ` L ${f(right)} ${f(baseY)}`
      );
    }
    return (
      `M ${f(left - ear)} ${f(baseY)}` +
      ` A ${f(ear)} ${f(ear)} 0 0 0 ${f(left)} ${f(baseY - ear)}` +
      ` L ${f(left)} ${f(topY + r)}` +
      ` A ${f(r)} ${f(r)} 0 0 1 ${f(left + r)} ${f(topY)}` +
      ` L ${f(right - r)} ${f(topY)}` +
      ` A ${f(r)} ${f(r)} 0 0 1 ${f(right)} ${f(topY + r)}` +
      ` L ${f(right)} ${f(baseY - ear)}` +
      ` A ${f(ear)} ${f(ear)} 0 0 0 ${f(right + ear)} ${f(baseY)}`
    );
  }
  if (ear < 0.5) {
    return (
      `M ${f(left)} 0` +
      ` L ${f(left)} ${f(safeH - r)}` +
      ` A ${f(r)} ${f(r)} 0 0 0 ${f(left + r)} ${f(safeH)}` +
      ` L ${f(right - r)} ${f(safeH)}` +
      ` A ${f(r)} ${f(r)} 0 0 0 ${f(right)} ${f(safeH - r)}` +
      ` L ${f(right)} 0`
    );
  }
  return (
    `M ${f(left - ear)} 0` +
    ` A ${f(ear)} ${f(ear)} 0 0 1 ${f(left)} ${f(ear)}` +
    ` L ${f(left)} ${f(safeH - r)}` +
    ` A ${f(r)} ${f(r)} 0 0 0 ${f(left + r)} ${f(safeH)}` +
    ` L ${f(right - r)} ${f(safeH)}` +
    ` A ${f(r)} ${f(r)} 0 0 0 ${f(right)} ${f(safeH - r)}` +
    ` L ${f(right)} ${f(ear)}` +
    ` A ${f(ear)} ${f(ear)} 0 0 1 ${f(right + ear)} 0`
  );
}

/** How far a held / carried notch moves off its edge, so its rounded flush-side corners show (logical px). */
export const LIFT_GAP_PX = 6;

const clamp01 = (n: number): number => Math.max(0, Math.min(1, n));

/**
 * Edge Nook outline at a lift of 0–1 (owner, 2026-11-26: a held notch is a full capsule, not half-rounded). The first
 * half of the lift folds the ears away, the second rounds the flush-edge corners; the pill also moves `LIFT_GAP_PX` off
 * the edge. At `lift = 0` the paths equal `buildNookFillPath` / `buildNookRimPath`. Bottom anchors are the mirror image.
 */
export function buildLiftedNookPaths(
  left: number,
  right: number,
  h: number,
  r: number,
  ear: number,
  lift: number,
  bottom = false,
): { fill: string; rim: string; ear: number; edgeR: number; gap: number } {
  const safeH = Math.max(4, h);
  const k = clamp01(lift);
  const earNow = ear * clamp01(1 - 2 * k);
  const e = earNow >= 0.5 ? earNow : 0;
  const edgeRaw = Math.min(r * clamp01(2 * k - 1), Math.max(0, safeH - r));
  const t = e === 0 && edgeRaw >= 0.5 ? edgeRaw : 0;
  const gap = LIFT_GAP_PX * Math.max(0, lift);
  const y0 = gap;
  const y1 = gap + safeH;
  const f = (n: number) => n.toFixed(2);
  const P = (x: number, y: number) => `${f(x)} ${f(bottom ? CANVAS_H - y : y)}`;
  // Mirroring top↔bottom reverses every arc's direction.
  const A = (rad: number, sweep: 0 | 1, x: number, y: number) =>
    ` A ${f(rad)} ${f(rad)} 0 0 ${bottom ? 1 - sweep : sweep} ${P(x, y)}`;

  let d = e > 0 ? `M ${P(left - e, y0)}${A(e, 1, left, y0 + e)}` : `M ${P(left, y0 + t)}`;
  d += ` L ${P(left, y1 - r)}${A(r, 0, left + r, y1)}`;
  d += ` L ${P(right - r, y1)}${A(r, 0, right, y1 - r)}`;
  d += ` L ${P(right, y0 + (e || t))}`;
  if (e > 0) d += A(e, 1, right + e, y0);
  else if (t > 0) d += A(t, 0, right - t, y0);
  // Once the edge corners are round, the edge side is part of the outline too, so the rim closes over it.
  const rim = t > 0 ? `${d} L ${P(left + t, y0)}${A(t, 0, left, y0 + t)}` : d;
  return { fill: `${rim} Z`, rim, ear: e, edgeR: t, gap };
}

export const floatingStyleStrategy: NotchStyleStrategy = Object.freeze({
  id: 'floating',
  topPad: TOP_PAD,
  usesSideBySideLyrics: false,
  earRadius: () => 0,
  targetGeometry(ctx: GeometryContext): PillBox {
    const lyricsExtraH = ctx.expanded ? ctx.cardLyricsExtraH : ctx.pillLyricsExtraH;
    const base = floatingTargetGeometry(
      ctx.view,
      ctx.expanded,
      ctx.micMuted,
      ctx.idleMode,
      ctx.hasMedia,
      ctx.fallbackVolume,
      lyricsExtraH,
    );
    if (!ctx.expanded) return base;
    return {
      w: EXPANDED_W,
      h: (ctx.hasMedia ? EXPANDED_H_MEDIA : EXPANDED_H_EMPTY) + (ctx.hasMedia ? ctx.cardLyricsExtraH : 0),
      r: EXPANDED_R,
      vol: base.vol,
    };
  },
  computeSurfaceShape(pillSize: PillBox, bottom = false): SurfaceShape {
    const { insetLeft, insetTop, clipPath } = computeClipPath(pillSize, bottom);
    return {
      insetLeft,
      insetTop,
      topPad: TOP_PAD,
      bottomPad: BOTTOM_PAD,
      isBottom: bottom,
      earRadius: 0,
      clipPath,
      shadowRadiusCss: `${Math.max(0, pillSize.r).toFixed(2)}px`,
      rimKind: 'rect',
      rimPathD: '',
    };
  },
  computeHitRect: (target: { w: number; h: number }, bottom = false): HitRect =>
    computeFloatingHitRect(target, bottom),
});

function nookEarForHeight(h: number): number {
  if (h <= 10) return 0;
  if (h <= 22) return 6;
  if (h <= 64) return 8;
  return 12;
}

export const nookStyleStrategy: NotchStyleStrategy = Object.freeze({
  id: 'nook',
  topPad: 0,
  usesSideBySideLyrics: true,
  earRadius: (box: { h: number }) => nookEarForHeight(box.h),
  targetGeometry(ctx: GeometryContext): PillBox {
    const vol = ctx.view.kind === 'volume' ? ctx.view.percent : ctx.fallbackVolume;
    if (ctx.expanded) {
      if (ctx.hasMedia && ctx.showCardLyrics) {
        return { w: NOOK_EXPANDED_W_SPLIT, h: NOOK_EXPANDED_H_SPLIT, r: NOOK_EXPANDED_R, vol };
      }
      return {
        w: NOOK_EXPANDED_W_SINGLE,
        h: ctx.hasMedia ? NOOK_EXPANDED_H_MEDIA : NOOK_EXPANDED_H_EMPTY,
        r: NOOK_EXPANDED_R,
        vol,
      };
    }
    switch (ctx.view.kind) {
      case 'volume':
        return { w: 236, h: 40, r: 16, vol: ctx.view.percent };
      case 'switch':
        return { w: 296, h: 46, r: 18, vol };
      case 'plug':
        return { w: 268, h: 46, r: 18, vol };
      case 'mic':
        return { w: 204, h: 40, r: 16, vol };
      case 'track':
        return { w: 292, h: 48 + ctx.pillLyricsExtraH, r: ctx.pillLyricsExtraH > 0 ? 18 : 19, vol };
      case 'idle':
      default:
        return idleGeometry(ctx.idleMode, ctx.hasMedia, ctx.micMuted, vol, 2, true);
    }
  },
  computeSurfaceShape(pillSize: PillBox, bottom = false, lift = 0): SurfaceShape {
    const rawEar = nookEarForHeight(pillSize.h);
    const { left, right, r, e } = clampNookArcs(pillSize.w, pillSize.h, pillSize.r, rawEar);
    const p = buildLiftedNookPaths(left, right, pillSize.h, r, e, lift, bottom);
    const free = `${r.toFixed(2)}px`;
    const edge = `${p.edgeR.toFixed(2)}px`;
    return {
      insetLeft: Math.max(0, left),
      insetTop: bottom ? Math.max(0, CANVAS_H - pillSize.h - p.gap) : p.gap,
      topPad: bottom ? 0 : p.gap,
      bottomPad: bottom ? p.gap : 0,
      isBottom: bottom,
      earRadius: p.ear,
      clipPath: `path('${p.fill}')`,
      shadowRadiusCss: bottom ? `${free} ${free} ${edge} ${edge}` : `${edge} ${edge} ${free} ${free}`,
      rimKind: 'path',
      rimPathD: p.rim,
    };
  },
  computeHitRect(target: { w: number; h: number }, bottom = false): HitRect {
    const ear = nookEarForHeight(target.h);
    const width = target.w + ear * 2;
    return {
      x: Math.max(0, (CANVAS_W - width) / 2),
      y: bottom ? Math.max(0, CANVAS_H - target.h) : 0,
      width,
      height: target.h,
    };
  },
});

/** D-27 hibernate reports this: the plain notch takes no input, so the shell keeps the window click-through. */
export const NO_HIT_RECT: HitRect = Object.freeze({ x: 0, y: 0, width: 0, height: 0 });

/**
 * D-27 hibernate: the plain notch is always the idle shape, the same size whatever plays, whether the mic is muted or a
 * pop-up would show (twin: `rasterize_plain_frame` in `crates/ice-win/src/native_dot.rs`, which uses the idle metrics).
 */
export function plainGeometryContext(ctx: GeometryContext): GeometryContext {
  return {
    ...ctx,
    view: { kind: 'idle' },
    expanded: false,
    micMuted: false,
    hasMedia: false,
    pillLyricsExtraH: 0,
    cardLyricsExtraH: 0,
    showCardLyrics: false,
  };
}

const STRATEGIES: Record<NotchStyle, NotchStyleStrategy> = {
  floating: floatingStyleStrategy,
  nook: nookStyleStrategy,
};

export function getNotchStyleStrategy(style: NotchStyle | null | undefined): NotchStyleStrategy {
  return style === 'floating' ? STRATEGIES.floating : STRATEGIES.nook;
}
