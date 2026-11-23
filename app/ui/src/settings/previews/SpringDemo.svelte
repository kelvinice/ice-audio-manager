<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { SpringPreset } from '../../lib/ipc/engine';
  import { SpringGroup } from '../../lib/motion/spring.svelte';

  interface Props {
    preset: SpringPreset;
    /** Windows' animation effects are off: show the end state only. */
    reduceMotion: boolean;
  }

  let { preset, reduceMotion }: Props = $props();

  const NARROW = 44;
  const WIDE = 118;
  /** Rest between morphs while the pointer stays on the card. */
  const PAUSE_MS = 650;

  let width = $state(NARROW);
  let wide = false;
  let timer: ReturnType<typeof setTimeout> | null = null;
  const effective = $derived<SpringPreset>(reduceMotion ? 'off' : preset);
  const springs = new SpringGroup({ w: NARROW }, 'smooth', (v) => (width = v.w));

  $effect(() => springs.setPreset(effective));

  // The notch's own spring module drives the morph, so the demo is exactly what the preset does. It runs only while
  // the pointer is on the card (stop condition: pointer leaves or the page closes).
  function morph(): void {
    wide = !wide;
    springs.setTargets({ w: wide ? WIDE : NARROW });
    timer = setTimeout(morph, PAUSE_MS + (effective === 'off' ? 0 : 450));
  }

  function play(): void {
    if (timer === null) morph();
  }

  function stop(): void {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    wide = false;
    springs.setTargets({ w: NARROW });
  }

  onDestroy(() => {
    if (timer !== null) clearTimeout(timer);
    springs.setPaused(true);
  });

  const inset = $derived((WIDE - width) / 2);
</script>

<div class="desk" role="presentation" onpointerenter={play} onpointerleave={stop}>
  <div class="pill" style:clip-path={`inset(0 ${inset}px round 14px)`}>
    <span class="led"></span>
  </div>
</div>

<style>
  .desk {
    position: relative;
    height: 100%;
    background: var(--ice-set-desktop);
    overflow: hidden;
  }

  .pill {
    position: absolute;
    top: 14px;
    left: 50%;
    width: 118px;
    height: 28px;
    margin-left: -59px;
    display: grid;
    place-items: center;
    background: var(--ice-bg);
    box-shadow: inset 0 0 0 1px var(--ice-border);
  }

  .led {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ice-accent);
  }
</style>
