<script lang="ts">
  import type { DeviceSummary } from '../../lib/ipc/engine';
  import type { MonitorSummary, SpringPreset, ThemeKind } from '../../lib/ipc/types';
  import { shortDeviceName } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    playback: DeviceSummary[];
    activeDeviceName: string;
    spatialBadge: string | null;
    volumePercent: number;
    muted: boolean;
    barPercent: number;
    micMuted: boolean;
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
    onCycleMonitor,
    onOpenSettings,
    onSetPreset,
    onSetTheme,
  }: Props = $props();

  const currentMonitor = $derived(monitors[activeMonitor]);
  const clampedScale = $derived(Math.max(0, Math.min(1, barPercent / 100)));
</script>

<div class="expanded-card">
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

  {#if playback.length > 0}
    <div class="exp-devices" role="group" aria-label="Playback devices">
      {#each playback as dev (dev.id)}
        <button
          class="chip dev-chip"
          class:primary={dev.isDefault}
          type="button"
          title={dev.name}
          onclick={() => onSwitchDevice(dev.id)}
        >
          <DeviceIcon kind="auto" formFactor={dev.formFactor} size={12} />
          <span class="dev-chip-label">{shortDeviceName(dev.name)}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="exp-actions">
    <button class="chip mic-btn" class:danger-chip={micMuted} type="button" onclick={onToggleMicMute}>
      <DeviceIcon kind={micMuted ? 'mic-muted' : 'mic'} size={12} />
      <span>{micMuted ? 'Mic Off' : 'Mic On'}</span>
    </button>
    {#if monitors.length > 1}
      <button class="chip" type="button" onclick={onCycleMonitor}>
        Mon {activeMonitor + 1} ({currentMonitor?.refreshHz ?? 60}Hz)
      </button>
    {/if}
    <button class="chip" type="button" onclick={onOpenSettings}>Settings</button>
  </div>

  <div class="exp-footer">
    <div class="preset-row">
      {#each ['snappy', 'smooth', 'bouncy', 'off'] as SpringPreset[] as p}
        <button class="mini-btn" class:active={preset === p} type="button" onclick={() => onSetPreset(p)}>
          {p}
        </button>
      {/each}
    </div>
    <div class="meta">
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
    padding: 9px 2px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 6px;
  }

  .exp-header,
  .exp-vol,
  .exp-devices,
  .exp-actions,
  .exp-footer,
  .preset-row,
  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
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

  .exp-devices {
    overflow: hidden;
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
    padding: 3px 8px;
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .icon-chip {
    padding: 4px 6px;
  }

  .dev-chip {
    min-width: 0;
    max-width: 116px;
    padding: 3px 7px;
    font-size: 10px;
  }

  .dev-chip-label {
    overflow: hidden;
    text-overflow: ellipsis;
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
    padding: 2px 6px;
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
</style>
