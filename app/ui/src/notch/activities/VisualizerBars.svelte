<script lang="ts">
  import { onMount } from 'svelte';
  import type { SpringPreset } from '../../lib/ipc/types';
  import {
    barScaleY,
    DEFAULT_BAR_COUNT,
    spreadLevels,
    subscribeVisualizer,
    WIDE_BAR_COUNT,
    type VisualizerHandle,
  } from '../../lib/notch/visualizer';

  interface Props {
    active: boolean;
    preset?: SpringPreset;
    instant?: boolean;
    /** `wide`: the now-playing pill's waveform scrubber (more bars, resting shape, progress colouring). */
    variant?: 'compact' | 'expanded' | 'wide';
    /** `wide` only: played fraction 0–1 (bars up to it use the accent); `null` = unknown (all accent). */
    progress?: number | null;
  }

  let {
    active,
    preset = 'smooth',
    instant = false,
    variant = 'compact',
    progress = null,
  }: Props = $props();

  // The variant never changes for a mounted bar set.
  // svelte-ignore state_referenced_locally
  const wide = variant === 'wide';
  const barIndices = Array.from({ length: wide ? WIDE_BAR_COUNT : DEFAULT_BAR_COUNT }, (_, i) => i);
  const playedCount = $derived(
    progress === null ? barIndices.length : Math.round(Math.max(0, Math.min(1, progress)) * barIndices.length),
  );

  const barEls: Array<HTMLSpanElement | undefined> = [];
  const lastClips: string[] = [];
  const lastOpacities: string[] = [];
  const wideLevels: number[] = [];
  let available = $state(true);
  let handle: VisualizerHandle | null = null;

  function applyLevels(rawLevels: ReadonlyArray<number>, nextAvailable: boolean): void {
    available = nextAvailable;
    // The wide waveform stays visible without a meter (resting shape), so its progress still reads.
    if (!nextAvailable && !wide) {
      lastClips.length = 0;
      lastOpacities.length = 0;
      return;
    }
    const levels = wide ? spreadLevels(nextAvailable ? rawLevels : [], barIndices.length, wideLevels) : rawLevels;
    for (let i = 0; i < barIndices.length; i += 1) {
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

{#if available || wide}
  <div class="visualizer-bars" class:expanded={variant === 'expanded'} class:wide aria-hidden="true">
    {#each barIndices as idx (idx)}
      <span class="bar" class:unplayed={wide && idx >= playedCount} bind:this={barEls[idx]}></span>
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

  /* Waveform scrubber: 14 × 2 px bars, 3 px gaps (whole pixels at 100 % and 150 %). */
  .visualizer-bars.wide {
    gap: 3px;
    height: 24px;
  }

  /* Not heard yet: a dim neutral, so the lit accent part reads as progress. */
  .bar.unplayed {
    background: color-mix(in srgb, var(--ice-fg-muted) 50%, transparent);
    transition: background-color 400ms ease;
  }
</style>
