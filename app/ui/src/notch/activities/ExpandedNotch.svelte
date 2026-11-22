<script lang="ts">
  import type { DeviceSummary, MediaAction, NowPlaying } from '../../lib/ipc/engine';
  import type { MonitorSummary, SpringPreset, ThemeKind } from '../../lib/ipc/types';
  import {
    formatTimecode,
    isMediaActive,
    mediaProgressPercent,
    shortDeviceName,
  } from '../../lib/notch/scheduler';
  import AlbumArtTile from './AlbumArtTile.svelte';
  import DeviceIcon from './DeviceIcon.svelte';

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
    onCycleMonitor,
    onOpenSettings,
    onSetPreset,
    onSetTheme,
  }: Props = $props();

  const currentMonitor = $derived(monitors[activeMonitor]);
  const clampedScale = $derived(Math.max(0, Math.min(1, barPercent / 100)));
  const activeMedia = $derived(isMediaActive(media) ? media : null);
  const isPlaying = $derived(activeMedia?.status === 'playing');
  const scrubberScale = $derived(Math.max(0, Math.min(1, mediaProgressPercent(activeMedia) / 100)));
  const elapsedLabel = $derived(formatTimecode(activeMedia?.positionMs));
  const durationLabel = $derived(formatTimecode(activeMedia?.durationMs));
</script>

<div class="expanded-card" class:stagger={!instant}>
  {#if activeMedia}
    <div class="media-card">
      <div class="media-top">
        <AlbumArtTile art={activeMedia.art} size={34} {instant} />
        <div class="media-info">
          <span class="media-title">{activeMedia.title || 'Unknown track'}</span>
          <span class="media-sub">
            {activeMedia.artist || 'Unknown artist'}
            {#if spatialBadge}
              <span class="spatial-inline">· {spatialBadge}</span>
            {/if}
          </span>
        </div>
        <div class="media-controls" role="group" aria-label="Media transport">
          <button
            class="chip icon-chip"
            type="button"
            aria-label="Previous track"
            onclick={() => onMediaAction('previous')}
          >
            <DeviceIcon kind="prev" size={11} />
          </button>
          <button
            class="chip icon-chip media-play"
            type="button"
            aria-label={isPlaying ? 'Pause' : 'Play'}
            onclick={() => onMediaAction('playPause')}
          >
            <DeviceIcon kind={isPlaying ? 'pause' : 'play'} size={12} />
          </button>
          <button
            class="chip icon-chip"
            type="button"
            aria-label="Next track"
            onclick={() => onMediaAction('next')}
          >
            <DeviceIcon kind="next" size={11} />
          </button>
        </div>
        <button class="chip pin-chip" class:primary={expanded} type="button" onclick={onToggleExpand}>
          {expanded ? 'Pinned' : 'Pin'}
        </button>
      </div>

      <div class="scrubber-row" aria-label="Track progress">
        <span class="timecode">{elapsedLabel}</span>
        <div class="scrubber-track">
          <div class="scrubber-fill" style:transform={`scaleX(${scrubberScale.toFixed(4)})`}></div>
        </div>
        <span class="timecode">{durationLabel}</span>
      </div>
    </div>
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

  .media-card {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .media-top,
  .scrubber-row,
  .exp-header,
  .exp-vol,
  .exp-devices,
  .exp-actions,
  .exp-footer,
  .preset-row,
  .meta,
  .media-controls {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .exp-header,
  .exp-footer {
    justify-content: space-between;
  }

  .media-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 1px;
    white-space: nowrap;
  }

  .media-title {
    font-size: 12px;
    font-weight: 600;
    line-height: 1.2;
    color: var(--ice-fg);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .media-sub {
    font-size: 10px;
    font-weight: 500;
    line-height: 1.2;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .spatial-inline {
    color: var(--ice-accent-strong);
  }

  .scrubber-track {
    flex: 1;
    height: 4px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
  }

  .scrubber-fill {
    width: 100%;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    transform-origin: left center;
    background: var(--ice-active-accent, var(--ice-accent));
  }

  .timecode {
    min-width: 26px;
    font-family: var(--ice-mono);
    font-size: 10px;
    color: var(--ice-fg-muted);
    flex-shrink: 0;
  }

  .timecode:last-child {
    text-align: right;
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
    padding: 3px 7px;
    font-size: 10px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .icon-chip {
    padding: 4px 5px;
  }

  .media-play {
    border-color: var(--ice-active-accent, var(--ice-accent));
    background: var(--ice-accent-soft);
    color: var(--ice-accent-strong);
  }

  .pin-chip {
    padding: 3px 6px;
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
