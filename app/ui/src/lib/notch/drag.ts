/**
 * Pure D-20 drag-and-snap gesture guards for the WebView notch.
 * Keeps pointer-threshold and interactive-control checks unit-testable outside the DOM runtime.
 */

/** Logical pixel movement before a primary press promotes from click to window drag (~Windows SM_CXDRAG). */
export const DRAG_THRESHOLD_PX = 5;

/** Interactive controls inside the notch that own their pointer gestures and must never initiate a window drag. */
export const INTERACTIVE_DRAG_BLOCK_SELECTOR =
  'button, a, input, select, textarea, [role="slider"], [role="button"], [data-no-drag]';

interface ElementLike {
  closest?: (selector: string) => unknown;
}

/** Returns `true` when `target` is inside an interactive control that should handle clicks/drags itself. */
export function isDragBlockedTarget(target: EventTarget | null): boolean {
  if (!target || typeof target !== 'object') return false;
  const el = target as ElementLike;
  if (typeof el.closest !== 'function') return false;
  return Boolean(el.closest(INTERACTIVE_DRAG_BLOCK_SELECTOR));
}

/** Returns `true` once the pointer moves further than `threshold` px along either axis from press origin. */
export function shouldStartDrag(
  startX: number,
  startY: number,
  curX: number,
  curY: number,
  threshold = DRAG_THRESHOLD_PX,
): boolean {
  return Math.abs(curX - startX) > threshold || Math.abs(curY - startY) > threshold;
}

/**
 * Hover intent (Edge Nook plan item, 2026-11-26): pointer speed in px/ms above which the pointer is passing over the
 * notch (on its way to a browser tab or title bar) rather than settling on it, so the hover-open dwell starts over.
 */
export const HOVER_PASS_SPEED_PX_PER_MS = 0.5;

/** Returns `true` when a pointer step of `(dx, dy)` px in `dtMs` is faster than `HOVER_PASS_SPEED_PX_PER_MS`. */
export function isPassingThrough(dx: number, dy: number, dtMs: number): boolean {
  if (!(dtMs > 0)) return false;
  return Math.hypot(dx, dy) / dtMs > HOVER_PASS_SPEED_PX_PER_MS;
}
