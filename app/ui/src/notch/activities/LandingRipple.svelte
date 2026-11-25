<script lang="ts">
  // Experimental "Landing ripple" (owner, 2026-11-26: "more elegant, something wavy"): like a drop landing on water,
  // three softly wavy rings spread from the edge the notch docked on, a beat apart, over a faint bloom of the song's
  // colour. One-shot per landing (`{#key}` in NotchApp), `transform` + `opacity` only; never with reduce motion /
  // preset off (the caller checks).
  import { CANVAS_H, CANVAS_W } from '../../lib/notch/geometry';
  import type { SurfaceShape } from '../../lib/notch/notchStyle';
  import { wavyEllipse } from '../../lib/notch/waves';

  interface Props {
    shape: SurfaceShape;
    pill: { w: number; h: number };
    /** Solid accent colour (the song's, or the theme's). */
    accent: string;
  }

  let { shape, pill, accent }: Props = $props();

  const RINGS = 3;
  const id = `ripple-${Math.random().toString(36).slice(2, 8)}`;

  // Edge Nook: half rings hugging the docked edge; Floating: full rings around the pill.
  const geo = $derived.by(() => {
    const docked = shape.rimKind === 'path';
    const cx = CANVAS_W / 2;
    if (!docked) {
      return { cx, cy: shape.insetTop + pill.h / 2, rx: pill.w / 2 + 3, ry: pill.h / 2 + 3, from: 0, to: Math.PI * 2 };
    }
    const cy = shape.isBottom ? CANVAS_H : 0;
    const [from, to] = shape.isBottom ? [Math.PI, Math.PI * 2] : [0, Math.PI];
    return { cx, cy, rx: pill.w / 2 + shape.earRadius + 3, ry: pill.h + 3, from, to };
  });

  const rings = $derived(
    Array.from({ length: RINGS }, (_, i) =>
      wavyEllipse(geo.cx, geo.cy, geo.rx, geo.ry, {
        waves: geo.to - geo.from > Math.PI ? 10 : 6,
        amp: 2.4 - i * 0.5,
        phase: i * 1.9,
        from: geo.from,
        to: geo.to,
      }),
    ),
  );
</script>

<svg class="ripple-svg" width={CANVAS_W} height={CANVAS_H} viewBox={`0 0 ${CANVAS_W} ${CANVAS_H}`} aria-hidden="true">
  <defs>
    <radialGradient id={id}>
      <stop offset="0%" stop-color={accent} stop-opacity="0.32" />
      <stop offset="60%" stop-color={accent} stop-opacity="0.12" />
      <stop offset="100%" stop-color={accent} stop-opacity="0" />
    </radialGradient>
  </defs>
  <ellipse
    class="bloom"
    cx={geo.cx}
    cy={geo.cy}
    rx={geo.rx * 1.15}
    ry={geo.ry * 1.15}
    fill={`url(#${id})`}
    style:transform-origin={`${geo.cx}px ${geo.cy}px`}
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
      style:--ring-peak={(0.8 - i * 0.18).toFixed(2)}
    />
  {/each}
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
    opacity: 0;
    animation: notch-ripple-ring 1000ms cubic-bezier(0.2, 0.65, 0.3, 1) both;
  }

  .bloom {
    opacity: 0;
    animation: notch-ripple-bloom 760ms cubic-bezier(0.2, 0.65, 0.3, 1) both;
  }

  @keyframes notch-ripple-ring {
    0% {
      opacity: 0;
      transform: scale(1, 1);
    }
    12% {
      opacity: var(--ring-peak, 0.8);
    }
    100% {
      opacity: 0;
      transform: scale(1.32, 1.75);
    }
  }

  @keyframes notch-ripple-bloom {
    0% {
      opacity: 0.9;
      transform: scale(0.85);
    }
    100% {
      opacity: 0;
      transform: scale(1.35);
    }
  }
</style>
