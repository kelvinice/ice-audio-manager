<script lang="ts">
  // Edge Nook media block: a larger (64 px) cover with title, artist and the scrubber beside it, then one transport row with
  // an accent play button. Keeping the scrubber in the meta column frees the height the bigger cover needs.
  import type { MediaAction, NowPlaying } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import AlbumArtTile from './AlbumArtTile.svelte';
  import DeviceIcon from './DeviceIcon.svelte';
  import MediaScrubber from './MediaScrubber.svelte';
  import VisualizerBars from './VisualizerBars.svelte';

  interface Props {
    media: NowPlaying;
    notchVisible: boolean;
    instant: boolean;
    preset: SpringPreset;
    onMediaAction: (action: MediaAction) => void;
  }

  let { media, notchVisible, instant, preset, onMediaAction }: Props = $props();

  const isPlaying = $derived(media.status === 'playing');
</script>

<div class="media">
  <div class="head">
    <div class="cover">
      <AlbumArtTile art={media.art} size={64} radius="var(--ice-radius-md)" {instant} />
    </div>
    <div class="meta">
      <div class="title-row">
        <span class="title" title={media.title}>{media.title || 'Unknown track'}</span>
        {#if isPlaying}
          <VisualizerBars active={isPlaying && notchVisible} {preset} {instant} variant="expanded" />
        {/if}
      </div>
      <span class="artist" title={media.artist}>{media.artist || 'Unknown artist'}</span>
      <div class="scrub">
        <MediaScrubber
          {media}
          {notchVisible}
          {instant}
          showRemaining
          onSeek={(positionMs) => onMediaAction({ seek: { positionMs } })}
        />
      </div>
    </div>
  </div>

  <div class="transport" role="group" aria-label="Media transport">
    <button class="t-btn" type="button" aria-label="Previous track" onclick={() => onMediaAction('previous')}>
      <DeviceIcon kind="prev" size={13} />
    </button>
    <button
      class="t-btn t-play"
      type="button"
      aria-label={isPlaying ? 'Pause' : 'Play'}
      onclick={() => onMediaAction('playPause')}
    >
      <DeviceIcon kind={isPlaying ? 'pause' : 'play'} size={14} />
    </button>
    <button class="t-btn" type="button" aria-label="Next track" onclick={() => onMediaAction('next')}>
      <DeviceIcon kind="next" size={13} />
    </button>
  </div>
</div>

<style>
  .media {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 64px;
  }

  .cover {
    flex-shrink: 0;
    border-radius: var(--ice-radius-md);
    box-shadow: var(--ice-drop-shadow);
  }

  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .title {
    flex: 1;
    min-width: 0;
    font-size: 14px;
    font-weight: 650;
    letter-spacing: -0.01em;
    line-height: 19px;
    color: var(--ice-fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .artist {
    font-size: 11.5px;
    font-weight: 500;
    line-height: 16px;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scrub {
    margin-top: 5px;
  }

  /* Centred under the title/scrubber column (cover 64 + gap 12), so the controls line up with the scrubber. */
  .transport {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 18px;
    height: 32px;
    padding-left: 76px;
  }

  .t-btn {
    appearance: none;
    width: 28px;
    height: 28px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--ice-radius-pill);
    background: transparent;
    color: var(--ice-fg);
    cursor: pointer;
    transition:
      background-color 120ms ease,
      transform 120ms ease;
  }

  .t-btn:hover {
    background: var(--ice-control-bg-hover);
  }

  .t-btn:active {
    transform: scale(0.92);
  }

  .t-btn:focus-visible {
    outline: 2px solid var(--ice-accent);
    outline-offset: 1px;
  }

  .t-play {
    width: 32px;
    height: 32px;
    background: var(--ice-play-bg);
    color: var(--ice-play-fg);
  }

  .t-play:hover {
    background: var(--ice-play-bg);
    filter: brightness(1.08);
  }
</style>
