<script lang="ts">
  import type { DeviceSummary, LyricsState, MediaAction, NowPlaying, SettingsView } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import type { WidgetBinding } from '../../lib/notch/widget';
  import { hasAnyLyrics } from '../../lib/notch/lyrics';
  import { mediaSource } from '../../lib/notch/mediaSource';
  import { isMediaActive } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';
  import DragHandle from './DragHandle.svelte';
  import ExpandedEmptyCard from './ExpandedEmptyCard.svelte';
  import OutputStrip from './OutputStrip.svelte';
  import LyricsExtension from './LyricsExtension.svelte';
  import NookMediaPanel from './NookMediaPanel.svelte';
  import NookToolbar from './NookToolbar.svelte';

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
    onHibernate?: () => void;
    isBottom?: boolean;
    /** Experimental widget (`NotchController.widget`); `null` keeps the built-in buttons. */
    widget?: WidgetBinding | null;
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
    onHibernate = () => {},
    isBottom = false,
    widget = null,
  }: Props = $props();

  const activeMedia = $derived(isMediaActive(media) ? media : null);
  const lyricsConfig = $derived(settings?.lyrics ?? null);
  const showLyricsToggle = $derived(Boolean(activeMedia && lyricsConfig?.enabled && lyricsConfig?.showOnCard));
  // The header names the player; the device already shows as the highlighted chip (and in the empty card).
  const source = $derived(activeMedia ? mediaSource(activeMedia.appId) : null);
  const paused = $derived(activeMedia !== null && activeMedia.status !== 'playing');
</script>

<div class="nook-shell" class:stagger={!instant} class:bottom-anchor={isBottom}>
  <DragHandle edge={isBottom ? 'top' : 'bottom'} visible={(settings?.notchPlacement ?? 'fixed') !== 'fixed'} />
  <header class="nook-top">
    <div class="nook-status">
      {#if source}
        <span class="source" class:paused title={activeMedia?.appId}>
          <DeviceIcon kind={source.icon} size={12} />
          <span class="source-name">{source.label}</span>
          {#if paused}<span class="source-state">· Paused</span>{/if}
        </span>
      {/if}
      {#if spatialBadge}
        <span class="spatial-badge">{spatialBadge}</span>
      {/if}
    </div>
    <NookToolbar
      services={musicServices}
      showServices={Boolean(activeMedia)}
      {showLyricsToggle}
      lyricsOn={lyricsDrawerOpen}
      lyricsAvailable={hasAnyLyrics(lyrics)}
      {micMuted}
      pinned={expanded}
      onOpenService={onOpenMusicService}
      {onToggleLyrics}
      {onToggleMicMute}
      {onHibernate}
      onTogglePin={onToggleExpand}
      {onOpenSettings}
      {widget}
    />
  </header>

  <div class="nook-body">
    <div class="left-pane">
      {#if activeMedia}
        <NookMediaPanel
          media={activeMedia}
          {notchVisible}
          {instant}
          {preset}
          wavyProgress={settings?.appearance.wavyProgress ?? false}
          coverTilt={settings?.appearance.experiments.coverTilt ?? false}
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

    {#if activeMedia && showLyrics}
      <div class="right-lyrics">
        <LyricsExtension
          {lyrics}
          media={activeMedia}
          mode="nook"
          wordHighlight={lyricsConfig?.wordHighlight ?? true}
          estimatedWordSync={lyricsConfig?.estimatedWordSync ?? false}
          showRomaji={lyricsConfig?.romaji ?? true}
          {instant}
          {notchVisible}
          delayMs={lyricsDelayMs}
          onSeek={(positionMs) => onMediaAction({ seek: { positionMs } })}
        />
      </div>
    {/if}
  </div>
</div>

<style>
  .nook-shell {
    position: relative;
    width: 100%;
    height: 100%;
    padding: 6px 6px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .nook-shell.bottom-anchor {
    padding: 10px 6px 6px;
  }

  .nook-shell.stagger {
    animation: nook-reveal 220ms cubic-bezier(0.22, 1, 0.36, 1) 30ms both;
  }

  .nook-shell.stagger.bottom-anchor {
    animation-name: nook-reveal-bottom;
  }

  .nook-top {
    height: 28px;
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

  .source {
    min-width: 0;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--ice-active-accent, var(--ice-accent));
  }

  .source.paused {
    color: var(--ice-fg-muted);
  }

  .source-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .source-state {
    font-weight: 500;
    color: var(--ice-fg-muted);
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

  /* A soft divider that fades out at both ends instead of a hard full-height rule. */
  .right-lyrics {
    position: relative;
    width: 238px;
    flex-shrink: 0;
    padding-left: 12px;
    display: flex;
    align-items: center;
  }

  .right-lyrics::before {
    content: '';
    position: absolute;
    left: 0;
    top: 10%;
    bottom: 10%;
    width: 1px;
    background: linear-gradient(180deg, transparent, var(--ice-control-border) 25%, var(--ice-control-border) 75%, transparent);
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

  @keyframes nook-reveal-bottom {
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
