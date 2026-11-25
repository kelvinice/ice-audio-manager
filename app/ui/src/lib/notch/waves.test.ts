import assert from 'node:assert/strict';
import test from 'node:test';
import { taperedWavePath, wavyEllipse } from './waves.ts';

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
