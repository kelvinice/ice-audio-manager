import assert from 'node:assert/strict';
import test from 'node:test';
import { CANVAS_W, TOP_PAD } from './geometry.ts';
import {
  buildNookFillPath,
  buildNookRimPath,
  floatingStyleStrategy,
  getNotchStyleStrategy,
  NOOK_EXPANDED_W_SINGLE,
  NOOK_EXPANDED_W_SPLIT,
  nookStyleStrategy,
} from './notchStyle.ts';

test('strategy registry returns nook by default and floating when selected', () => {
  assert.equal(getNotchStyleStrategy('floating'), floatingStyleStrategy);
  assert.equal(getNotchStyleStrategy(null), nookStyleStrategy);
  assert.equal(getNotchStyleStrategy(undefined), nookStyleStrategy);
  assert.equal(getNotchStyleStrategy('nook'), nookStyleStrategy);
  assert.equal(floatingStyleStrategy.topPad, TOP_PAD);
  assert.equal(nookStyleStrategy.topPad, 0);
});

test('expanded Nook widens into two-column split only when lyrics are active', () => {
  const base = {
    view: { kind: 'idle' } as const,
    expanded: true,
    micMuted: false,
    idleMode: 'mini_pill' as const,
    hasMedia: true,
    fallbackVolume: 42,
    pillLyricsExtraH: 0,
    cardLyricsExtraH: 82,
  };
  const floatBox = floatingStyleStrategy.targetGeometry({ ...base, showCardLyrics: true });
  const nookSingle = nookStyleStrategy.targetGeometry({ ...base, showCardLyrics: false });
  const nookSplit = nookStyleStrategy.targetGeometry({ ...base, showCardLyrics: true });

  assert.equal(nookSingle.w, NOOK_EXPANDED_W_SINGLE);
  assert.equal(nookSplit.w, NOOK_EXPANDED_W_SPLIT);
  assert.ok(nookSplit.w > nookSingle.w);
  assert.equal(nookSingle.h, nookSplit.h, 'toggling Edge Nook lyrics split only widens, with zero vertical jump');
  assert.ok(floatBox.h > nookSplit.h, 'Floating stacks lyrics vertically while Nook splits side-by-side');
});

test('Nook surface paths stay centered, flush to y=0, open along top rim, and safe under extreme springs', () => {
  const shape = nookStyleStrategy.computeSurfaceShape({ w: 292, h: 48, r: 19, vol: 40 });
  assert.equal(shape.topPad, 0);
  assert.equal(shape.earRadius, 8);
  assert.equal(shape.rimKind, 'path');
  assert.ok(shape.clipPath.startsWith("path('M "));
  assert.ok(shape.clipPath.endsWith(" Z')"));
  assert.ok(!shape.rimPathD.includes('Z'), 'rim must stay open across the top bezel edge');

  const left = (CANVAS_W - 292) / 2;
  const fill = buildNookFillPath(left, left + 292, 48, 19, 8);
  const rim = buildNookRimPath(left, left + 292, 48, 19, 8);
  assert.equal(`${rim} Z`, fill);

  // Overshoot / collapsed heights clamp cleanly without NaN or reversed arcs.
  const collapsed = nookStyleStrategy.computeSurfaceShape({ w: 96, h: 8, r: 4, vol: 0 });
  assert.equal(collapsed.earRadius, 0);
  assert.ok(!collapsed.clipPath.includes('NaN'));
});

test('hit rects stay top-aligned and expand for concave ears in Edge Nook mode', () => {
  const floatHit = floatingStyleStrategy.computeHitRect({ w: 116, h: 28 });
  assert.equal(floatHit.y, TOP_PAD);
  assert.equal(floatHit.width, 116);

  const nookHit = nookStyleStrategy.computeHitRect({ w: 116, h: 30 });
  assert.equal(nookHit.y, 0);
  assert.equal(nookHit.width, 116 + 16);
  assert.equal(nookHit.x, (CANVAS_W - nookHit.width) / 2);
});
