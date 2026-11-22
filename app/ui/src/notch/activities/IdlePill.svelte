<script lang="ts">
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    deviceLabel: string;
    volumePercent: number;
    muted: boolean;
    micMuted: boolean;
  }

  let { deviceLabel, volumePercent, muted, micMuted }: Props = $props();
</script>

<div class="idle-row">
  {#if micMuted}
    <span class="mic-badge" title="Microphone muted" aria-label="Microphone muted">
      <DeviceIcon kind="mic-muted" size={11} />
    </span>
  {:else}
    <span class="dot" aria-hidden="true"></span>
  {/if}
  <span class="idle-label">{deviceLabel}</span>
  <span class="vol-mini" class:muted>{muted ? 'Mute' : `${volumePercent}%`}</span>
</div>

<style>
  .idle-row {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    font-size: 11px;
    font-weight: 500;
    white-space: nowrap;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-accent);
    flex-shrink: 0;
  }

  .mic-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-accent-soft);
    border: 1px solid var(--ice-danger);
    color: var(--ice-danger);
    flex-shrink: 0;
  }

  .idle-label {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ice-fg);
  }

  .vol-mini {
    font-family: var(--ice-mono);
    font-size: 11px;
    color: var(--ice-fg-muted);
    flex-shrink: 0;
  }

  .vol-mini.muted {
    color: var(--ice-danger);
  }
</style>
