import assert from 'node:assert/strict';
import test from 'node:test';
import {
  DRAG_THRESHOLD_PX,
  INTERACTIVE_DRAG_BLOCK_SELECTOR,
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
