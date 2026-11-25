<script lang="ts">
  // Experimental "Landing ripple" (owner, 2026-11-26): when a dropped notch lands, an accent copy of its outline
  // spreads out from the edge it docked on and fades. One-shot: mounted per landing (`{#key}`), transform + opacity
  // only, never shown with reduce motion / preset off (the caller checks).
  import { CANVAS_H, CANVAS_W } from '../../lib/notch/geometry';
  import type { SurfaceShape } from '../../lib/notch/notchStyle';

  interface Props {
    shape: SurfaceShape;
    pill: { w: number; h: number; r: number };
  }

  let { shape, pill }: Props = $props();
</script>

<svg
  class="ripple-svg"
  class:path={shape.rimKind === 'path'}
  class:bottom={shape.isBottom}
  width={CANVAS_W}
  height={CANVAS_H}
  viewBox={`0 0 ${CANVAS_W} ${CANVAS_H}`}
  aria-hidden="true"
>
  {#if shape.rimKind === 'path'}
    <path class="ring" d={shape.rimPathD} fill="none" vector-effect="non-scaling-stroke" />
  {:else}
    <rect
      class="ring"
      x={shape.insetLeft + 0.5}
      y={shape.insetTop + 0.5}
      width={Math.max(1, pill.w - 1)}
      height={Math.max(1, pill.h - 1)}
      rx={pill.r}
      ry={pill.r}
      fill="none"
      vector-effect="non-scaling-stroke"
    />
  {/if}
</svg>

<style>
  .ripple-svg {
    position: absolute;
    inset: 0;
    overflow: visible;
    pointer-events: none;
  }

  .ring {
    stroke: var(--ice-active-accent, var(--ice-accent));
    stroke-width: 2;
    transform-box: fill-box;
    transform-origin: 50% 50%;
    animation: notch-ripple 640ms cubic-bezier(0.2, 0.7, 0.3, 1) both;
  }

  /* Edge Nook spreads away from the edge it docks on. */
  .path .ring {
    transform-origin: 50% 0%;
  }

  .path.bottom .ring {
    transform-origin: 50% 100%;
  }

  @keyframes notch-ripple {
    0% {
      opacity: 0.85;
      transform: scale(1, 1);
    }
    100% {
      opacity: 0;
      transform: scale(1.22, 1.9);
    }
  }
</style>
