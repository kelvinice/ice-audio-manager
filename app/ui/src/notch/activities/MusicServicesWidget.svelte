<script lang="ts">
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    services: MusicServiceItem[];
    variant: 'empty' | 'compact';
    activeDeviceName?: string;
    spatialBadge?: string | null;
    expanded?: boolean;
    onOpenService: (service: MusicServiceId) => void;
    onToggleExpand?: () => void;
  }

  let {
    services,
    variant,
    activeDeviceName = '',
    spatialBadge = null,
    expanded = false,
    onOpenService,
    onToggleExpand = () => {},
  }: Props = $props();
</script>

{#if variant === 'empty'}
  <div class="empty-music-card">
    <div class="empty-top">
      <div class="empty-tile" aria-hidden="true">
        <DeviceIcon kind="music" size={16} />
      </div>
      <div class="empty-meta">
        <span class="empty-title">Nothing playing</span>
        <span class="empty-sub">
          {activeDeviceName || 'Ready'}
          {#if spatialBadge}
            <span class="spatial-inline">· {spatialBadge}</span>
          {/if}
        </span>
      </div>
      <button class="svc-chip pin-btn" class:primary={expanded} type="button" onclick={onToggleExpand}>
        <DeviceIcon kind="pin" size={10} />
        <span>{expanded ? 'Pinned' : 'Pin'}</span>
      </button>
    </div>

    {#if services.length > 0}
      <div class="empty-launch" role="group" aria-label="Music services">
        <span class="launch-label">Play on</span>
        <div class="launch-chips">
          {#each services as svc (svc.id)}
            <button
              class="svc-chip launch-chip"
              class:primary={svc.active}
              type="button"
              onclick={() => onOpenService(svc.id)}
            >
              <DeviceIcon kind={svc.id === 'spotify' ? 'spotify' : 'youtube-music'} size={11} />
              <span>{svc.label}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{:else if services.length > 0}
  <div class="compact-launch" role="group" aria-label="Music services">
    {#each services as svc (svc.id)}
      <button
        class="svc-chip compact-chip"
        class:primary={svc.active}
        type="button"
        title={`Open ${svc.label}`}
        onclick={() => onOpenService(svc.id)}
      >
        <DeviceIcon kind={svc.id === 'spotify' ? 'spotify' : 'youtube-music'} size={11} />
        <span>{svc.shortLabel}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .empty-music-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .empty-top,
  .empty-launch,
  .launch-chips,
  .compact-launch {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .empty-top {
    gap: 8px;
  }

  .empty-launch {
    justify-content: space-between;
  }

  .empty-tile {
    width: 34px;
    height: 34px;
    border-radius: var(--ice-radius-sm);
    background: var(--ice-control-bg);
    border: 1px solid var(--ice-control-border);
    color: var(--ice-accent-strong);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .empty-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    white-space: nowrap;
  }

  .empty-title {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.2;
    color: var(--ice-fg);
  }

  .empty-sub {
    font-size: 10.5px;
    font-weight: 500;
    line-height: 1.2;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .spatial-inline {
    color: var(--ice-accent-strong);
  }

  .launch-label {
    font-size: 10px;
    font-weight: 500;
    color: var(--ice-fg-muted);
    flex-shrink: 0;
  }

  .svc-chip {
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

  .svc-chip:hover {
    background: var(--ice-control-bg-hover);
  }

  .compact-chip {
    padding: 3px 7px;
    color: var(--ice-fg-muted);
  }

  .svc-chip.primary {
    border-color: var(--ice-accent);
    background: var(--ice-control-bg-active);
    color: var(--ice-fg);
  }
</style>
