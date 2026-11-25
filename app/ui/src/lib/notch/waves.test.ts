import assert from 'node:assert/strict';
import test from 'node:test';
import { sineStripPath, wavyEllipse } from './waves.ts';

const points = (d: string): [number, number][] =>
  [...d.matchAll(/[ML] (-?[\d.]+) (-?[\d.]+)/g)].map((m) => [Number(m[1]), Number(m[2])]);

test('a full wavy ring is closed and wobbles around its radius', () => {
  const ring = wavyEllipse(100, 50, 40, 20, { waves: 8, amp: 3 });
  assert.ok(ring.d.endsWith(' Z'));
  const dist = points(ring.d).map(([x, y]) => Math.hypot((x - 100) / 40, (y - 50) / 20));
  assert.ok(Math.min(...dist) < 0.95 && Math.max(...dist) > 1.05, 'the radius wobbles both ways');
  assert.ok(Math.min(...dist) > 0.8 && Math.max(...dist) < 1.2, 'but only by the amplitude');
});

test('a top-docked half ring stays below its edge and is open', () => {
  const ring = wavyEllipse(324, 0, 80, 40, { waves: 5, amp: 2, from: 0, to: Math.PI });
  assert.ok(!ring.d.endsWith('Z'));
  assert.ok(points(ring.d).every(([, y]) => y >= -0.01), 'never above the screen edge');
  const bottom = wavyEllipse(324, 292, 80, 40, { waves: 5, amp: 2, from: Math.PI, to: Math.PI * 2 });
  assert.ok(points(bottom.d).every(([, y]) => y <= 292.01), 'bottom-docked: never below the taskbar edge');
});

test('the progress sine covers the width plus one wavelength, so sliding it loops seamlessly', () => {
  const d = sineStripPath(300, 20, 2.5, 6);
  const xs = [...d.matchAll(/ T ([\d.]+) /g)].map((m) => Number(m[1]));
  assert.ok(Math.max(...xs) >= 320);
  assert.ok(d.startsWith('M 0 6.00 Q 5.00 1.00 10.00 6.00'), 'first half wave peaks 2.5 px above the middle');
});
