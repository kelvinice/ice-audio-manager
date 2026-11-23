<script lang="ts">
  import type { DeviceSummary } from '../../lib/ipc/engine';
  import { shortDeviceName } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    playback: DeviceSummary[];
    onSwitchDevice: (deviceId: string) => void;
  }

  let { playback, onSwitchDevice }: Props = $props();
</script>

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

<style>
  .exp-devices {
    display: flex;
    align-items: center;
    gap: 5px;
    overflow: hidden;
  }

  .chip {
    appearance: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 1px solid var(--ice-border);
    background: var(--ice-surface);
    color: var(--ice-fg);
    border-radius: var(--ice-radius-sm);
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
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
</style>
