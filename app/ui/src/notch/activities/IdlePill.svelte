<script lang="ts">
  import type { NowPlaying } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import { DEFAULT_IDLE_MODE, isMediaActive, type IdleMode } from '../../lib/notch/scheduler';
  import AlbumArtTile from './AlbumArtTile.svelte';
  import DeviceIcon from './DeviceIcon.svelte';
  import VisualizerBars from './VisualizerBars.svelte';

  interface Props {
    deviceLabel: string;
    volumePercent: number;
    muted: boolean;
    micMuted: boolean;
    media?: NowPlaying | null;
    idleMode?: IdleMode;
    preset?: SpringPreset;
    instant?: boolean;
    notchVisible?: boolean;
  }

  let {
    deviceLabel,
    volumePercent,
    muted,
    micMuted,
    media = null,
    idleMode = DEFAULT_IDLE_MODE,
    preset = 'bouncy',
    instant = false,
    notchVisible = true,
  }: Props = $props();

  const activeMedia = $derived(isMediaActive(media) ? media : null);
  const isPlaying = $derived(activeMedia?.status === 'playing');
</script>

{#if idleMode === 'hidden'}
  <div class="idle-hidden-sensor" aria-hidden="true"></div>
{:else if idleMode === 'dot'}
  <div class="idle-row dot-idle">
    {#if micMuted}
      <span class="mic-badge" title="Microphone muted" aria-label="Microphone muted">
        <DeviceIcon kind="mic-muted" size={11} />
      </span>
    {:else}
      <span class="dot" aria-hidden="true"></span>
    {/if}
    {#if activeMedia}
      <span
        class="status-glyph"
        class:playing={isPlaying}
        title={isPlaying ? 'Playing' : 'Paused'}
        aria-label={isPlaying ? 'Playing' : 'Paused'}
      >
        <DeviceIcon kind={isPlaying ? 'play' : 'pause'} size={9} />
      </span>
    {/if}
  </div>
{:else if activeMedia}
  <div class="idle-row media-idle">
    {#if micMuted}
      <span class="mic-badge" title="Microphone muted" aria-label="Microphone muted">
        <DeviceIcon kind="mic-muted" size={11} />
      </span>
    {/if}
    <AlbumArtTile art={activeMedia.art} size={18} instant={true} />
    <div class="media-meta">
      <span class="idle-label">{activeMedia.title || activeMedia.artist}</span>
      {#if activeMedia.title && activeMedia.artist}
        <span class="media-sep" aria-hidden="true">·</span>
        <span class="media-artist">{activeMedia.artist}</span>
      {/if}
    </div>
    {#if isPlaying}
      <VisualizerBars active={isPlaying && notchVisible} {preset} {instant} variant="compact" />
    {:else}
      <span class="status-glyph" title="Paused" aria-label="Paused">
        <DeviceIcon kind="pause" size={10} />
      </span>
    {/if}
  </div>
{:else}
  <div class="idle-row">
    {#if micMuted}
      <span class="mic-badge" title="Microphone muted" aria-label="Microphone muted">
        <DeviceIcon kind="mic-muted" size={11} />
      </span>
    {:else}
      <span class="dot" aria-hidden="true"></span>
    {/if}
    <span class="idle-label">{deviceLabel}</span>
    <span class="vol-mini" class:muted>{muted ? 'Mute' : `${volumePercent}%`}</span>
  </div>
{/if}

<style>
  .idle-row {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    font-size: 11px;
    font-weight: 500;
    white-space: nowrap;
  }

  .idle-row.dot-idle {
    justify-content: center;
    gap: 5px;
  }

  .idle-hidden-sensor {
    width: 100%;
    height: 100%;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-accent);
    flex-shrink: 0;
  }

  .mic-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-accent-soft);
    border: 1px solid var(--ice-danger);
    color: var(--ice-danger);
    flex-shrink: 0;
  }

  .media-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
  }

  .idle-label {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ice-fg);
  }

  .media-sep {
    color: var(--ice-fg-muted);
    flex-shrink: 0;
  }

  .media-artist {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ice-fg-muted);
    font-size: 10px;
  }

  .status-glyph {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--ice-fg-muted);
    flex-shrink: 0;
  }

  .status-glyph.playing {
    color: var(--ice-active-accent, var(--ice-accent));
  }

  .vol-mini {
    font-family: var(--ice-mono);
    font-size: 11px;
    color: var(--ice-fg-muted);
    flex-shrink: 0;
  }

  .vol-mini.muted {
    color: var(--ice-danger);
  }
</style>
