<script lang="ts">
  import type { DeviceSummary, MediaAction, NowPlaying, SettingsView } from '../../lib/ipc/engine';
  import type { SpringPreset } from '../../lib/ipc/types';
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import { isMediaActive } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';
  import ExpandedDeviceRow from './ExpandedDeviceRow.svelte';
  import ExpandedMediaCard from './ExpandedMediaCard.svelte';
  import MusicServicesWidget from './MusicServicesWidget.svelte';

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
    onAdjustVolume: (delta: number) => void;
    onSwitchDevice: (deviceId: string) => void;
    onSetNotchDevices: (deviceIds: string[]) => void;
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
    onAdjustVolume,
    onSwitchDevice,
    onSetNotchDevices,
    onToggleMicMute,
    onMediaAction = () => {},
    onOpenMusicService = () => {},
    onOpenSettings,
  }: Props = $props();

  let pickerOpen = $state(false);

  const clampedScale = $derived(Math.max(0, Math.min(1, barPercent / 100)));
  const activeMedia = $derived(isMediaActive(media) ? media : null);
</script>

<div class="expanded-card" class:stagger={!instant}>
  {#if activeMedia}
    <ExpandedMediaCard
      media={activeMedia}
      {spatialBadge}
      {instant}
      {notchVisible}
      {preset}
      {expanded}
      {onMediaAction}
      {onToggleExpand}
    />
  {:else}
    <MusicServicesWidget
      services={musicServices}
      variant="empty"
      {activeDeviceName}
      {spatialBadge}
      {expanded}
      onOpenService={onOpenMusicService}
      {onToggleExpand}
    />
  {/if}

  {#if !pickerOpen}
    <div class="exp-vol">
      <button
        class="chip icon-chip"
        class:danger-chip={muted}
        type="button"
        aria-label={muted ? 'Unmute audio' : 'Mute audio'}
        onclick={onToggleMute}
      >
        <DeviceIcon kind={muted ? 'speaker-muted' : 'speaker'} size={13} />
      </button>
      <div class="bar-track">
        <div class="bar-fill" class:muted style:transform={`scaleX(${clampedScale.toFixed(4)})`}></div>
      </div>
      <span class="pct" class:muted>{muted ? 'Mute' : `${volumePercent}%`}</span>
      <button class="chip vol-step" type="button" aria-label="Decrease volume" onclick={() => onAdjustVolume(-8)}>−</button>
      <button class="chip vol-step" type="button" aria-label="Increase volume" onclick={() => onAdjustVolume(8)}>+</button>
    </div>
  {/if}

  <ExpandedDeviceRow
    {playback}
    {settings}
    {disconnected}
    {pickerOpen}
    {onSwitchDevice}
    {onSetNotchDevices}
    onTogglePicker={() => {
      pickerOpen = !pickerOpen;
    }}
  />

  <div class="exp-footer">
    <button class="chip" class:danger-chip={micMuted} type="button" onclick={onToggleMicMute}>
      <DeviceIcon kind={micMuted ? 'mic-muted' : 'mic'} size={11} />
      <span>{micMuted ? 'Mic Off' : 'Mic On'}</span>
    </button>
    {#if activeMedia}
      <MusicServicesWidget
        services={musicServices}
        variant="compact"
        onOpenService={onOpenMusicService}
      />
    {/if}
    <button class="chip" type="button" onclick={onOpenSettings}>Settings</button>
  </div>
</div>

<style>
  .expanded-card {
    width: 100%;
    height: 100%;
    padding: 9px 2px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 5px;
  }

  .expanded-card.stagger {
    animation: card-reveal 200ms cubic-bezier(0.22, 1, 0.36, 1) 30ms both;
  }

  .exp-vol,
  .exp-footer {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .exp-footer {
    justify-content: space-between;
  }

  .bar-track {
    flex: 1;
    height: 4px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
  }

  .bar-fill {
    width: 100%;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    transform-origin: left center;
    background: linear-gradient(90deg, var(--ice-accent), var(--ice-accent-strong));
  }

  .bar-fill.muted {
    background: var(--ice-danger);
  }

  .pct {
    min-width: 28px;
    text-align: right;
    font-family: var(--ice-mono);
    font-size: 10.5px;
    color: var(--ice-fg-muted);
  }

  .pct.muted {
    color: var(--ice-danger);
  }

  .chip {
    appearance: none;
    display: inline-flex;
    align-items: center;
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
    padding: 4px 6px;
  }

  .vol-step {
    padding: 2px 6px;
    color: var(--ice-fg-muted);
  }

  .chip.danger-chip {
    border-color: var(--ice-danger);
    color: var(--ice-danger);
  }

  @keyframes card-reveal {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
