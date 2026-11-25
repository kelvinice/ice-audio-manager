<script lang="ts">
  import type { DeviceSummary, LyricsState, MediaAction, NowPlaying, SettingsView } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import { hasAnyLyrics } from '../../lib/notch/lyrics';
  import { isMediaActive } from '../../lib/notch/scheduler';
  import ExpandedEmptyCard from './ExpandedEmptyCard.svelte';
  import ExpandedMediaCard from './ExpandedMediaCard.svelte';
  import OutputStrip from './OutputStrip.svelte';
  import LyricsExtension from './LyricsExtension.svelte';
  import NotchSideRail from './NotchSideRail.svelte';

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
    /** The playing output's lyric delay (`DeviceSummary.lyricsDelayMs`). */
    lyricsDelayMs?: number;
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
    isBottom?: boolean;
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
    lyricsDelayMs = 0,
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
    isBottom = false,
  }: Props = $props();

  const activeMedia = $derived(isMediaActive(media) ? media : null);
  const lyricsConfig = $derived(settings?.lyrics ?? null);
  const lyricsAvailable = $derived(hasAnyLyrics(lyrics));
</script>

<div class="expanded-shell" class:stagger={!instant} class:bottom-anchor={isBottom}>
  <div class="expanded-card">
    <div class="main">
      {#if activeMedia}
        <ExpandedMediaCard
          media={activeMedia}
          {spatialBadge}
          {instant}
          {notchVisible}
          {preset}
          showLyricsToggle={Boolean(lyricsConfig?.enabled && lyricsConfig?.showOnCard)}
          lyricsOpen={lyricsDrawerOpen}
          {lyricsAvailable}
          wavyProgress={settings?.appearance.wavyProgress ?? false}
          {onToggleLyrics}
          {onMediaAction}
        />
      {:else}
        <ExpandedEmptyCard
          services={musicServices}
          {activeDeviceName}
          {spatialBadge}
          onOpenService={onOpenMusicService}
        />
      {/if}

      <div class="output">
        <div class="divider" aria-hidden="true"></div>
        <OutputStrip
          {playback}
          {settings}
          {disconnected}
          {volumePercent}
          {muted}
          {barPercent}
          {instant}
          {onToggleMute}
          {onSetVolume}
          {onSwitchDevice}
        />
      </div>
    </div>

    <NotchSideRail
      pinned={expanded}
      {micMuted}
      services={musicServices}
      showServices={activeMedia !== null}
      onTogglePin={onToggleExpand}
      {onToggleMicMute}
      onOpenService={onOpenMusicService}
      {onOpenSettings}
    />
  </div>

  {#if activeMedia && showLyrics}
    <LyricsExtension
      {lyrics}
      media={activeMedia}
      mode="card"
      wordHighlight={lyricsConfig?.wordHighlight ?? true}
      estimatedWordSync={lyricsConfig?.estimatedWordSync ?? false}
      showRomaji={lyricsConfig?.romaji ?? true}
      {instant}
      {notchVisible}
      delayMs={lyricsDelayMs}
      attachedTop={isBottom}
      onSeek={(positionMs) => onMediaAction({ seek: { positionMs } })}
    />
  {/if}
</div>

<style>
  .expanded-shell {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .expanded-shell.bottom-anchor {
    flex-direction: column-reverse;
  }

  .expanded-shell.stagger {
    animation: card-reveal 220ms cubic-bezier(0.22, 1, 0.36, 1) 40ms both;
  }

  .expanded-shell.stagger.bottom-anchor {
    animation-name: card-reveal-up;
  }

  .expanded-card {
    width: 100%;
    flex: 1;
    min-height: 140px;
    padding: 12px 0 12px 2px;
    display: flex;
    gap: 10px;
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 10px;
  }

  /* D-28: the output strip sits below a soft divider that fades out at both ends. */
  .output {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .divider {
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--ice-control-border) 20%, var(--ice-control-border) 80%, transparent);
  }

  @keyframes card-reveal {
    from {
      opacity: 0;
      transform: translateY(-3px) scale(0.985);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes card-reveal-up {
    from {
      opacity: 0;
      transform: translateY(3px) scale(0.985);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
