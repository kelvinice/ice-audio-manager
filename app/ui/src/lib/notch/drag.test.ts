import assert from 'node:assert/strict';
import test from 'node:test';
import {
  DRAG_THRESHOLD_PX,
  INTERACTIVE_DRAG_BLOCK_SELECTOR,
  isPassingThrough,
  isDragBlockedTarget,
  shouldStartDrag,
} from './drag.ts';

test('shouldStartDrag triggers only beyond threshold', () => {
  assert.equal(shouldStartDrag(100, 50, 100 + DRAG_THRESHOLD_PX, 50 + DRAG_THRESHOLD_PX), false);
  assert.equal(shouldStartDrag(100, 50, 100 + DRAG_THRESHOLD_PX + 1, 50), true);
  assert.equal(shouldStartDrag(100, 50, 100, 50 - DRAG_THRESHOLD_PX - 1), true);
});

test('isDragBlockedTarget respects interactive controls and ignores surface elements', () => {
  assert.equal(isDragBlockedTarget(null), false);
  assert.equal(
    isDragBlockedTarget({
      closest: (sel: string) => {
        assert.equal(sel, INTERACTIVE_DRAG_BLOCK_SELECTOR);
        return { tagName: 'BUTTON' };
      },
    } as unknown as EventTarget),
    true,
  );
  assert.equal(
    isDragBlockedTarget({
      closest: () => null,
    } as unknown as EventTarget),
    false,
  );
});

test('a fast pointer is passing through; a slow or settled one is not', () => {
  assert.equal(isPassingThrough(40, 0, 16), true, '2.5 px/ms: sweeping to a tab');
  assert.equal(isPassingThrough(3, 2, 16), false, 'about 0.2 px/ms: settling on the notch');
  assert.equal(isPassingThrough(5, 0, 0), false, 'no time elapsed: no speed');
});
