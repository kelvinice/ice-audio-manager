<script lang="ts">
  import type { DeviceSummary, SettingsView } from '../../lib/ipc/engine';
  import { buildNotchDeviceRow } from '../../lib/notch/devices';
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    playback: DeviceSummary[];
    settings: SettingsView | null;
    disconnected: DeviceSummary[];
    onSwitchDevice: (deviceId: string) => void;
  }

  let { playback, settings, disconnected, onSwitchDevice }: Props = $props();

  // Which devices show here, and in what order, is set in Settings → Devices.
  const row = $derived(buildNotchDeviceRow(playback, settings, disconnected));
</script>

<div class="exp-devices" role="group" aria-label="Playback devices">
  {#each row.chips as chip (chip.savedId)}
    {#if chip.disconnected}
      <span class="chip disconnected" aria-disabled="true" title={`${chip.fullName} (disconnected)`}>
        <DeviceIcon kind="auto" formFactor={chip.formFactor} size={12} />
        <span class="label">{chip.label}</span>
      </span>
    {:else}
      <button
        class="chip"
        class:current={chip.isCurrent}
        type="button"
        title={chip.fullName}
        aria-pressed={chip.isCurrent}
        onclick={() => onSwitchDevice(chip.id)}
      >
        <DeviceIcon kind="auto" formFactor={chip.formFactor} size={12} />
        <span class="label">{chip.label}</span>
      </button>
    {/if}
  {/each}
</div>

<style>
  .exp-devices {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    overflow: hidden;
  }

  .chip {
    appearance: none;
    flex: 0 1 auto;
    min-width: 44px;
    max-width: 170px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 10px 0 8px;
    border: 1px solid var(--ice-control-border);
    border-radius: var(--ice-radius-pill);
    background: var(--ice-control-bg);
    color: var(--ice-fg-muted);
    font-family: inherit;
    font-size: 10.5px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease, border-color 120ms ease;
  }

  .chip:hover:not(.disconnected) {
    background: var(--ice-control-bg-hover);
    color: var(--ice-fg);
  }

  .chip.current {
    border-color: var(--ice-accent);
    background: var(--ice-control-bg-active);
    color: var(--ice-fg);
  }

  .chip.disconnected {
    opacity: 0.5;
    cursor: default;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
