// D-28 (owner, 2026-11-25): the card's volume control never jumps on a click, and it sits in the output strip next to
// the device chips, collapsing to a chip when they need the room. Pure maths, unit-tested in volume.test.ts.

/** One click on the volume rail moves this far towards the click (the arrow keys use the same step). */
export const VOLUME_CLICK_STEP = 2;
/** Pointer travel before a press on the rail counts as a drag instead of a click (px). */
export const VOLUME_DRAG_THRESHOLD_PX = 3;

const clampPercent = (p: number): number => Math.max(0, Math.min(100, Math.round(p)));

/**
 * Volume while dragging: relative to where the drag started, so grabbing the rail anywhere never jumps (B2).
 * The whole rail width is 0–100 %.
 */
export function volumeDragPercent(startPercent: number, dxPx: number, railWidthPx: number): number {
  if (!(railWidthPx > 0)) return clampPercent(startPercent);
  return clampPercent(startPercent + (dxPx / railWidthPx) * 100);
}

/** A click (no drag) steps towards the clicked point by at most `VOLUME_CLICK_STEP`, never further (B2). */
export function volumeClickPercent(currentPercent: number, clickedPercent: number): number {
  const diff = clickedPercent - currentPercent;
  if (Math.abs(diff) < 0.5) return clampPercent(currentPercent);
  return clampPercent(currentPercent + Math.sign(diff) * Math.min(VOLUME_CLICK_STEP, Math.abs(diff)));
}

/**
 * Whether the output strip has room for the inline volume slider beside `chipCount` device chips at `widthPx`;
 * otherwise the volume shows as a chip that opens the slider in place of the chips. Budgets measured on the running
 * notch (2026-11-25): chips 80–100 px with their gap, the inline slider (icon, 84 px rail, %) 148 px.
 */
export function volumeFitsInline(chipCount: number, widthPx: number): boolean {
  if (!(widthPx > 0)) return true;
  const CHIP_PX = 100;
  const SLIDER_PX = 150;
  return Math.max(0, chipCount) * CHIP_PX + SLIDER_PX <= widthPx;
}
