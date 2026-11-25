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
