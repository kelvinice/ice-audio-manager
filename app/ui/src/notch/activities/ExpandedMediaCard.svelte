<script lang="ts">
  import type { MediaAction, NowPlaying } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import { formatTimecode, mediaProgressPercent } from '../../lib/notch/scheduler';
  import AlbumArtTile from './AlbumArtTile.svelte';
  import DeviceIcon from './DeviceIcon.svelte';
  import VisualizerBars from './VisualizerBars.svelte';

  interface Props {
    media: NowPlaying;
    spatialBadge: string | null;
    instant: boolean;
    notchVisible: boolean;
    preset: SpringPreset;
    onMediaAction: (action: MediaAction) => void;
  }

  let { media, spatialBadge, instant, notchVisible, preset, onMediaAction }: Props = $props();

  const isPlaying = $derived(media.status === 'playing');
  const scrubberScale = $derived(Math.max(0, Math.min(1, mediaProgressPercent(media) / 100)));
  const elapsedLabel = $derived(formatTimecode(media.positionMs));
  const durationLabel = $derived(formatTimecode(media.durationMs));
</script>

<div class="media-card">
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
      </div>
    </div>
  </div>

  <div class="scrubber-row" aria-label="Track progress">
    <span class="timecode">{elapsedLabel}</span>
    <div class="scrubber-track">
      <div class="scrubber-fill" style:transform={`scaleX(${scrubberScale.toFixed(4)})`}></div>
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

  .scrubber-row {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 12px;
  }

  .scrubber-track {
    flex: 1;
    height: 3px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
  }

  .scrubber-fill {
    width: 100%;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    transform-origin: left center;
    background: var(--ice-active-accent, var(--ice-accent));
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
