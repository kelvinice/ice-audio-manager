import assert from 'node:assert/strict';
import test from 'node:test';
import { FIT_EDGE_PX, FIT_MIN_SCALE, fitLineLayout } from './fitLine.ts';

test('a line that fits keeps room for the active word lift', () => {
  assert.deepEqual(fitLineLayout(200, 180, 180), { scale: 1, x: 0, fadeLeft: false, fadeRight: false });
  // Exactly filling the row used to lose its last pixels to the 1.07 lift: now it shrinks a little.
  const tight = fitLineLayout(200, 200, 200);
  assert.ok(tight.scale < 1 && tight.scale * 200 <= 200 - FIT_EDGE_PX + 1e-9, `${JSON.stringify(tight)}`);
});

test('a slightly long line shrinks to fit instead of being cut', () => {
  const l = fitLineLayout(200, 220, 0);
  assert.equal(l.x, 0);
  assert.ok(l.scale >= FIT_MIN_SCALE && 220 * l.scale <= 200 - FIT_EDGE_PX + 1e-9);
  assert.equal(l.fadeRight, false, 'nothing is hidden');
});

test('a very long line slides to keep the sung word in view and reaches its end', () => {
  const need = 400;
  const start = fitLineLayout(200, need, 40);
  assert.equal(start.scale, FIT_MIN_SCALE);
  assert.equal(start.x, 0, 'early words: no slide yet');
  assert.ok(start.fadeRight && !start.fadeLeft);

  const mid = fitLineLayout(200, need, 250);
  assert.ok(mid.x < 0 && mid.fadeLeft && mid.fadeRight);
  // The sung word's end stays inside the visible width.
  assert.ok(250 * mid.scale + mid.x <= 200 - FIT_EDGE_PX + 1e-9);

  const end = fitLineLayout(200, need, need);
  const overflow = need * FIT_MIN_SCALE - (200 - FIT_EDGE_PX);
  assert.ok(Math.abs(end.x + overflow) < 1e-9, 'the last word is fully visible');
  assert.equal(end.fadeRight, false);
});

test('the card centres a shrunk line', () => {
  const l = fitLineLayout(200, 210, 0, true);
  const width = 210 * l.scale;
  assert.ok(Math.abs(l.x - (200 - FIT_EDGE_PX - width) / 2) < 1e-9);
});
