<script lang="ts">
  import type { MediaAction, NowPlaying } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import { formatTimecode, toMsNumber } from '../../lib/notch/scheduler';
  import {
    clampSeekPositionMs,
    liveMediaPositionMs,
    nextSecondTickDelayMs,
    pointerSeekMs,
    stepSeekMs,
  } from '../../lib/notch/scrubber';
  import AlbumArtTile from './AlbumArtTile.svelte';
  import DeviceIcon from './DeviceIcon.svelte';
  import VisualizerBars from './VisualizerBars.svelte';

  interface Props {
    media: NowPlaying;
    spatialBadge: string | null;
    instant: boolean;
    notchVisible: boolean;
    preset: SpringPreset;
    /** Lyrics drawer toggle (shown when lyrics are enabled for the card). */
    showLyricsToggle?: boolean;
    lyricsOpen?: boolean;
    lyricsAvailable?: boolean;
    onToggleLyrics?: () => void;
    onMediaAction: (action: MediaAction) => void;
  }

  let {
    media,
    spatialBadge,
    instant,
    notchVisible,
    preset,
    showLyricsToggle = false,
    lyricsOpen = false,
    lyricsAvailable = false,
    onToggleLyrics = () => {},
    onMediaAction,
  }: Props = $props();

  const SEEK_STEP_MS = 5_000;
  const SEEK_PAGE_MS = 15_000;
  const SEEK_HOLD_MS = 1_500;

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragMs = $state<number | null>(null);
  let nowEpochMs = $state(Date.now());
  let fallbackAnchorMs = $state(Date.now());
  let pendingSeek = $state<{ targetMs: number; setAtEpochMs: number; untilEpochMs: number } | null>(null);
  let lastTrackKey = '';
  let lastRawPos: number | null = null;
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
      lastStatus = status;
      fallbackAnchorMs = now;
      pendingSeek = null;
      dragMs = null;
      nowEpochMs = now;
      return;
    }

    if (incomingPos !== lastRawPos || status !== lastStatus) {
      lastRawPos = incomingPos;
      lastStatus = status;
      if (incomingUpdated === null) {
        fallbackAnchorMs = now;
      }
      if (pendingSeek) {
        const expected =
          pendingSeek.targetMs + (status === 'playing' ? Math.max(0, now - pendingSeek.setAtEpochMs) : 0);
        const reported = liveMediaPositionMs(media, now, fallbackAnchorMs);
        if (now >= pendingSeek.untilEpochMs || Math.abs(reported - expected) <= 3500) {
          pendingSeek = null;
        }
      }
      nowEpochMs = now;
    }
  });

  const liveElapsedMs = $derived.by(() => {
    if (pendingSeek && nowEpochMs < pendingSeek.untilEpochMs) {
      const raw = pendingSeek.targetMs + (isPlaying ? Math.max(0, nowEpochMs - pendingSeek.setAtEpochMs) : 0);
      return canSeek ? clampSeekPositionMs(raw, durationMs) : Math.max(0, Math.round(raw));
    }
    return liveMediaPositionMs(media, nowEpochMs, fallbackAnchorMs);
  });

  $effect(() => {
    if (!isPlaying || !canSeek || !notchVisible || dragMs !== null) return;
    nowEpochMs = Date.now();
    let timerId: number | null = null;
    const schedule = () => {
      const now = Date.now();
      const activePending = pendingSeek && now < pendingSeek.untilEpochMs ? pendingSeek : null;
      const current = activePending
        ? clampSeekPositionMs(activePending.targetMs + Math.max(0, now - activePending.setAtEpochMs), durationMs)
        : liveMediaPositionMs(media, now, fallbackAnchorMs);
      if (durationMs !== null && current >= durationMs) return;
      timerId = window.setTimeout(() => {
        nowEpochMs = Date.now();
        if (pendingSeek && nowEpochMs >= pendingSeek.untilEpochMs) {
          pendingSeek = null;
        }
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
  const durationLabel = $derived(formatTimecode(durationMs));

  function seekTo(positionMs: number): void {
    const clamped = clampSeekPositionMs(positionMs, durationMs);
    const now = Date.now();
    nowEpochMs = now;
    pendingSeek = { targetMs: clamped, setAtEpochMs: now, untilEpochMs: now + SEEK_HOLD_MS };
    onMediaAction({ seek: { positionMs: clamped } });
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

<div class="media-card" class:instant>
  <div class="media-head">
    <AlbumArtTile art={media.art} size={58} {instant} />
    <div class="media-info">
      <div class="title-row">
        <span class="media-title">{media.title || 'Unknown track'}</span>
        {#if isPlaying}
          <VisualizerBars active={isPlaying && notchVisible} {preset} {instant} variant="expanded" />
        {/if}
      </div>
      <span class="media-sub">
        {media.artist || 'Unknown artist'}
        {#if spatialBadge}
          <span class="spatial-inline">· {spatialBadge}</span>
        {/if}
      </span>
      <div class="transport" role="group" aria-label="Media transport">
        <button class="t-btn" type="button" aria-label="Previous track" onclick={() => onMediaAction('previous')}>
          <DeviceIcon kind="prev" size={12} />
        </button>
        <button
          class="t-btn t-play"
          type="button"
          aria-label={isPlaying ? 'Pause' : 'Play'}
          onclick={() => onMediaAction('playPause')}
        >
          <DeviceIcon kind={isPlaying ? 'pause' : 'play'} size={12} />
        </button>
        <button class="t-btn" type="button" aria-label="Next track" onclick={() => onMediaAction('next')}>
          <DeviceIcon kind="next" size={12} />
        </button>
        <!-- Lyrics belong to the song, so the toggle sits with its controls (the side rail only fits five buttons). -->
        {#if showLyricsToggle}
          <button
            class="t-btn t-lyrics"
            class:on={lyricsOpen && lyricsAvailable}
            type="button"
            title={lyricsAvailable ? (lyricsOpen ? 'Hide lyrics' : 'Show lyrics') : 'No lyrics for this song'}
            aria-label={lyricsOpen ? 'Hide lyrics' : 'Show lyrics'}
            aria-pressed={lyricsOpen && lyricsAvailable}
            disabled={!lyricsAvailable}
            onclick={onToggleLyrics}
          >
            <DeviceIcon kind="lyrics" size={13} />
          </button>
        {/if}
      </div>
    </div>
  </div>

  <div class="scrubber-row" class:dragging={dragMs !== null}>
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
      aria-valuetext={`${elapsedLabel} of ${durationLabel}`}
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
    <span class="timecode end">{durationLabel}</span>
  </div>
</div>

<style>
  .media-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .media-head {
    display: flex;
    align-items: stretch;
    gap: 12px;
    height: 58px;
  }

  .media-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    white-space: nowrap;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 16px;
  }

  .media-title {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 16px;
    color: var(--ice-fg);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .media-sub {
    margin-top: 1px;
    font-size: 11px;
    font-weight: 500;
    line-height: 14px;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .spatial-inline {
    color: var(--ice-accent-strong);
  }

  .transport {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: -4px;
  }

  .t-btn {
    appearance: none;
    width: 24px;
    height: 24px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--ice-radius-pill);
    background: transparent;
    color: var(--ice-fg);
    cursor: pointer;
    transition: background-color 120ms ease, transform 120ms ease;
  }

  .t-btn:hover {
    background: var(--ice-control-bg-hover);
  }

  .t-btn:active {
    transform: scale(0.92);
  }

  .t-play {
    width: 26px;
    height: 26px;
    background: var(--ice-play-bg);
    color: var(--ice-play-fg);
  }

  .t-play:hover {
    background: var(--ice-play-bg);
    filter: brightness(1.08);
  }

  /* Pushed to the right end of the row, away from prev/play/next. */
  .t-lyrics {
    margin-left: auto;
    color: var(--ice-fg-muted);
  }

  .t-lyrics.on {
    color: var(--ice-accent-strong);
    background: var(--ice-control-bg-active);
  }

  .t-lyrics:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .t-lyrics:disabled:hover {
    background: transparent;
  }

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

  .scrubber-rail {
    width: 100%;
    height: 3px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
    transition: height 120ms ease;
  }

  .scrubber-track.interactive:hover .scrubber-rail,
  .scrubber-track.dragging .scrubber-rail,
  .scrubber-track.interactive:focus-visible .scrubber-rail {
    height: 5px;
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

  .media-card.instant .scrubber-rail,
  .media-card.instant .scrubber-thumb,
  .media-card.instant .t-btn {
    transition: none;
  }

  .timecode {
    min-width: 26px;
    font-family: var(--ice-mono);
    font-size: 9.5px;
    color: var(--ice-fg-muted);
    flex-shrink: 0;
  }

  .timecode.end {
    text-align: right;
  }
</style>
