<script lang="ts">
  import type { DeviceSummary, SettingsView } from '../../lib/ipc/engine';
  import {
    buildNotchDeviceRow,
    isDeviceFavourite,
    toggleFavouriteDeviceIds,
  } from '../../lib/notch/devices';
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    playback: DeviceSummary[];
    settings: SettingsView | null;
    disconnected: DeviceSummary[];
    pickerOpen: boolean;
    onSwitchDevice: (deviceId: string) => void;
    onSetNotchDevices: (deviceIds: string[]) => void;
    onTogglePicker: () => void;
  }

  let {
    playback,
    settings,
    disconnected,
    pickerOpen,
    onSwitchDevice,
    onSetNotchDevices,
    onTogglePicker,
  }: Props = $props();

  const row = $derived(buildNotchDeviceRow(playback, settings, disconnected));
  const favourites = $derived(settings?.favouritesPlayback);
</script>

{#if pickerOpen}
  <div class="device-picker" role="dialog" aria-label="Show in notch devices">
    <div class="picker-head">
      <span class="picker-title">Show in notch</span>
      <button class="chip picker-done" type="button" onclick={onTogglePicker}>Done</button>
    </div>
    <div class="picker-list">
      {#each playback as dev (dev.id)}
        {@const checked = isDeviceFavourite(favourites, dev.id)}
        <div class="picker-row">
          <button
            class="chip picker-dev"
            class:primary={dev.isDefault}
            type="button"
            title={dev.name}
            onclick={() => onSwitchDevice(dev.id)}
          >
            <DeviceIcon kind="auto" formFactor={dev.formFactor} size={12} />
            <span class="dev-chip-label">{dev.name}</span>
          </button>
          <button
            class="chip picker-check"
            class:primary={checked}
            type="button"
            role="checkbox"
            aria-checked={checked}
            aria-label={`Show ${dev.name} in notch`}
            onclick={() => onSetNotchDevices(toggleFavouriteDeviceIds(favourites, dev.id))}
          >
            {checked ? '✓ Show' : '+ Show'}
          </button>
        </div>
      {/each}
    </div>
  </div>
{:else}
  <div class="exp-devices" role="group" aria-label="Playback devices">
    {#each row.chips as chip (chip.savedId)}
      {#if chip.disconnected}
        <span
          class="chip dev-chip disconnected"
          aria-disabled="true"
          title={`${chip.fullName} (disconnected)`}
        >
          <DeviceIcon kind="auto" formFactor={chip.formFactor} size={12} />
          <span class="dev-chip-label">{chip.label}</span>
        </span>
      {:else}
        <button
          class="chip dev-chip"
          class:primary={chip.isCurrent}
          type="button"
          title={chip.fullName}
          onclick={() => onSwitchDevice(chip.id)}
        >
          <DeviceIcon kind="auto" formFactor={chip.formFactor} size={12} />
          <span class="dev-chip-label">{chip.label}</span>
        </button>
      {/if}
    {/each}
    {#if row.pickerLabel}
      <button class="chip more-chip" type="button" onclick={onTogglePicker}>{row.pickerLabel}</button>
    {/if}
  </div>
{/if}

<style>
  .exp-devices {
    display: flex;
    align-items: center;
    gap: 5px;
    overflow: hidden;
  }

  .device-picker {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px 6px;
    border-radius: var(--ice-radius-sm);
    border: 1px solid var(--ice-control-border);
    background: var(--ice-bg);
  }

  .picker-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .picker-title {
    font-size: 10px;
    font-weight: 600;
    color: var(--ice-fg-muted);
  }

  .picker-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--ice-border) transparent;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding-right: 2px;
  }

  .picker-list::-webkit-scrollbar {
    width: 4px;
  }

  .picker-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .picker-list::-webkit-scrollbar-thumb {
    background: var(--ice-border);
    border-radius: var(--ice-radius-pill);
  }

  .picker-row {
    display: flex;
    align-items: center;
    gap: 5px;
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

  .chip:hover:not(.disconnected) {
    background: var(--ice-control-bg-hover);
  }

  .dev-chip {
    flex: 0 1 auto;
    min-width: 48px;
    max-width: 176px;
  }

  .dev-chip.disconnected {
    opacity: 0.5;
    color: var(--ice-fg-muted);
    cursor: default;
  }

  .more-chip,
  .picker-done,
  .picker-check {
    flex-shrink: 0;
    color: var(--ice-fg-muted);
  }

  .picker-dev {
    flex: 1;
    min-width: 0;
    justify-content: flex-start;
  }

  .dev-chip-label {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip.primary {
    border-color: var(--ice-accent);
    background: var(--ice-control-bg-active);
    color: var(--ice-fg);
  }
</style>
