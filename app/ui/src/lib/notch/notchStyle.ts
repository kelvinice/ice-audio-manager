import type { HitRect, NotchStyle } from '../ipc/types.ts';
import {
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
  topPad: number;
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
  computeSurfaceShape(pillSize: PillBox): SurfaceShape;
  computeHitRect(target: { w: number; h: number }): HitRect;
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

/** Closed clockwise SVG path anchored flush at `y = 0` with concave top ears and rounded bottom corners. */
export function buildNookFillPath(left: number, right: number, h: number, r: number, ear: number): string {
  const open = buildNookRimPath(left, right, h, r, ear);
  return `${open} Z`;
}

/** Open clockwise SVG stroke path along ears, side walls, and bottom corners (omits top bezel line at `y = 0`). */
export function buildNookRimPath(left: number, right: number, h: number, r: number, ear: number): string {
  const safeH = Math.max(4, h);
  const f = (n: number) => n.toFixed(2);
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
  computeSurfaceShape(pillSize: PillBox): SurfaceShape {
    const { insetLeft, clipPath } = computeClipPath(pillSize);
    return {
      insetLeft,
      topPad: TOP_PAD,
      earRadius: 0,
      clipPath,
      shadowRadiusCss: `${Math.max(0, pillSize.r).toFixed(2)}px`,
      rimKind: 'rect',
      rimPathD: '',
    };
  },
  computeHitRect: computeFloatingHitRect,
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
  computeSurfaceShape(pillSize: PillBox): SurfaceShape {
    const rawEar = nookEarForHeight(pillSize.h);
    const { left, right, r, e } = clampNookArcs(pillSize.w, pillSize.h, pillSize.r, rawEar);
    const fillD = buildNookFillPath(left, right, pillSize.h, r, e);
    const rimD = buildNookRimPath(left, right, pillSize.h, r, e);
    return {
      insetLeft: Math.max(0, left),
      topPad: 0,
      earRadius: e,
      clipPath: `path('${fillD}')`,
      shadowRadiusCss: `0 0 ${r.toFixed(2)}px ${r.toFixed(2)}px`,
      rimKind: 'path',
      rimPathD: rimD,
    };
  },
  computeHitRect(target: { w: number; h: number }): HitRect {
    const ear = nookEarForHeight(target.h);
    const width = target.w + ear * 2;
    return {
      x: Math.max(0, (CANVAS_W - width) / 2),
      y: 0,
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
