<script lang="ts">
  // Edge Nook song block (D-28, owner 2026-11-25): a large cover beside title/artist with the transport under them, then
  // the full-width seek bar with both times: the card's only accent bar, so it can't be mistaken for the volume in the
  // output strip. Owner, 2026-11-25: fill the card (bigger cover, transport and seek bar; the title gets the full row).
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

  /** Cover edge; the meta column (title, artist, transport) is exactly this tall. */
  const COVER_PX = 84;

  const isPlaying = $derived(media.status === 'playing');
</script>

<div class="media">
  <div class="head">
    <div class="cover">
      <AlbumArtTile art={media.art} size={COVER_PX} radius="var(--ice-radius-md)" {instant} />
    </div>
    <div class="meta">
      <div class="text">
        <div class="title-row">
          <span class="title" title={media.title}>{media.title || 'Unknown track'}</span>
          {#if isPlaying}
            <VisualizerBars active={isPlaying && notchVisible} {preset} {instant} variant="expanded" />
          {/if}
        </div>
        <span class="artist" title={media.artist}>{media.artist || 'Unknown artist'}</span>
      </div>
      <div class="transport" role="group" aria-label="Media transport">
        <button class="t-btn" type="button" aria-label="Previous track" onclick={() => onMediaAction('previous')}>
          <DeviceIcon kind="prev" size={16} />
        </button>
        <button
          class="t-btn t-play"
          type="button"
          aria-label={isPlaying ? 'Pause' : 'Play'}
          onclick={() => onMediaAction('playPause')}
        >
          <DeviceIcon kind={isPlaying ? 'pause' : 'play'} size={18} />
        </button>
        <button class="t-btn" type="button" aria-label="Next track" onclick={() => onMediaAction('next')}>
          <DeviceIcon kind="next" size={16} />
        </button>
      </div>
    </div>
  </div>

  <MediaScrubber
    {media}
    {notchVisible}
    {instant}
    showRemaining
    size="large"
    onSeek={(positionMs) => onMediaAction({ seek: { positionMs } })}
  />
</div>

<style>
  /* Owner, 2026-11-26: the transport sat right under the artist and the seek bar right under the transport; the head
     is 6 px taller (a clear gap under the artist) and the seek bar 2 px further down. Fits the 156 px card body:
     84 + 12 + 16 (song) + 6 + 35 (output strip) = 153. */
  .media {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 14px;
    height: 84px;
  }

  .cover {
    flex-shrink: 0;
    border-radius: var(--ice-radius-md);
    box-shadow: var(--ice-drop-shadow);
  }

  /* Title and artist on top, the transport along the bottom edge of the cover. */
  .meta {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding-top: 1px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .title {
    flex: 1;
    min-width: 0;
    font-size: 15px;
    font-weight: 650;
    letter-spacing: -0.01em;
    line-height: 20px;
    color: var(--ice-fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .artist {
    font-size: 12px;
    font-weight: 500;
    line-height: 16px;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The negative margin lines the "previous" glyph up with the title text instead of its hover circle. */
  .transport {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: -9px;
  }

  .t-btn {
    appearance: none;
    width: 34px;
    height: 34px;
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
    width: 40px;
    height: 40px;
    background: var(--ice-play-bg);
    color: var(--ice-play-fg);
  }

  .t-play:hover {
    background: var(--ice-play-bg);
    filter: brightness(1.08);
  }
</style>
