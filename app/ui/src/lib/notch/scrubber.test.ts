import assert from 'node:assert/strict';
import test from 'node:test';
import {
  applyOptimisticSeek,
  clampSeekPositionMs,
  liveMediaPositionMs,
  mediaTrackKey,
  nextSecondTickDelayMs,
  OPTIMISTIC_SEEK_HOLD_MS,
  pointerSeekMs,
  shouldClearOptimisticSeek,
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

test('optimistic seek projects position immediately and clears on track change, expiry, or confirmation', () => {
  const t0 = 1_700_000_000_000;
  const base = {
    appId: 'Spotify.exe',
    title: 'Song A',
    artist: 'Artist',
    album: 'Album',
    status: 'playing' as const,
    positionMs: 10_000n,
    durationMs: 180_000n,
    positionUpdatedMs: t0,
    art: null,
  };
  const pending = {
    trackKey: mediaTrackKey(base),
    targetMs: 90_000,
    setAtEpochMs: t0,
    untilEpochMs: t0 + OPTIMISTIC_SEEK_HOLD_MS,
  };

  // Before confirmation or expiry, applied overlay overrides position and timestamp.
  const projected = applyOptimisticSeek(base, pending, t0 + 400);
  assert.equal(projected?.positionMs, 90_000n);
  assert.equal(projected?.positionUpdatedMs, t0);
  assert.equal(shouldClearOptimisticSeek(pending, base, t0 + 400), false);

  // Confirming near target clears pending; different track or timeout also clears pending.
  assert.equal(shouldClearOptimisticSeek(pending, { ...base, positionMs: 91_200n, positionUpdatedMs: t0 + 500 }, t0 + 500), true);
  assert.equal(shouldClearOptimisticSeek(pending, { ...base, title: 'Song B' }, t0 + 200), true);
  assert.equal(shouldClearOptimisticSeek(pending, base, t0 + OPTIMISTIC_SEEK_HOLD_MS), true);
  assert.equal(applyOptimisticSeek({ ...base, title: 'Song B' }, pending, t0 + 200)?.positionMs, 10_000n);
});
