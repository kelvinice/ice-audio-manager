import assert from 'node:assert/strict';
import test from 'node:test';
import { VOLUME_CLICK_STEP, volumeClickPercent, volumeDragPercent, volumeFitsInline } from './volume.ts';

test('a click never jumps the volume, it only steps towards the click (D-28 B2)', () => {
  // The owner's case: 13 %, a click near the right end of the rail.
  assert.equal(volumeClickPercent(13, 95), 13 + VOLUME_CLICK_STEP);
  assert.equal(volumeClickPercent(13, 2), 13 - VOLUME_CLICK_STEP);
  assert.equal(volumeClickPercent(13, 14), 14, 'never past the clicked point');
  assert.equal(volumeClickPercent(13, 13.2), 13, 'on the thumb: no change');
  assert.equal(volumeClickPercent(100, 100), 100);
  assert.equal(volumeClickPercent(1, 0), 0);
});

test('dragging is relative to where it started, so grabbing anywhere is safe', () => {
  assert.equal(volumeDragPercent(13, 0, 84), 13, 'a press alone changes nothing');
  assert.equal(volumeDragPercent(13, 42, 84), 63, 'half the rail = +50 %');
  assert.equal(volumeDragPercent(13, -84, 84), 0, 'clamped at 0');
  assert.equal(volumeDragPercent(90, 84, 84), 100, 'clamped at 100');
  assert.equal(volumeDragPercent(40, 10, 0), 40, 'no width yet: unchanged');
});

test('the inline slider fits beside the chips only when there is room, else it becomes a chip', () => {
  assert.ok(volumeFitsInline(2, 372), 'Nook card (372 px strip), two chips');
  assert.ok(!volumeFitsInline(3, 372), 'three chips: chip mode');
  assert.ok(!volumeFitsInline(2, 296), 'lyrics split column');
  assert.ok(volumeFitsInline(1, 296));
  assert.ok(volumeFitsInline(0, 0), 'unmeasured: inline');
});
