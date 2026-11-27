<script lang="ts">
  // Seek bar shared by the player bar and Now Playing, built like Ice's notch scrubber (MediaScrubber / WavyProgress):
  // one pointer-driven slider; a flat rail that thickens on hover with a thumb that appears on hover or drag; or, with
  // `wavy`, a travelling wave as thick as the flat rest, round-capped, tapering into it and flattening on pause, with a
  // round dot at the play head.
  import { untrack } from "svelte";
  import { formatTimecode, pointerSeekMs, SEEK_WAVE, taperedWavePath, type SeekSize } from "../lib/wave";

  interface Props {
    positionMs: number;
    durationMs: number;
    playing: boolean;
    wavy: boolean;
    size?: SeekSize;
    /** Times sit left and right of the bar (`inline`) or under its ends (`below`). */
    times?: "inline" | "below";
    onSeek: (ms: number) => void;
  }

  let { positionMs, durationMs, playing, wavy, size = "regular", times = "inline", onSeek }: Props = $props();

  const STEP_MS = 5_000;
  const PAGE_MS = 15_000;
  /** Wave loop cap and how fast it swells on play / settles on pause. */
  const FRAME_MS = 1000 / 30;
  const SWELL_S = 0.14;

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragMs = $state<number | null>(null);
  let width = $state(0);
  let phase = $state(0);
  let ampScale = $state(0);

  let canSeek = $derived(durationMs > 0);
  let shownMs = $derived(dragMs ?? positionMs);
  let progress = $derived(canSeek ? Math.max(0, Math.min(1, shownMs / durationMs)) : 0);

  let wave = $derived(SEEK_WAVE[size]);
  let waveHeight = $derived(Math.ceil(wave.amp * 2 + wave.stroke + 2));
  let mid = $derived(waveHeight / 2);
  let cap = $derived(wave.stroke / 2);
  let headX = $derived(cap + progress * Math.max(0, width - wave.stroke));
  let playedD = $derived(
    taperedWavePath(cap, headX, mid, { wavelength: wave.wavelength, amp: wave.amp, phase, ampScale }),
  );

  const reduceMotion =
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // Bounded loop: only while wavy and playing, or while the wave is still flattening after a pause.
  $effect(() => {
    const target = wavy && playing ? 1 : 0;
    if (!wavy || reduceMotion) {
      ampScale = reduceMotion ? target : 0;
      return;
    }
    let raf = 0;
    let last = performance.now();
    let lastPaint = 0;
    // Advance locally every frame; publish (and so redraw) at most every FRAME_MS.
    let localPhase = untrack(() => phase);
    let localAmp = untrack(() => ampScale);
    const tick = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      if (playing) localPhase = (localPhase + wave.speed * dt) % wave.wavelength;
      localAmp += (target - localAmp) * (1 - Math.exp(-dt / SWELL_S));
      if (!playing && Math.abs(target - localAmp) < 0.01) {
        ampScale = target;
        return;
      }
      if (now - lastPaint >= FRAME_MS) {
        lastPaint = now;
        phase = localPhase;
        ampScale = localAmp;
      }
      // WebView2 pauses animation frames while the window is hidden, so this idles there by itself.
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  function msAt(clientX: number): number | null {
    if (!trackEl || !canSeek) return null;
    const rect = trackEl.getBoundingClientRect();
    return pointerSeekMs(clientX, rect.left, rect.width, durationMs);
  }

  function onDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const ms = msAt(e.clientX);
    if (ms === null || !trackEl) return;
    trackEl.setPointerCapture(e.pointerId);
    dragMs = ms;
  }

  function onMove(e: PointerEvent) {
    if (dragMs === null) return;
    const ms = msAt(e.clientX);
    if (ms !== null) dragMs = ms;
  }

  function onUp(e: PointerEvent) {
    if (dragMs === null) return;
    const ms = msAt(e.clientX) ?? dragMs;
    dragMs = null;
    onSeek(ms);
  }

  function onKey(e: KeyboardEvent) {
    if (!canSeek) return;
    const steps: Record<string, number> = {
      ArrowRight: STEP_MS,
      ArrowUp: STEP_MS,
      ArrowLeft: -STEP_MS,
      ArrowDown: -STEP_MS,
      PageUp: PAGE_MS,
      PageDown: -PAGE_MS,
    };
    let target: number | null = null;
    if (e.key in steps) target = shownMs + steps[e.key];
    else if (e.key === "Home") target = 0;
    else if (e.key === "End") target = durationMs;
    if (target === null) return;
    e.preventDefault();
    onSeek(Math.max(0, Math.min(durationMs, target)));
  }
</script>

<div class="seek" class:below={times === "below"} class:large={size === "large"} class:dragging={dragMs !== null}>
  <span class="time start">{formatTimecode(shownMs)}</span>
  <div
    class="track"
    class:interactive={canSeek}
    bind:this={trackEl}
    bind:clientWidth={width}
    role="slider"
    tabindex={canSeek ? 0 : -1}
    aria-label="Seek"
    aria-disabled={!canSeek}
    aria-valuemin={0}
    aria-valuemax={durationMs}
    aria-valuenow={shownMs}
    aria-valuetext={`${formatTimecode(shownMs)} of ${formatTimecode(durationMs)}`}
    onpointerdown={onDown}
    onpointermove={onMove}
    onpointerup={onUp}
    onpointercancel={() => (dragMs = null)}
    onkeydown={onKey}
  >
    {#if wavy && width > 0}
      <svg class="wave" {width} height={waveHeight} viewBox={`0 0 ${width} ${waveHeight}`} aria-hidden="true">
        {#if headX < width - cap}
          <line class="rest" x1={headX} y1={mid} x2={width - cap} y2={mid} stroke-width={wave.stroke} stroke-linecap="round" />
        {/if}
        <path class="played" d={playedD} fill="none" stroke-width={wave.stroke} stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    {:else}
      <div class="rail">
        <div class="fill" style:transform={`scaleX(${progress.toFixed(4)})`}></div>
      </div>
    {/if}
    {#if canSeek}
      <span class="thumb" class:dot={wavy} style:left={`${(progress * 100).toFixed(2)}%`}></span>
    {/if}
  </div>
  <span class="time end">{formatTimecode(durationMs)}</span>
</div>

<style>
  .seek {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    width: 100%;
  }

  .seek.below {
    grid-template-columns: 1fr 1fr;
    row-gap: 4px;
  }

  .seek.below .track {
    grid-column: 1 / -1;
    grid-row: 1;
  }

  .seek.below .time {
    grid-row: 2;
  }

  .seek.below .end {
    text-align: right;
  }

  .track {
    position: relative;
    height: 18px;
    display: flex;
    align-items: center;
    touch-action: none;
    outline: none;
  }

  .large .track {
    height: 22px;
  }

  .track.interactive {
    cursor: pointer;
  }

  .track:focus-visible {
    outline: 2px solid var(--p-accent);
    outline-offset: 4px;
    border-radius: 4px;
  }

  .rail {
    width: 100%;
    height: 4px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--p-border-strong);
    transition: height 0.12s ease;
  }

  .large .rail {
    height: 5px;
  }

  .track.interactive:hover .rail,
  .dragging .rail,
  .track:focus-visible .rail {
    height: 6px;
  }

  .large .track.interactive:hover .rail,
  .large.dragging .rail,
  .large .track:focus-visible .rail {
    height: 7px;
  }

  .fill {
    width: 100%;
    height: 100%;
    border-radius: 999px;
    transform-origin: left center;
    background: var(--p-accent);
  }

  .track.interactive:hover .fill,
  .dragging .fill {
    background: var(--p-accent-bright);
  }

  .wave {
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    overflow: visible;
    pointer-events: none;
  }

  .rest {
    stroke: var(--p-border-strong);
  }

  .played {
    stroke: var(--p-accent);
  }

  .thumb {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    border-radius: 999px;
    background: var(--p-text);
    box-shadow: 0 1px 6px rgb(0 0 0 / 0.4);
    transform: translate(-50%, -50%) scale(0);
    transition: transform 0.12s ease;
    pointer-events: none;
  }

  /* With the wave, a round dot in the accent marks the head and stays visible; it grows a little on hover or drag. */
  .thumb.dot {
    background: var(--p-accent-bright);
    transform: translate(-50%, -50%) scale(0.85);
  }

  .large .thumb {
    width: 14px;
    height: 14px;
  }

  .track.interactive:hover .thumb,
  .dragging .thumb,
  .track:focus-visible .thumb {
    transform: translate(-50%, -50%) scale(1);
  }

  .track.interactive:hover .thumb.dot,
  .dragging .thumb.dot,
  .track:focus-visible .thumb.dot {
    transform: translate(-50%, -50%) scale(1.15);
  }

  .time {
    min-width: 36px;
    font-size: 11.5px;
    color: var(--p-text-dim);
    font-variant-numeric: tabular-nums;
  }

  .large .time {
    font-size: 12px;
  }

  .end {
    text-align: right;
  }

  .dragging .start {
    color: var(--p-text);
  }
</style>
