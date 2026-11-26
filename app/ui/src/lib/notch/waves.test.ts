import assert from 'node:assert/strict';
import test from 'node:test';
import { pillRippleRing, taperedWavePath, type RipplePill } from './waves.ts';

const points = (d: string): [number, number][] =>
  [...d.matchAll(/[ML] (-?[\d.]+) (-?[\d.]+)/g)].map((m) => [Number(m[1]), Number(m[2])]);

/** Distance from a point to a rounded rect's outline, outside positive (the signed distance of a box with radius). */
function outsideBy(x: number, y: number, box: { left: number; top: number; w: number; h: number; r: number }): number {
  const qx = Math.abs(x - (box.left + box.w / 2)) - (box.w / 2 - box.r);
  const qy = Math.abs(y - (box.top + box.h / 2)) - (box.h / 2 - box.r);
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - box.r;
}

// The wide compact music pill (292 × 48, r 19), floating 6 px under the top edge.
const musicPill: RipplePill = { left: 178, top: 6, w: 292, h: 48, r: 19, dock: 'none' };

test('a floating ring is the pill outline pushed out evenly, round the corners too (not an ellipse)', () => {
  const ring = pillRippleRing(musicPill, 20, { amp: 1.5, wavelength: 30, canvasH: 292 });
  assert.ok(ring.d.endsWith(' Z'));
  const gaps = points(ring.d).map(([x, y]) => outsideBy(x, y, musicPill));
  assert.ok(gaps.every((g) => g > 20 - 1.5 - 1e-6 && g < 20 + 1.5 + 1e-6), 'every point is 20 ± amp px off the pill');
  assert.ok(gaps.some((g) => g > 21) && gaps.some((g) => g < 19), 'and it is wavy');
  assert.deepEqual([ring.cx, ring.cy], [324, 30]);
});

test('the start scale puts a ring just off the pill, the same distance on every side', () => {
  const ring = pillRippleRing(musicPill, 20, { amp: 0, wavelength: 30, canvasH: 292, startOffset: 2 });
  const scaled = points(ring.d).map(([x, y]) => [ring.cx + (x - ring.cx) * ring.sx0, ring.cy + (y - ring.cy) * ring.sy0]);
  const xs = scaled.map(([x]) => x);
  const ys = scaled.map(([, y]) => y);
  assert.ok(Math.abs(Math.min(...xs) - (178 - 2)) < 0.05 && Math.abs(Math.max(...xs) - (470 + 2)) < 0.05);
  assert.ok(Math.abs(Math.min(...ys) - (6 - 2)) < 0.05 && Math.abs(Math.max(...ys) - (54 + 2)) < 0.05);
});

test('a docked ring is an open U hugging the pill, never past the edge it docks on', () => {
  const top: RipplePill = { left: 178, top: 0, w: 292, h: 48, r: 19, dock: 'top' };
  const ring = pillRippleRing(top, 16, { amp: 1.5, wavelength: 30, canvasH: 292 });
  assert.ok(!ring.d.endsWith('Z'));
  const pts = points(ring.d);
  assert.ok(pts.every(([, y]) => y >= -0.01), 'never above the screen edge');
  assert.deepEqual(pts[0], [178 - 16, 0], 'starts on the edge, wobble tapered away');
  assert.ok(Math.max(...pts.map(([, y]) => y)) <= 48 + 16 + 1.5 + 1e-6);
  assert.deepEqual([ring.cx, ring.cy], [324, 0]);

  const bottom = pillRippleRing({ ...top, top: 244, dock: 'bottom' }, 16, { amp: 1.5, wavelength: 30, canvasH: 292 });
  const bpts = points(bottom.d);
  assert.ok(bpts.every(([, y]) => y <= 292.01), 'bottom-docked: never below the taskbar edge');
  assert.ok(Math.min(...bpts.map(([, y]) => y)) >= 292 - 48 - 16 - 1.5 - 1e-6);
  assert.deepEqual([bottom.cx, bottom.cy], [324, 292]);
});

test('the played wave starts and stops on the centre line, whatever the phase (round caps, no clipping)', () => {
  for (const phase of [0, 3.3, 7.9, 19.5]) {
    const pts = points(taperedWavePath(2.5, 180, 6, { wavelength: 20, amp: 2.6, phase }));
    const [first, last] = [pts[0], pts[pts.length - 1]];
    assert.deepEqual(first, [2.5, 6], `starts on the centre line at phase ${phase}`);
    assert.deepEqual(last, [180, 6], `ends at the playhead on the centre line at phase ${phase}`);
    assert.ok(pts.every(([, y]) => Math.abs(y - 6) <= 2.6 + 1e-9), 'never taller than the amplitude');
    assert.ok(pts.some(([, y]) => Math.abs(y - 6) > 2), 'but really wavy in the middle');
  }
});

test('a paused wave is a straight line, and a tiny played part stays flat', () => {
  const paused = points(taperedWavePath(2.5, 180, 6, { wavelength: 20, amp: 2.6, phase: 4, ampScale: 0 }));
  assert.ok(paused.every(([, y]) => y === 6));
  const tiny = points(taperedWavePath(2.5, 6, 6, { wavelength: 20, amp: 2.6, phase: 4 }));
  assert.ok(tiny.every(([, y]) => Math.abs(y - 6) < 1), 'a few px of progress barely ripples');
});
