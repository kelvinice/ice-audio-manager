<script lang="ts">
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import DeviceIcon from './DeviceIcon.svelte';
  import MusicServicesWidget from './MusicServicesWidget.svelte';

  interface Props {
    services: MusicServiceItem[];
    activeDeviceName: string;
    spatialBadge: string | null;
    onOpenService: (service: MusicServiceId) => void;
  }

  let { services, activeDeviceName, spatialBadge, onOpenService }: Props = $props();
</script>

<div class="empty-card">
  <div class="empty-tile" aria-hidden="true">
    <DeviceIcon kind="music" size={20} />
  </div>
  <div class="empty-info">
    <span class="empty-title">Nothing playing</span>
    <span class="empty-sub">
      {activeDeviceName || 'Ready'}
      {#if spatialBadge}
        <span class="spatial-inline">· {spatialBadge}</span>
      {/if}
    </span>
    <div class="empty-launch">
      <MusicServicesWidget {services} variant="launch" {onOpenService} />
    </div>
  </div>
</div>

<style>
  .empty-card {
    display: flex;
    align-items: stretch;
    gap: 12px;
    height: 58px;
  }

  .empty-tile {
    width: 58px;
    height: 58px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--ice-radius-md);
    border: 1px solid var(--ice-control-border);
    background: var(--ice-control-bg);
    color: var(--ice-accent-strong);
  }

  .empty-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    white-space: nowrap;
  }

  .empty-title {
    font-size: 13px;
    font-weight: 600;
    line-height: 16px;
    color: var(--ice-fg);
  }

  .empty-sub {
    margin-top: 1px;
    font-size: 11px;
    font-weight: 500;
    line-height: 14px;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .spatial-inline {
    color: var(--ice-accent-strong);
  }

  .empty-launch {
    margin-top: auto;
  }
</style>
