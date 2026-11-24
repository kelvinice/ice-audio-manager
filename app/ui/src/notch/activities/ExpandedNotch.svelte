<script lang="ts">
  import type { DeviceSummary, MediaAction, NowPlaying, SettingsView } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import { isMediaActive } from '../../lib/notch/scheduler';
  import ExpandedDeviceRow from './ExpandedDeviceRow.svelte';
  import ExpandedEmptyCard from './ExpandedEmptyCard.svelte';
  import ExpandedMediaCard from './ExpandedMediaCard.svelte';
  import ExpandedVolumeRow from './ExpandedVolumeRow.svelte';
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
    media?: NowPlaying | null;
    instant?: boolean;
    notchVisible?: boolean;
    preset: SpringPreset;
    expanded: boolean;
    onToggleExpand: () => void;
    onToggleMute: () => void;
    onSetVolume: (percent: number) => void;
    onSwitchDevice: (deviceId: string) => void;
    onToggleMicMute: () => void;
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
    instant = false,
    notchVisible = true,
    preset,
    expanded,
    onToggleExpand,
    onToggleMute,
    onSetVolume,
    onSwitchDevice,
    onToggleMicMute,
    onMediaAction = () => {},
    onOpenMusicService = () => {},
    onOpenSettings,
  }: Props = $props();

  const activeMedia = $derived(isMediaActive(media) ? media : null);
</script>

<div class="expanded-card" class:stagger={!instant}>
  <div class="main">
    {#if activeMedia}
      <ExpandedMediaCard media={activeMedia} {spatialBadge} {instant} {notchVisible} {preset} {onMediaAction} />
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

<style>
  .expanded-card {
    width: 100%;
    height: 100%;
    padding: 12px 0 12px 2px;
    display: flex;
    gap: 10px;
  }

  .expanded-card.stagger {
    animation: card-reveal 220ms cubic-bezier(0.22, 1, 0.36, 1) 40ms both;
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 10px;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 6px;
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
</style>
