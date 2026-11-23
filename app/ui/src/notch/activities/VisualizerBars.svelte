<script lang="ts">
  import { onMount } from 'svelte';
  import type { SpringPreset } from '../../lib/ipc/types';
  import {
    barScaleY,
    DEFAULT_BAR_COUNT,
    subscribeVisualizer,
    type VisualizerHandle,
  } from '../../lib/notch/visualizer';

  interface Props {
    active: boolean;
    preset?: SpringPreset;
    instant?: boolean;
    variant?: 'compact' | 'expanded';
  }

  const BAR_INDICES = Array.from({ length: DEFAULT_BAR_COUNT }, (_, i) => i);

  let {
    active,
    preset = 'smooth',
    instant = false,
    variant = 'compact',
  }: Props = $props();

  const barEls: Array<HTMLSpanElement | undefined> = [];
  const lastTransforms: string[] = [];
  const lastOpacities: string[] = [];
  let handle: VisualizerHandle | null = null;

  function applyLevels(levels: ReadonlyArray<number>): void {
    for (let i = 0; i < BAR_INDICES.length; i += 1) {
      const el = barEls[i];
      if (!el) continue;
      const level = levels[i] ?? 0;
      const nextTransform = `scaleY(${barScaleY(level).toFixed(4)})`;
      if (lastTransforms[i] !== nextTransform) {
        lastTransforms[i] = nextTransform;
        el.style.transform = nextTransform;
      }
      const nextOpacity = (0.65 + 0.35 * level).toFixed(3);
      if (lastOpacities[i] !== nextOpacity) {
        lastOpacities[i] = nextOpacity;
        el.style.opacity = nextOpacity;
      }
    }
  }

  onMount(() => {
    const sub = subscribeVisualizer(applyLevels, { active, preset, instant });
    handle = sub;
    return () => {
      sub.dispose();
      handle = null;
    };
  });

  $effect(() => {
    handle?.update({ active, preset, instant });
  });
</script>

<div class="visualizer-bars" class:expanded={variant === 'expanded'} aria-hidden="true">
  {#each BAR_INDICES as idx (idx)}
    <span class="bar" bind:this={barEls[idx]}></span>
  {/each}
</div>

<style>
  .visualizer-bars {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 1.5px;
    height: 12px;
    flex-shrink: 0;
  }

  .visualizer-bars.expanded {
    gap: 2px;
    height: 16px;
    padding: 0 2px;
  }

  .bar {
    width: 2px;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-active-accent, var(--ice-accent));
    transform-origin: center center;
    transform: scaleY(0.18);
    opacity: 0.65;
    will-change: transform, opacity;
  }

  .visualizer-bars.expanded .bar {
    width: 2.5px;
  }
</style>
