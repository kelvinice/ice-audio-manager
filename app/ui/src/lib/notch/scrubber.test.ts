import assert from 'node:assert/strict';
import test from 'node:test';
import {
  clampSeekPositionMs,
  liveMediaPositionMs,
  nextSecondTickDelayMs,
  pointerSeekMs,
  stepSeekMs,
} from './scrubber.ts';

test('clampSeekPositionMs clamps seek targets within valid track bounds', () => {
  assert.equal(clampSeekPositionMs(-500, 180_000), 0);
  assert.equal(clampSeekPositionMs(45_200.4, 180_000), 45_200);
  assert.equal(clampSeekPositionMs(220_000, 180_000), 180_000);
  assert.equal(clampSeekPositionMs(10_000, null), 0);
  assert.equal(clampSeekPositionMs(10_000, 0), 0);
});

test('liveMediaPositionMs extrapolates smoothly from positionUpdatedMs while playing and freezes when paused', () => {
  const t0 = 1_700_000_000_000;
  assert.equal(
    liveMediaPositionMs(
      { status: 'playing', positionMs: 12_000n, durationMs: 180_000n, positionUpdatedMs: t0 },
      t0 + 15_400,
      t0 + 10_000,
    ),
    27_400,
  );
  assert.equal(
    liveMediaPositionMs(
      { status: 'paused', positionMs: 12_000n, durationMs: 180_000n, positionUpdatedMs: t0 },
      t0 + 15_400,
      t0,
    ),
    12_000,
  );
  assert.equal(
    liveMediaPositionMs(
      { status: 'playing', positionMs: 170_000n, durationMs: 180_000n, positionUpdatedMs: t0 },
      t0 + 30_000,
      t0,
    ),
    180_000,
  );
  assert.equal(
    liveMediaPositionMs(
      { status: 'playing', positionMs: 5_000n, durationMs: 180_000n, positionUpdatedMs: null },
      t0 + 4_000,
      t0 + 1_000,
    ),
    8_000,
  );
});

test('pointerSeekMs maps pointer X across track width cleanly', () => {
  assert.equal(pointerSeekMs(100, 100, 200, 180_000), 0);
  assert.equal(pointerSeekMs(200, 100, 200, 180_000), 90_000);
  assert.equal(pointerSeekMs(300, 100, 200, 180_000), 180_000);
  assert.equal(pointerSeekMs(50, 100, 200, 180_000), 0);
  assert.equal(pointerSeekMs(380, 100, 200, 180_000), 180_000);
  assert.equal(pointerSeekMs(150, 100, 0, 180_000), null);
  assert.equal(pointerSeekMs(150, 100, 200, null), null);
});

test('stepSeekMs steps keyboard seeks and clamps at ends', () => {
  assert.equal(stepSeekMs(10_000, 5_000, 180_000), 15_000);
  assert.equal(stepSeekMs(2_000, -5_000, 180_000), 0);
  assert.equal(stepSeekMs(178_000, 5_000, 180_000), 180_000);
  assert.equal(stepSeekMs(10_000, 5_000, null), null);
});

test('nextSecondTickDelayMs computes aligned whole-second tick delays', () => {
  assert.equal(nextSecondTickDelayMs(0), 1025);
  assert.equal(nextSecondTickDelayMs(250), 775);
  assert.equal(nextSecondTickDelayMs(980), 200);
});
