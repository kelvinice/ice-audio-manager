<script lang="ts">
  // "Lively" Now Playing background, after the Ice notch's lava lamp (app/ui/src/notch/activities/NotchBackdrop.svelte):
  // nine soft orbs in the cover's colours, each looping around its own spot while its distance and size swell and
  // ease (closed loops on coprime-ish periods, so the pattern never visibly repeats), and the whole field breathing
  // with the bass (`lib/breath.ts`). Transform-only. The breath samples the analyser at most 30 Hz, only while playing
  // and mounted; reduced motion skips it (and app.css stops the loops).
  import { onDestroy } from "svelte";
  import { audioEngine } from "../lib/audio";
  import { BassBreath, BREATH_SCALE } from "../lib/breath";

  interface Props {
    playing: boolean;
  }

  let { playing }: Props = $props();

  const C1 = "var(--art-primary, var(--p-tone))";
  const C2 = "var(--art-secondary, var(--p-companion))";
  const C3 = "var(--art-tertiary, var(--p-seed))";
  const mix = (a: string, b: string) => `color-mix(in oklab, ${a}, ${b})`;

  /** Base loop length; each orb runs at its own multiple of it. */
  const PERIOD_S = 26;

  /**
   * One orb: its spot (% of the page), size (vmax), colour, period multiple, start angle and direction, the four
   * distances from its spot (vmin) and four sizes it passes through on one loop.
   */
  const ORBS = [
    { x: 18, y: 24, size: 46, c: C1, k: 0.83, a: 20, dir: 1, r: [9, 13, 10, 14], s: [0.96, 1.08, 1, 1.12] },
    { x: 80, y: 28, size: 44, c: C2, k: 1.0, a: 140, dir: -1, r: [8, 12, 9.5, 13], s: [1.06, 0.94, 1.1, 0.98] },
    { x: 42, y: 60, size: 50, c: C3, k: 0.91, a: 250, dir: 1, r: [7, 11, 8.5, 12], s: [0.95, 1.1, 0.98, 1.06] },
    { x: 62, y: 36, size: 42, c: mix(C1, C3), k: 1.17, a: 80, dir: -1, r: [9, 12.5, 10, 13.5], s: [1.04, 0.95, 1.12, 1] },
    { x: 90, y: 70, size: 46, c: mix(C2, C1), k: 0.77, a: 300, dir: 1, r: [8, 11.5, 9, 12.5], s: [0.97, 1.09, 1.02, 1.11] },
    { x: 24, y: 78, size: 38, c: C2, k: 1.29, a: 190, dir: -1, r: [9.5, 13, 10.5, 14], s: [1.05, 0.96, 1.1, 0.94] },
    { x: 50, y: 20, size: 44, c: C1, k: 1.09, a: 40, dir: 1, r: [11, 15, 12, 16], s: [0.94, 1.1, 1, 1.08] },
    { x: 72, y: 84, size: 40, c: C3, k: 0.95, a: 220, dir: -1, r: [8, 11.5, 9, 12.5], s: [1.08, 0.96, 1.04, 0.92] },
    { x: 6, y: 50, size: 42, c: mix(C3, C2), k: 1.37, a: 330, dir: 1, r: [8.5, 12, 9.5, 13], s: [0.96, 1.06, 0.94, 1.1] },
  ] as const;

  /** Starting phases, so the orbs don't all begin at their start angle. */
  const PHASES = [0.1, 0.47, 0.66, 0.34, 0.45, 0.86, 0.38, 0.61, 0.26];

  const reduceMotion = typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let field = $state<HTMLDivElement | null>(null);
  const breath = new BassBreath();
  const bands = new Float32Array(16);
  let timer: number | null = null;
  let last = 0;
  let shown = 0;

  function stopBreath() {
    if (timer !== null) window.clearInterval(timer);
    timer = null;
    breath.reset();
    shown = 0;
    if (field) field.style.transform = "";
  }

  function tick() {
    const now = performance.now();
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    // 16 bands over ~8 octaves: the first two cover ~55–110 Hz, where the kick drum sits.
    const bass = audioEngine.getSpectrumDb(bands.length, bands) ? Math.max(bands[0], bands[1]) : Number.NaN;
    const value = breath.update(bass, dt);
    if (value === shown || !field) return;
    shown = value;
    field.style.transform = value > 0 ? `scale(${(1 + BREATH_SCALE * value).toFixed(4)})` : "";
  }

  $effect(() => {
    if (!playing || reduceMotion || !field) return;
    last = performance.now();
    timer = window.setInterval(tick, 1000 / 30);
    return stopBreath;
  });

  onDestroy(stopBreath);
</script>

<div class="lamp" class:paused={!playing}>
  <div class="field" bind:this={field}>
    {#each ORBS as orb, i (i)}
      <div
        class="spot"
        style:left={`${orb.x}%`}
        style:top={`${orb.y}%`}
        style:--a={`${orb.a}deg`}
        style:--dir={orb.dir}
        style:--r1={`${orb.r[0]}vmin`}
        style:--r2={`${orb.r[1]}vmin`}
        style:--r3={`${orb.r[2]}vmin`}
        style:--r4={`${orb.r[3]}vmin`}
        style:--s1={orb.s[0]}
        style:--s2={orb.s[1]}
        style:--s3={orb.s[2]}
        style:--s4={orb.s[3]}
        style:animation-duration={`${(PERIOD_S * orb.k).toFixed(2)}s`}
        style:animation-delay={`${(-PERIOD_S * orb.k * PHASES[i]).toFixed(2)}s`}
      >
        <div class="blob" style:width={`${orb.size}vmax`} style:--c={orb.c}></div>
      </div>
    {/each}
  </div>
</div>

<style>
  .lamp {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }

  /* The breath target and the blend group for the orbs. */
  .field {
    position: absolute;
    inset: 0;
    isolation: isolate;
    transform-origin: 50% 50%;
    transition: transform 90ms linear;
  }

  /* A zero-size point that wanders: turning and moving out from its spot, so its blob draws an uneven loop. */
  .spot {
    position: absolute;
    width: 0;
    height: 0;
    will-change: transform;
    animation-name: wander;
    animation-timing-function: linear;
    animation-iteration-count: infinite;
  }

  .lamp.paused .spot {
    animation-play-state: paused;
  }

  /* Full-bodied orbs with a soft edge; `lighten` keeps overlaps from climbing towards white behind the lyrics. */
  .blob {
    position: absolute;
    aspect-ratio: 1;
    border-radius: 50%;
    translate: -50% -50%;
    opacity: 0.6;
    mix-blend-mode: lighten;
    background: radial-gradient(
      closest-side,
      var(--c) 0%,
      var(--c) 30%,
      color-mix(in srgb, var(--c) 55%, transparent) 60%,
      transparent 100%
    );
  }

  :global([data-theme="daylight"]) .blob,
  :global([data-theme="iceLight"]) .blob {
    mix-blend-mode: multiply;
    opacity: 0.32;
  }

  /* A closed loop: it ends a full turn later at its first distance and size, so `infinite` wraps without a jump. */
  @keyframes wander {
    0% {
      transform: rotate(var(--a)) translateX(var(--r1)) scale(var(--s1));
    }
    25% {
      transform: rotate(calc(var(--a) + var(--dir) * 90deg)) translateX(var(--r2)) scale(var(--s2));
    }
    50% {
      transform: rotate(calc(var(--a) + var(--dir) * 180deg)) translateX(var(--r3)) scale(var(--s3));
    }
    75% {
      transform: rotate(calc(var(--a) + var(--dir) * 270deg)) translateX(var(--r4)) scale(var(--s4));
    }
    100% {
      transform: rotate(calc(var(--a) + var(--dir) * 360deg)) translateX(var(--r1)) scale(var(--s1));
    }
  }
</style>
