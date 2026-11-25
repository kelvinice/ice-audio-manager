<script lang="ts">
  import type { NowPlaying } from '../../lib/ipc/engine';
  import { formatTimecode, toMsNumber } from '../../lib/notch/scheduler';
  import {
    clampSeekPositionMs,
    liveMediaPositionMs,
    nextSecondTickDelayMs,
    pointerSeekMs,
    stepSeekMs,
  } from '../../lib/notch/scrubber';

  interface Props {
    media: NowPlaying;
    notchVisible: boolean;
    instant?: boolean;
    /** Edge Nook shows remaining time (`-mm:ss`) like NotchNook; Floating shows total duration. */
    showRemaining?: boolean;
    /** `large`: Edge Nook's roomier song block (5 px rail, bigger times); `regular`: the Floating card. */
    size?: 'regular' | 'large';
    onSeek: (positionMs: number) => void;
  }

  let {
    media,
    notchVisible,
    instant = false,
    showRemaining = false,
    size = 'regular',
    onSeek,
  }: Props = $props();

  const SEEK_STEP_MS = 5_000;
  const SEEK_PAGE_MS = 15_000;

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragMs = $state<number | null>(null);
  let nowEpochMs = $state(Date.now());
  let fallbackAnchorMs = $state(Date.now());
  let lastTrackKey = '';
  let lastRawPos: number | null = null;
  let lastRawUpdated: number | null = null;
  let lastStatus: NowPlaying['status'] | null = null;

  const isPlaying = $derived(media.status === 'playing');
  const durationMs = $derived(toMsNumber(media.durationMs));
  const canSeek = $derived(durationMs !== null && durationMs > 0);
  const trackKey = $derived(`${media.appId}\0${media.title}\0${media.artist}\0${media.album}`);

  $effect(() => {
    const incomingPos = toMsNumber(media.positionMs);
    const incomingUpdated = toMsNumber(media.positionUpdatedMs);
    const status = media.status;
    const key = trackKey;
    const now = Date.now();

    if (key !== lastTrackKey) {
      lastTrackKey = key;
      lastRawPos = incomingPos;
      lastRawUpdated = incomingUpdated;
      lastStatus = status;
      fallbackAnchorMs = now;
      dragMs = null;
      nowEpochMs = now;
      return;
    }

    if (incomingPos !== lastRawPos || incomingUpdated !== lastRawUpdated || status !== lastStatus) {
      lastRawPos = incomingPos;
      lastRawUpdated = incomingUpdated;
      lastStatus = status;
      if (incomingUpdated === null) {
        fallbackAnchorMs = now;
      }
      nowEpochMs = now;
    }
  });

  const liveElapsedMs = $derived(liveMediaPositionMs(media, nowEpochMs, fallbackAnchorMs));

  $effect(() => {
    if (!isPlaying || !canSeek || !notchVisible || dragMs !== null) return;
    nowEpochMs = Date.now();
    let timerId: number | null = null;
    const schedule = () => {
      const now = Date.now();
      const current = liveMediaPositionMs(media, now, fallbackAnchorMs);
      if (durationMs !== null && current >= durationMs) return;
      timerId = window.setTimeout(() => {
        nowEpochMs = Date.now();
        schedule();
      }, nextSecondTickDelayMs(current));
    };
    schedule();
    return () => {
      if (timerId !== null) window.clearTimeout(timerId);
    };
  });

  const shownElapsedMs = $derived(dragMs ?? liveElapsedMs);
  const scrubberScale = $derived(
    durationMs && durationMs > 0 ? Math.max(0, Math.min(1, shownElapsedMs / durationMs)) : 0,
  );
  const elapsedLabel = $derived(formatTimecode(shownElapsedMs));
  const rightLabel = $derived(
    showRemaining && durationMs !== null
      ? `-${formatTimecode(Math.max(0, durationMs - shownElapsedMs))}`
      : formatTimecode(durationMs),
  );

  function seekTo(positionMs: number): void {
    const clamped = clampSeekPositionMs(positionMs, durationMs);
    nowEpochMs = Date.now();
    onSeek(clamped);
  }

  function resolvePointerMs(clientX: number): number | null {
    if (!trackEl) return null;
    const rect = trackEl.getBoundingClientRect();
    return pointerSeekMs(clientX, rect.left, rect.width, durationMs);
  }

  function onScrubPointerDown(e: PointerEvent): void {
    if (e.button !== 0 || !canSeek || !trackEl) return;
    const next = resolvePointerMs(e.clientX);
    if (next === null) return;
    trackEl.setPointerCapture(e.pointerId);
    dragMs = next;
  }

  function onScrubPointerMove(e: PointerEvent): void {
    if (dragMs === null) return;
    const next = resolvePointerMs(e.clientX);
    if (next !== null) dragMs = next;
  }

  function onScrubPointerUp(e: PointerEvent): void {
    if (dragMs === null) return;
    const target = resolvePointerMs(e.clientX) ?? dragMs;
    dragMs = null;
    seekTo(target);
  }

  function onScrubPointerCancel(): void {
    dragMs = null;
  }

  function onScrubKeyDown(e: KeyboardEvent): void {
    if (!canSeek || durationMs === null) return;
    let target: number | null = null;
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp') {
      target = stepSeekMs(shownElapsedMs, SEEK_STEP_MS, durationMs);
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') {
      target = stepSeekMs(shownElapsedMs, -SEEK_STEP_MS, durationMs);
    } else if (e.key === 'PageUp') {
      target = stepSeekMs(shownElapsedMs, SEEK_PAGE_MS, durationMs);
    } else if (e.key === 'PageDown') {
      target = stepSeekMs(shownElapsedMs, -SEEK_PAGE_MS, durationMs);
    } else if (e.key === 'Home') {
      target = 0;
    } else if (e.key === 'End') {
      target = durationMs;
    }
    if (target === null) return;
    e.preventDefault();
    seekTo(target);
  }
</script>

<div class="scrubber-row" class:dragging={dragMs !== null} class:instant class:large={size === 'large'}>
  <span class="timecode">{elapsedLabel}</span>
  <div
    class="scrubber-track"
    class:interactive={canSeek}
    class:dragging={dragMs !== null}
    bind:this={trackEl}
    role="slider"
    tabindex={canSeek ? 0 : -1}
    aria-label="Seek track progress"
    aria-disabled={!canSeek}
    aria-valuemin={0}
    aria-valuemax={durationMs ?? 0}
    aria-valuenow={shownElapsedMs}
    aria-valuetext={`${elapsedLabel} of ${formatTimecode(durationMs)}`}
    onpointerdown={onScrubPointerDown}
    onpointermove={onScrubPointerMove}
    onpointerup={onScrubPointerUp}
    onpointercancel={onScrubPointerCancel}
    onkeydown={onScrubKeyDown}
  >
    <div class="scrubber-rail">
      <div class="scrubber-fill" style:transform={`scaleX(${scrubberScale.toFixed(4)})`}></div>
    </div>
    {#if canSeek}
      <div class="scrubber-thumb" style:left={`${(scrubberScale * 100).toFixed(2)}%`}></div>
    {/if}
  </div>
  <span class="timecode end">{rightLabel}</span>
</div>

<style>
  .scrubber-row {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 12px;
  }

  .scrubber-track {
    position: relative;
    flex: 1;
    height: 18px;
    display: flex;
    align-items: center;
    touch-action: none;
    outline: none;
  }

  .scrubber-track.interactive {
    cursor: pointer;
  }

  /* D-28 B1: the seek bar is the card's one accent bar and thicker than the volume rail (3 px, neutral). */
  .scrubber-rail {
    width: 100%;
    height: 4px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
    transition: height 120ms ease;
  }

  .scrubber-track.interactive:hover .scrubber-rail,
  .scrubber-track.dragging .scrubber-rail,
  .scrubber-track.interactive:focus-visible .scrubber-rail {
    height: 6px;
  }

  .scrubber-fill {
    width: 100%;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    transform-origin: left center;
    background: var(--ice-active-accent, var(--ice-accent));
  }

  .scrubber-thumb {
    position: absolute;
    top: 50%;
    width: 9px;
    height: 9px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-play-bg);
    box-shadow: var(--ice-shadow);
    transform: translate(-50%, -50%) scale(0);
    transition: transform 120ms ease;
    pointer-events: none;
  }

  .scrubber-track.interactive:hover .scrubber-thumb,
  .scrubber-track.dragging .scrubber-thumb,
  .scrubber-track.interactive:focus-visible .scrubber-thumb {
    transform: translate(-50%, -50%) scale(1);
  }

  .scrubber-row.dragging .timecode:first-child {
    color: var(--ice-fg);
  }

  .scrubber-row.instant .scrubber-rail,
  .scrubber-row.instant .scrubber-thumb {
    transition: none;
  }

  .timecode {
    min-width: 28px;
    font-family: var(--ice-mono);
    font-size: 9.5px;
    color: var(--ice-fg-muted);
    flex-shrink: 0;
  }

  .timecode.end {
    text-align: right;
  }

  /* Still thicker than the 3 px neutral volume rail (D-28 B1). */
  .scrubber-row.large {
    gap: 10px;
    height: 16px;
  }

  .large .scrubber-track {
    height: 22px;
  }

  .large .scrubber-rail {
    height: 5px;
  }

  .large .scrubber-track.interactive:hover .scrubber-rail,
  .large .scrubber-track.dragging .scrubber-rail,
  .large .scrubber-track.interactive:focus-visible .scrubber-rail {
    height: 7px;
  }

  .large .scrubber-thumb {
    width: 12px;
    height: 12px;
  }

  .large .timecode {
    min-width: 34px;
    font-size: 10.5px;
  }
</style>
