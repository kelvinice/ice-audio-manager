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
  const lastClips: string[] = [];
  const lastOpacities: string[] = [];
  let available = $state(true);
  let handle: VisualizerHandle | null = null;

  function applyLevels(levels: ReadonlyArray<number>, nextAvailable: boolean): void {
    available = nextAvailable;
    if (!nextAvailable) {
      lastClips.length = 0;
      lastOpacities.length = 0;
      return;
    }
    for (let i = 0; i < BAR_INDICES.length; i += 1) {
      const el = barEls[i];
      if (!el) continue;
      const level = levels[i] ?? 0;
      // Clip a full-height bar from both ends instead of `scaleY`, which squashed the round caps into blocks.
      const insetPct = ((1 - barScaleY(level)) * 50).toFixed(2);
      const nextClip = `inset(${insetPct}% 0 ${insetPct}% 0 round 999px)`;
      if (lastClips[i] !== nextClip) {
        lastClips[i] = nextClip;
        el.style.clipPath = nextClip;
      }
      const nextOpacity = (0.72 + 0.28 * level).toFixed(2);
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

{#if available}
  <div class="visualizer-bars" class:expanded={variant === 'expanded'} aria-hidden="true">
    {#each BAR_INDICES as idx (idx)}
      <span class="bar" bind:this={barEls[idx]}></span>
    {/each}
  </div>
{/if}

<style>
  .visualizer-bars {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    /* Whole-pixel width and gap stay crisp at 100 % and 150 % scaling (fractional sizes blurred into blocks). */
    gap: 2px;
    height: 12px;
    flex-shrink: 0;
  }

  .visualizer-bars.expanded {
    height: 16px;
    padding: 0 2px;
  }

  .bar {
    width: 2px;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-active-accent, var(--ice-accent));
    /* Resting = MIN_BAR_SCALE (0.18): clip 41 % from each end. */
    clip-path: inset(41% 0 41% 0 round 999px);
    opacity: 0.72;
  }
</style>
