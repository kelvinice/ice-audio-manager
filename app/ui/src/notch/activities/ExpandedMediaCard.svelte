<script lang="ts">
  import type { MediaAction, NowPlaying } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import AlbumArtTile from './AlbumArtTile.svelte';
  import DeviceIcon from './DeviceIcon.svelte';
  import MediaScrubber from './MediaScrubber.svelte';
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
    /** Wavy progress line (`appearance.wavyProgress`, Settings → Appearance). */
    wavyProgress?: boolean;
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
    wavyProgress = false,
    onMediaAction,
  }: Props = $props();

  const isPlaying = $derived(media.status === 'playing');
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

  <MediaScrubber
    {media}
    {notchVisible}
    {instant}
    wavy={wavyProgress}
    onSeek={(positionMs) => onMediaAction({ seek: { positionMs } })}
  />
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

  .media-card.instant .t-btn {
    transition: none;
  }
</style>
