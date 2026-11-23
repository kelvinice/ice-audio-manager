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
    expanded: boolean;
    onMediaAction: (action: MediaAction) => void;
    onToggleExpand: () => void;
  }

  let {
    media,
    spatialBadge,
    instant,
    notchVisible,
    preset,
    expanded,
    onMediaAction,
    onToggleExpand,
  }: Props = $props();

  const isPlaying = $derived(media.status === 'playing');
  const scrubberScale = $derived(Math.max(0, Math.min(1, mediaProgressPercent(media) / 100)));
  const elapsedLabel = $derived(formatTimecode(media.positionMs));
  const durationLabel = $derived(formatTimecode(media.durationMs));
</script>

<div class="media-card">
  <div class="media-top">
    <AlbumArtTile art={media.art} size={34} {instant} />
    <div class="media-info">
      <span class="media-title">{media.title || 'Unknown track'}</span>
      <span class="media-sub">
        {media.artist || 'Unknown artist'}
        {#if spatialBadge}
          <span class="spatial-inline">· {spatialBadge}</span>
        {/if}
      </span>
    </div>
    {#if isPlaying}
      <VisualizerBars active={isPlaying && notchVisible} {preset} {instant} variant="expanded" />
    {/if}
    <div class="media-controls" role="group" aria-label="Media transport">
      <button
        class="chip icon-chip"
        type="button"
        aria-label="Previous track"
        onclick={() => onMediaAction('previous')}
      >
        <DeviceIcon kind="prev" size={11} />
      </button>
      <button
        class="chip icon-chip media-play"
        type="button"
        aria-label={isPlaying ? 'Pause' : 'Play'}
        onclick={() => onMediaAction('playPause')}
      >
        <DeviceIcon kind={isPlaying ? 'pause' : 'play'} size={12} />
      </button>
      <button
        class="chip icon-chip"
        type="button"
        aria-label="Next track"
        onclick={() => onMediaAction('next')}
      >
        <DeviceIcon kind="next" size={11} />
      </button>
    </div>
    <button class="chip pin-chip" class:primary={expanded} type="button" onclick={onToggleExpand}>
      <DeviceIcon kind="pin" size={10} />
      <span>{expanded ? 'Pinned' : 'Pin'}</span>
    </button>
  </div>

  <div class="scrubber-row" aria-label="Track progress">
    <span class="timecode">{elapsedLabel}</span>
    <div class="scrubber-track">
      <div class="scrubber-fill" style:transform={`scaleX(${scrubberScale.toFixed(4)})`}></div>
    </div>
    <span class="timecode">{durationLabel}</span>
  </div>
</div>

<style>
  .media-card {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .media-top,
  .scrubber-row,
  .media-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .media-controls {
    gap: 3px;
  }

  .media-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 1px;
    white-space: nowrap;
  }

  .media-title {
    font-size: 12.5px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 1.2;
    color: var(--ice-fg);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .media-sub {
    font-size: 10.5px;
    font-weight: 500;
    line-height: 1.2;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .spatial-inline {
    color: var(--ice-accent-strong);
  }

  .scrubber-track {
    flex: 1;
    height: 3.5px;
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

  .timecode:last-child {
    text-align: right;
  }

  .chip {
    appearance: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    border: 1px solid var(--ice-control-border);
    background: var(--ice-control-bg);
    color: var(--ice-fg);
    border-radius: var(--ice-radius-pill);
    padding: 3px 8px;
    font-size: 10px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .chip:hover {
    background: var(--ice-control-bg-hover);
  }

  .icon-chip {
    width: 22px;
    height: 22px;
    padding: 0;
  }

  .media-play {
    width: 25px;
    height: 25px;
    border-color: transparent;
    background: var(--ice-play-bg);
    color: var(--ice-play-fg);
  }

  .media-play:hover {
    background: var(--ice-play-bg);
    filter: brightness(1.06);
  }

  .pin-chip {
    gap: 3px;
    padding: 3px 6px;
  }

  .chip.primary {
    border-color: var(--ice-accent);
    background: var(--ice-control-bg-active);
    color: var(--ice-fg);
  }
</style>
