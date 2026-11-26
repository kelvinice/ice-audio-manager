<script lang="ts">
  // Experimental "Landing ripple" (owner, 2026-11-26: "come out of the outer side of the current pill shape"): like a
  // drop landing on water, three softly wavy rings rise off the pill's own outline — the wide music pill, the dot, a
  // docked Edge Nook (open rings against the edge) — and travel out the same distance on every side, a beat apart,
  // behind a soft accent halo. Masked to outside the pill, so it never tints the pill itself. One-shot per landing
  // (`{#key}` in NotchApp), `transform` + `opacity` only; never with reduce motion / preset off (the caller checks).
  import { CANVAS_H, CANVAS_W } from '../../lib/notch/geometry';
  import type { SurfaceShape } from '../../lib/notch/notchStyle';
  import { pillRippleRing, type RipplePill } from '../../lib/notch/waves';

  interface Props {
    shape: SurfaceShape;
    pill: { w: number; h: number; r: number };
    /** Solid accent colour (the song's, or the theme's). */
    accent: string;
  }

  let { shape, pill, accent }: Props = $props();

  /** How far each ring travels from the pill's edge (px), and how far off the edge it starts. */
  const RING_OFFSET = 22;
  const HALO_OFFSET = 12;
  const START_OFFSET = 1.5;
  const RINGS = 3;
  const id = `ripple-${Math.random().toString(36).slice(2, 8)}`;

  const body = $derived.by((): RipplePill => {
    const w = Math.max(1, pill.w);
    const h = Math.max(1, pill.h);
    if (shape.rimKind === 'path') {
      const dock = shape.isBottom ? 'bottom' : 'top';
      const r = Math.max(0, Math.min(pill.r, w / 2, h));
      return { left: (CANVAS_W - w) / 2, top: shape.isBottom ? CANVAS_H - h : 0, w, h, r, dock };
    }
    const r = Math.max(0, Math.min(pill.r, w / 2, h / 2));
    return { left: shape.insetLeft, top: shape.insetTop, w, h, r, dock: 'none' };
  });

  const rings = $derived(
    Array.from({ length: RINGS }, (_, i) =>
      pillRippleRing(body, RING_OFFSET, {
        amp: 1.6 - i * 0.35,
        wavelength: 34,
        phase: i * 1.9,
        startOffset: START_OFFSET,
        canvasH: CANVAS_H,
      }),
    ),
  );

  const halo = $derived(
    pillRippleRing(body, HALO_OFFSET, { amp: 0, wavelength: 34, startOffset: 0, canvasH: CANVAS_H }),
  );
</script>

<svg class="ripple-svg" width={CANVAS_W} height={CANVAS_H} viewBox={`0 0 ${CANVAS_W} ${CANVAS_H}`} aria-hidden="true">
  <defs>
    <!-- Everything but the pill (Edge Nook: its docked outline incl. ears, closed along the edge). -->
    <mask id={id} maskUnits="userSpaceOnUse" x={-CANVAS_W} y={-CANVAS_H} width={CANVAS_W * 3} height={CANVAS_H * 3}>
      <rect x={-CANVAS_W} y={-CANVAS_H} width={CANVAS_W * 3} height={CANVAS_H * 3} fill="white" />
      {#if shape.rimKind === 'path'}
        <path d={`${shape.rimPathD} Z`} fill="black" />
      {:else}
        <rect x={body.left} y={body.top} width={body.w} height={body.h} rx={body.r} ry={body.r} fill="black" />
      {/if}
    </mask>
  </defs>
  <g mask={`url(#${id})`}>
    <path
      class="halo"
      d={halo.d}
      fill="none"
      stroke={accent}
      vector-effect="non-scaling-stroke"
      style:transform-origin={`${halo.cx}px ${halo.cy}px`}
      style:--sx0={halo.sx0.toFixed(4)}
      style:--sy0={halo.sy0.toFixed(4)}
    />
    {#each rings as ring, i (i)}
      <path
        class="ring"
        d={ring.d}
        fill="none"
        stroke={accent}
        vector-effect="non-scaling-stroke"
        style:transform-origin={`${ring.cx}px ${ring.cy}px`}
        style:animation-delay={`${i * 150}ms`}
        style:--sx0={ring.sx0.toFixed(4)}
        style:--sy0={ring.sy0.toFixed(4)}
        style:--ring-peak={(0.85 - i * 0.2).toFixed(2)}
      />
    {/each}
  </g>
</svg>

<style>
  .ripple-svg {
    position: absolute;
    inset: 0;
    overflow: visible;
    pointer-events: none;
  }

  .ring {
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0;
    animation: notch-ripple-ring 1000ms cubic-bezier(0.2, 0.65, 0.3, 1) both;
  }

  /* A soft band of the song's colour that swells off the edge first, under the rings. */
  .halo {
    stroke-width: 7;
    filter: blur(3px);
    opacity: 0;
    animation: notch-ripple-halo 720ms cubic-bezier(0.2, 0.65, 0.3, 1) both;
  }

  @keyframes notch-ripple-ring {
    0% {
      opacity: 0;
      transform: scale(var(--sx0, 1), var(--sy0, 1));
    }
    10% {
      opacity: var(--ring-peak, 0.85);
    }
    100% {
      opacity: 0;
      transform: scale(1, 1);
    }
  }

  @keyframes notch-ripple-halo {
    0% {
      opacity: 0.55;
      transform: scale(var(--sx0, 1), var(--sy0, 1));
    }
    100% {
      opacity: 0;
      transform: scale(1, 1);
    }
  }
</style>
