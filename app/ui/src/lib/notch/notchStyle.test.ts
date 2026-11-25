import assert from 'node:assert/strict';
import test from 'node:test';
import { BOTTOM_PAD, CANVAS_H, CANVAS_W, TOP_PAD } from './geometry.ts';
import {
  buildLiftedNookPaths,
  buildNookFillPath,
  buildNookRimPath,
  floatingStyleStrategy,
  LIFT_GAP_PX,
  getNotchStyleStrategy,
  NOOK_EXPANDED_W_SINGLE,
  NOOK_EXPANDED_W_SPLIT,
  nookStyleStrategy,
  plainGeometryContext,
} from './notchStyle.ts';

test('a resting Edge Nook (lift 0) keeps exactly the docked outline, top and bottom', () => {
  for (const bottom of [false, true]) {
    for (const ear of [0, 8]) {
      const p = buildLiftedNookPaths(260, 388, 28, 13, ear, 0, bottom);
      // The docked builder writes the flush edge as a bare `0`; same coordinate.
      const norm = (d: string) => d.replace(/\b0\.00\b/g, '0');
      assert.equal(norm(p.rim), norm(buildNookRimPath(260, 388, 28, 13, ear, bottom)));
      assert.equal(norm(p.fill), norm(buildNookFillPath(260, 388, 28, 13, ear, bottom)));
      assert.equal(p.gap, 0);
    }
  }
});

test('a held Edge Nook (lift 1) is a full capsule off the edge, with no ears', () => {
  const top = nookStyleStrategy.computeSurfaceShape({ w: 116, h: 28, r: 13, vol: 40 }, false, 1);
  assert.equal(top.earRadius, 0);
  assert.equal(top.insetTop, LIFT_GAP_PX);
  assert.equal(top.topPad, LIFT_GAP_PX);
  assert.equal(top.shadowRadiusCss, '13.00px 13.00px 13.00px 13.00px');
  // Four convex corners (sweep 0), no concave ear arcs, and the rim closes over the edge side.
  assert.equal((top.rimPathD.match(/ A /g) ?? []).length, 4);
  assert.ok(!/ A [\d.]+ [\d.]+ 0 0 1 /.test(top.rimPathD), 'no concave arcs');
  assert.ok(top.rimPathD.startsWith(`M ${(CANVAS_W / 2 - 58).toFixed(2)} ${(LIFT_GAP_PX + 13).toFixed(2)}`));

  const bottom = nookStyleStrategy.computeSurfaceShape({ w: 116, h: 28, r: 13, vol: 40 }, true, 1);
  assert.equal(bottom.insetTop, CANVAS_H - 28 - LIFT_GAP_PX);
  assert.equal(bottom.bottomPad, LIFT_GAP_PX);
  assert.ok(!/ A [\d.]+ [\d.]+ 0 0 0 /.test(bottom.rimPathD), 'mirrored: every corner sweeps the other way');
});

test('halfway through a lift the ears are gone before the edge corners round', () => {
  const quarter = buildLiftedNookPaths(260, 388, 28, 13, 8, 0.25, false);
  assert.ok(quarter.ear > 0 && quarter.ear < 8);
  assert.equal(quarter.edgeR, 0);
  const half = buildLiftedNookPaths(260, 388, 28, 13, 8, 0.5, false);
  assert.equal(half.ear, 0);
  assert.equal(half.edgeR, 0);
  const most = buildLiftedNookPaths(260, 388, 28, 13, 8, 0.9, false);
  assert.ok(most.edgeR > 9 && most.edgeR < 13);
});

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

test('hibernate keeps the plain idle shape whatever plays, pops up or is muted', () => {
  const busy = {
    view: { kind: 'track', media: null } as never,
    expanded: true,
    micMuted: true,
    idleMode: 'mini_pill' as const,
    hasMedia: true,
    fallbackVolume: 42,
    pillLyricsExtraH: 30,
    cardLyricsExtraH: 82,
    showCardLyrics: true,
  };
  const quiet = { ...busy, view: { kind: 'idle' } as const, expanded: false, micMuted: false, hasMedia: false };
  for (const strategy of [nookStyleStrategy, floatingStyleStrategy]) {
    const plain = strategy.targetGeometry(plainGeometryContext(busy));
    const idle = strategy.targetGeometry({ ...quiet, pillLyricsExtraH: 0, cardLyricsExtraH: 0, showCardLyrics: false });
    assert.deepEqual(plain, idle, strategy.id);
  }
});

test('D-20 bottom anchors measure from BOTTOM_PAD in Floating and flush on CANVAS_H in Edge Nook', () => {
  const pill = { w: 116, h: 28, r: 14, vol: 40 };
  const floatBottom = floatingStyleStrategy.computeSurfaceShape(pill, true);
  assert.equal(floatBottom.isBottom, true);
  assert.equal(floatBottom.bottomPad, BOTTOM_PAD);
  assert.equal(floatBottom.insetTop, CANVAS_H - BOTTOM_PAD - pill.h);
  assert.ok(floatBottom.clipPath.includes(`inset(${floatBottom.insetTop.toFixed(2)}px`));

  const floatBottomHit = floatingStyleStrategy.computeHitRect({ w: 392, h: 164 }, true);
  assert.equal(floatBottomHit.y, CANVAS_H - BOTTOM_PAD - 164);

  const nookBottom = nookStyleStrategy.computeSurfaceShape(pill, true);
  assert.equal(nookBottom.isBottom, true);
  assert.equal(nookBottom.bottomPad, 0);
  assert.equal(nookBottom.insetTop, CANVAS_H - pill.h);
  assert.ok(nookBottom.rimPathD.includes(`${CANVAS_H.toFixed(2)}`));
  assert.ok(nookBottom.rimPathD.includes(`${(CANVAS_H - pill.h).toFixed(2)}`));
  const nookBottomHit = nookStyleStrategy.computeHitRect({ w: 116, h: 30 }, true);
  assert.equal(nookBottomHit.y, CANVAS_H - 30);
});
