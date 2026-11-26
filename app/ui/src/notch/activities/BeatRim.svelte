<script lang="ts">
  // Experimental "Beat rim" (owner, 2026-11-26, `appearance.experiments.beatRim`): the music pill's rim lights up in the
  // song's colour on each bass kick and fades between them. Driven by `subscribeBreath`, the kick envelope the lava lamp
  // already uses: it only emits while the meter runs for the pill's waveform, so this adds no timer, stream or loop.
  // One `opacity` write per changed value, straight to the element. The caller shows it only on the playing music
  // pill, never with reduce motion / preset off.
  import { CANVAS_H, CANVAS_W } from '../../lib/notch/geometry';
  import type { SurfaceShape } from '../../lib/notch/notchStyle';
  import { subscribeBreath } from '../../lib/notch/visualizer';

  interface Props {
    shape: SurfaceShape;
    insetLeft: number;
    pill: { w: number; h: number; r: number };
    /** Solid accent colour (the song's, or the theme's). */
    accent: string;
  }

  let { shape, insetLeft, pill, accent }: Props = $props();

  /** Brightest the rim gets on a full kick. */
  const PEAK = 0.95;

  let layer = $state<SVGGElement | null>(null);

  $effect(() => {
    const node = layer;
    if (!node) return;
    const stop = subscribeBreath((breath) => {
      node.style.opacity = (PEAK * Math.max(0, Math.min(1, breath))).toFixed(3);
    });
    return () => {
      stop();
      node.style.opacity = '0';
    };
  });
</script>

<svg class="beat-rim-svg" width={CANVAS_W} height={CANVAS_H} viewBox={`0 0 ${CANVAS_W} ${CANVAS_H}`} aria-hidden="true">
  <g bind:this={layer} class="beat-rim" style:--beat-c={accent}>
    {#if shape.rimKind === 'path'}
      <path d={shape.rimPathD} fill="none" stroke={accent} stroke-width="2" vector-effect="non-scaling-stroke" />
    {:else}
      <rect
        x={insetLeft + 0.5}
        y={shape.insetTop + 0.5}
        width={Math.max(1, pill.w - 1)}
        height={Math.max(1, pill.h - 1)}
        rx={pill.r}
        ry={pill.r}
        fill="none"
        stroke={accent}
        stroke-width="1.5"
        vector-effect="non-scaling-stroke"
      />
    {/if}
  </g>
</svg>

<style>
  .beat-rim-svg {
    position: absolute;
    inset: 0;
    overflow: visible;
    pointer-events: none;
  }

  /* The glow is static; only the group's opacity changes, so the compositor just fades a cached layer. */
  .beat-rim {
    opacity: 0;
    filter: drop-shadow(0 0 4px var(--beat-c)) drop-shadow(0 0 9px color-mix(in srgb, var(--beat-c) 55%, transparent));
    will-change: opacity;
  }
</style>
