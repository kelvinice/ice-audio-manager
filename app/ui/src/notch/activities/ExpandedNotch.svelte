<script lang="ts">
  import type { DeviceSummary, MediaAction, NowPlaying } from '../../lib/ipc/engine';
  import type { MonitorSummary, SpringPreset, ThemeKind } from '../../lib/ipc/types';
  import { DEFAULT_IDLE_MODE, isMediaActive, type IdleMode } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';
  import ExpandedDeviceRow from './ExpandedDeviceRow.svelte';
  import ExpandedMediaCard from './ExpandedMediaCard.svelte';

  interface Props {
    playback: DeviceSummary[];
    activeDeviceName: string;
    spatialBadge: string | null;
    volumePercent: number;
    muted: boolean;
    barPercent: number;
    micMuted: boolean;
    media?: NowPlaying | null;
    instant?: boolean;
    idleMode?: IdleMode;
    notchVisible?: boolean;
    activeMonitor: number;
    monitors: MonitorSummary[];
    preset: SpringPreset;
    theme: ThemeKind;
    frameRate: number;
    expanded: boolean;
    onToggleExpand: () => void;
    onToggleMute: () => void;
    onAdjustVolume: (delta: number) => void;
    onSwitchDevice: (deviceId: string) => void;
    onToggleMicMute: () => void;
    onMediaAction?: (action: MediaAction) => void;
    onSetIdleMode?: (mode: IdleMode) => void;
    onCycleMonitor: () => void;
    onOpenSettings: () => void;
    onSetPreset: (preset: SpringPreset) => void;
    onSetTheme: (theme: ThemeKind) => void;
  }

  let {
    playback,
    activeDeviceName,
    spatialBadge,
    volumePercent,
    muted,
    barPercent,
    micMuted,
    media = null,
    instant = false,
    idleMode = DEFAULT_IDLE_MODE,
    notchVisible = true,
    activeMonitor,
    monitors,
    preset,
    theme,
    frameRate,
    expanded,
    onToggleExpand,
    onToggleMute,
    onAdjustVolume,
    onSwitchDevice,
    onToggleMicMute,
    onMediaAction = () => {},
    onSetIdleMode = () => {},
    onCycleMonitor,
    onOpenSettings,
    onSetPreset,
    onSetTheme,
  }: Props = $props();

  const currentMonitor = $derived(monitors[activeMonitor]);
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
    <div class="exp-header">
      <div class="exp-title">
        <span class="accent-dot"></span>
        <span class="title-text">{activeDeviceName}</span>
        {#if spatialBadge}
          <span class="spatial-pill">{spatialBadge}</span>
        {/if}
      </div>
      <button class="chip" class:primary={expanded} type="button" onclick={onToggleExpand}>
        {expanded ? 'Pinned' : 'Pin'}
      </button>
    </div>
  {/if}

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
    <button class="chip" type="button" aria-label="Decrease volume" onclick={() => onAdjustVolume(-8)}>-</button>
    <button class="chip" type="button" aria-label="Increase volume" onclick={() => onAdjustVolume(8)}>+</button>
  </div>

  <ExpandedDeviceRow {playback} {onSwitchDevice} />

  <div class="exp-footer">
    <div class="exp-actions">
      <button class="chip mic-btn" class:danger-chip={micMuted} type="button" onclick={onToggleMicMute}>
        <DeviceIcon kind={micMuted ? 'mic-muted' : 'mic'} size={11} />
        <span>{micMuted ? 'Mic Off' : 'Mic On'}</span>
      </button>
      {#if monitors.length > 1}
        <button class="chip" type="button" onclick={onCycleMonitor}>
          Mon {activeMonitor + 1} ({currentMonitor?.refreshHz ?? 60}Hz)
        </button>
      {/if}
      <button class="chip" type="button" onclick={onOpenSettings}>Settings</button>
    </div>

    <div class="meta">
      <div class="preset-row" role="group" aria-label="Idle style">
        {#each [['mini_pill', 'Pill'], ['dot', 'Dot'], ['hidden', 'Hide']] as [IdleMode, string][] as [m, label]}
          <button
            class="mini-btn"
            class:active={idleMode === m}
            type="button"
            title={`Idle state: ${label}`}
            onclick={() => onSetIdleMode(m)}
          >
            {label}
          </button>
        {/each}
      </div>
      <div class="preset-row">
        {#each ['snappy', 'smooth', 'bouncy', 'off'] as SpringPreset[] as p}
          <button class="mini-btn" class:active={preset === p} type="button" onclick={() => onSetPreset(p)}>
            {p[0].toUpperCase()}
          </button>
        {/each}
      </div>
      {#each ['ice', 'midnight', 'fluent', 'minimal'] as ThemeKind[] as t}
        <button class="mini-btn" class:active={theme === t} type="button" onclick={() => onSetTheme(t)}>
          {t[0].toUpperCase()}
        </button>
      {/each}
      <span class="fps">{frameRate > 0 ? `${frameRate}Hz` : `${currentMonitor?.refreshHz ?? 60}Hz`}</span>
    </div>
  </div>
</div>

<style>
  .expanded-card {
    width: 100%;
    height: 100%;
    padding: 8px 2px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 5px;
  }

  .expanded-card.stagger {
    animation: card-reveal 200ms cubic-bezier(0.22, 1, 0.36, 1) 30ms both;
  }

  .exp-header,
  .exp-vol,
  .exp-actions,
  .exp-footer,
  .preset-row,
  .meta {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .exp-header,
  .exp-footer {
    justify-content: space-between;
  }

  .exp-title {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }

  .title-text {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ice-fg);
  }

  .accent-dot {
    width: 7px;
    height: 7px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-accent);
    flex-shrink: 0;
  }

  .bar-track {
    flex: 1;
    height: var(--ice-bar-height);
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
    min-width: 30px;
    text-align: right;
    font-family: var(--ice-mono);
    font-size: 11px;
    color: var(--ice-fg-muted);
  }

  .pct.muted {
    color: var(--ice-danger);
  }

  .spatial-pill {
    padding: 1px 6px;
    border-radius: var(--ice-radius-pill);
    font-size: 10px;
    background: var(--ice-accent-soft);
    color: var(--ice-accent-strong);
    border: 1px solid var(--ice-border);
    flex-shrink: 0;
  }

  .chip,
  .mini-btn {
    appearance: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 1px solid var(--ice-border);
    background: var(--ice-surface);
    color: var(--ice-fg);
    border-radius: var(--ice-radius-sm);
    padding: 3px 7px;
    font-size: 10px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .icon-chip {
    padding: 4px 5px;
  }

  .chip.primary {
    border-color: var(--ice-accent);
    background: var(--ice-accent-soft);
    color: var(--ice-accent-strong);
  }

  .chip.danger-chip {
    border-color: var(--ice-danger);
    color: var(--ice-danger);
  }

  .mini-btn {
    padding: 2px 5px;
    font-size: 10px;
    color: var(--ice-fg-muted);
  }

  .mini-btn.active {
    border-color: var(--ice-accent);
    color: var(--ice-accent-strong);
  }

  .fps {
    font-family: var(--ice-mono);
    font-size: 10px;
    color: var(--ice-fg-muted);
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
