<script lang="ts">
  import type { DeviceSummary, LyricsState, MediaAction, NowPlaying, SettingsView } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import { hasAnyLyrics } from '../../lib/notch/lyrics';
  import { isMediaActive } from '../../lib/notch/scheduler';
  import AlbumArtTile from './AlbumArtTile.svelte';
  import DeviceIcon from './DeviceIcon.svelte';
  import ExpandedDeviceRow from './ExpandedDeviceRow.svelte';
  import ExpandedEmptyCard from './ExpandedEmptyCard.svelte';
  import ExpandedVolumeRow from './ExpandedVolumeRow.svelte';
  import LyricsExtension from './LyricsExtension.svelte';
  import MediaScrubber from './MediaScrubber.svelte';
  import MusicServicesWidget from './MusicServicesWidget.svelte';
  import VisualizerBars from './VisualizerBars.svelte';

  interface Props {
    playback: DeviceSummary[];
    settings: SettingsView | null;
    disconnected: DeviceSummary[];
    musicServices: MusicServiceItem[];
    activeDeviceName: string;
    spatialBadge: string | null;
    volumePercent: number;
    muted: boolean;
    barPercent: number;
    micMuted: boolean;
    media?: NowPlaying | null;
    lyrics?: LyricsState;
    showLyrics?: boolean;
    lyricsDrawerOpen?: boolean;
    instant?: boolean;
    notchVisible?: boolean;
    preset: SpringPreset;
    expanded: boolean;
    onToggleExpand: () => void;
    onToggleMute: () => void;
    onSetVolume: (percent: number) => void;
    onSwitchDevice: (deviceId: string) => void;
    onToggleMicMute: () => void;
    onToggleLyrics?: () => void;
    onMediaAction?: (action: MediaAction) => void;
    onOpenMusicService?: (service: MusicServiceId) => void;
    onOpenSettings: () => void;
  }

  let {
    playback,
    settings,
    disconnected,
    musicServices,
    activeDeviceName,
    spatialBadge,
    volumePercent,
    muted,
    barPercent,
    micMuted,
    media = null,
    lyrics = { status: 'disabled', trackKey: null, document: null },
    showLyrics = false,
    lyricsDrawerOpen = true,
    instant = false,
    notchVisible = true,
    preset,
    expanded,
    onToggleExpand,
    onToggleMute,
    onSetVolume,
    onSwitchDevice,
    onToggleMicMute,
    onToggleLyrics = () => {},
    onMediaAction = () => {},
    onOpenMusicService = () => {},
    onOpenSettings,
  }: Props = $props();

  const activeMedia = $derived(isMediaActive(media) ? media : null);
  const isPlaying = $derived(activeMedia?.status === 'playing');
  const lyricsConfig = $derived(settings?.lyrics ?? null);
  const lyricsAvailable = $derived(hasAnyLyrics(lyrics));
  const showLyricsToggle = $derived(Boolean(activeMedia && lyricsConfig?.enabled && lyricsConfig?.showOnCard));
</script>

<div class="nook-shell" class:stagger={!instant}>
  <header class="nook-top">
    <div class="nook-status">
      <span class="device-chip" title={activeDeviceName}>{activeDeviceName}</span>
      {#if spatialBadge}
        <span class="spatial-badge">{spatialBadge}</span>
      {/if}
    </div>
    <nav class="top-actions" aria-label="Edge Nook quick actions">
      {#if activeMedia && musicServices.length > 0}
        <div class="svc-strip">
          <MusicServicesWidget services={musicServices} variant="rail" {onOpenService} />
        </div>
      {/if}
      {#if showLyricsToggle}
        <button
          class="top-btn"
          class:on={lyricsDrawerOpen && lyricsAvailable}
          type="button"
          title={lyricsAvailable ? (lyricsDrawerOpen ? 'Hide lyrics' : 'Show lyrics') : 'No lyrics for this song'}
          aria-label={lyricsDrawerOpen ? 'Hide lyrics' : 'Show lyrics'}
          aria-pressed={lyricsDrawerOpen && lyricsAvailable}
          disabled={!lyricsAvailable}
          onclick={onToggleLyrics}
        >
          <DeviceIcon kind="lyrics" size={12} />
        </button>
      {/if}
      <button
        class="top-btn"
        class:danger={micMuted}
        type="button"
        title={micMuted ? 'Microphone is off: turn on' : 'Microphone is on: turn off'}
        aria-label={micMuted ? 'Unmute microphone' : 'Mute microphone'}
        aria-pressed={micMuted}
        onclick={onToggleMicMute}
      >
        <DeviceIcon kind={micMuted ? 'mic-muted' : 'mic'} size={12} />
      </button>
      <button
        class="top-btn"
        class:on={expanded}
        type="button"
        title={expanded ? 'Unpin notch' : 'Pin notch open'}
        aria-label={expanded ? 'Unpin notch' : 'Pin notch open'}
        aria-pressed={expanded}
        onclick={onToggleExpand}
      >
        <DeviceIcon kind="pin" size={12} />
      </button>
      <button class="top-btn" type="button" title="Settings" aria-label="Open Settings" onclick={onOpenSettings}>
        <DeviceIcon kind="settings" size={12} />
      </button>
    </nav>
  </header>

  <div class="nook-body" class:split={Boolean(activeMedia && showLyrics)}>
    <div class="left-pane">
      {#if activeMedia}
        <div class="nook-media">
          <div class="media-head">
            <AlbumArtTile art={activeMedia.art} size={46} {instant} />
            <div class="media-meta">
              <div class="title-row">
                <span class="media-title">{activeMedia.title || 'Unknown track'}</span>
                {#if isPlaying}
                  <VisualizerBars active={isPlaying && notchVisible} {preset} {instant} variant="expanded" />
                {/if}
              </div>
              <span class="media-artist">{activeMedia.artist || 'Unknown artist'}</span>
            </div>
          </div>

          <MediaScrubber
            media={activeMedia}
            {notchVisible}
            {instant}
            showRemaining
            onSeek={(positionMs) => onMediaAction({ seek: { positionMs } })}
          />

          <div class="transport-center" role="group" aria-label="Media transport">
            <button class="t-btn" type="button" aria-label="Previous track" onclick={() => onMediaAction('previous')}>
              <DeviceIcon kind="prev" size={13} />
            </button>
            <button
              class="t-btn t-play"
              type="button"
              aria-label={isPlaying ? 'Pause' : 'Play'}
              onclick={() => onMediaAction('playPause')}
            >
              <DeviceIcon kind={isPlaying ? 'pause' : 'play'} size={13} />
            </button>
            <button class="t-btn" type="button" aria-label="Next track" onclick={() => onMediaAction('next')}>
              <DeviceIcon kind="next" size={13} />
            </button>
          </div>
        </div>
      {:else}
        <ExpandedEmptyCard
          services={musicServices}
          {activeDeviceName}
          {spatialBadge}
          onOpenService={onOpenMusicService}
        />
      {/if}

      <div class="controls">
        <ExpandedVolumeRow {volumePercent} {muted} {barPercent} {onToggleMute} {onSetVolume} />
        <ExpandedDeviceRow {playback} {settings} {disconnected} {onSwitchDevice} />
      </div>
    </div>

    {#if activeMedia && showLyrics}
      <div class="right-lyrics">
        <LyricsExtension
          {lyrics}
          media={activeMedia}
          mode="nook"
          wordHighlight={lyricsConfig?.wordHighlight ?? true}
          showRomaji={lyricsConfig?.romaji ?? true}
          {instant}
          {notchVisible}
          onSeek={(positionMs) => onMediaAction({ seek: { positionMs } })}
        />
      </div>
    {/if}
  </div>
</div>

<style>
  .nook-shell {
    width: 100%;
    height: 100%;
    padding: 8px 6px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .nook-shell.stagger {
    animation: nook-reveal 220ms cubic-bezier(0.22, 1, 0.36, 1) 30ms both;
  }

  .nook-top {
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-shrink: 0;
  }

  .nook-status {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .device-chip {
    min-width: 0;
    font-size: 10.5px;
    font-weight: 500;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .spatial-badge {
    padding: 1px 6px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-control-bg);
    border: 1px solid var(--ice-control-border);
    font-size: 9.5px;
    font-weight: 600;
    color: var(--ice-accent-strong);
    flex-shrink: 0;
  }

  .top-actions {
    display: flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
  }

  .svc-strip :global(.services.rail) {
    flex-direction: row;
    gap: 2px;
  }

  .top-btn {
    appearance: none;
    width: 22px;
    height: 22px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid transparent;
    border-radius: var(--ice-radius-pill);
    background: transparent;
    color: var(--ice-fg-muted);
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .top-btn:hover {
    background: var(--ice-control-bg-hover);
    color: var(--ice-fg);
  }

  .top-btn.on {
    border-color: var(--ice-accent);
    background: var(--ice-control-bg-active);
    color: var(--ice-accent-strong);
  }

  .top-btn.danger {
    background: color-mix(in srgb, var(--ice-danger) 16%, transparent);
    color: var(--ice-danger);
  }

  .top-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .top-btn:disabled:hover {
    background: transparent;
  }

  .nook-body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
  }

  .left-pane {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 6px;
  }

  .nook-media {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .media-head {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 46px;
  }

  .media-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .media-title {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--ice-fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .media-artist {
    font-size: 11px;
    font-weight: 500;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .transport-center {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
    height: 28px;
  }

  .t-btn {
    appearance: none;
    width: 26px;
    height: 26px;
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
    width: 28px;
    height: 28px;
    background: var(--ice-play-bg);
    color: var(--ice-play-fg);
  }

  .t-play:hover {
    background: var(--ice-play-bg);
    filter: brightness(1.08);
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .right-lyrics {
    width: 238px;
    flex-shrink: 0;
    border-left: 1px solid var(--ice-control-border);
    padding-left: 12px;
    display: flex;
    align-items: center;
  }

  @keyframes nook-reveal {
    from {
      opacity: 0;
      transform: translateY(-3px) scale(0.985);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
