<script lang="ts">
  import { switchIceDevice } from "../lib/ipc";
  import type { DeviceSummary } from "../lib/types";
  import Icon from "./Icon.svelte";

  /** Player-bar output switcher: the device button and a popover of Windows playback devices (through Ice). */
  interface Props {
    devices: DeviceSummary[];
  }

  let { devices }: Props = $props();

  let open = $state(false);
  let root = $state<HTMLDivElement | null>(null);
  let switching = $state<string | null>(null);

  let active = $derived(devices.find((d) => d.isDefault));

  /** Windows EndpointFormFactor: 3 headphones, 5 headset; everything else reads as a speaker. */
  function isHeadphones(device: DeviceSummary): boolean {
    return device.formFactor === 3 || device.formFactor === 5;
  }

  function kind(device: DeviceSummary): string {
    if (device.formFactor === 5) return "Headset";
    if (device.formFactor === 3) return "Headphones";
    if (device.isVirtual) return "Virtual device";
    return "Speakers";
  }

  async function choose(device: DeviceSummary) {
    if (device.isDefault) {
      open = false;
      return;
    }
    switching = device.id;
    try {
      await switchIceDevice(device.id);
    } finally {
      switching = null;
      open = false;
    }
  }
</script>

<svelte:window
  onclick={(e) => open && root && !root.contains(e.target as Node) && (open = false)}
  onkeydown={(e) => open && e.key === "Escape" && (open = false)}
/>

<div class="picker" bind:this={root}>
  <button
    class="btn-icon sm"
    class:on={open}
    title={active ? `Playing on ${active.name}` : "Audio output"}
    aria-label="Audio output"
    aria-expanded={open}
    onclick={() => (open = !open)}
  >
    <Icon name={active && isHeadphones(active) ? "headphones" : "speaker"} size={17} />
  </button>

  {#if open}
    <div class="popover" role="menu" aria-label="Audio output">
      <div class="head">
        <span class="title">Audio output</span>
        <span class="sub">Switches the Windows default device</span>
      </div>
      {#each devices as device (device.id)}
        <button
          class="device"
          class:current={device.isDefault}
          role="menuitemradio"
          aria-checked={device.isDefault}
          disabled={switching !== null}
          onclick={() => void choose(device)}
        >
          <span class="tile"><Icon name={isHeadphones(device) ? "headphones" : "speaker"} size={17} /></span>
          <span class="text">
            <span class="name">{device.name}</span>
            <span class="kind">{device.isDefault ? "Playing now" : kind(device)}</span>
          </span>
          {#if switching === device.id}
            <Icon name="loader" size={15} spin />
          {:else if device.isDefault}
            <Icon name="check" size={16} />
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
  }

  .popover {
    position: absolute;
    right: -8px;
    bottom: calc(100% + 12px);
    z-index: 30;
    width: 300px;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-radius: 12px;
    background: var(--p-surface-elevated);
    box-shadow: 0 16px 40px rgb(0 0 0 / 0.5);
    animation: pop 0.14s ease both;
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 10px 10px;
  }

  .title {
    font-size: 14px;
    font-weight: 750;
  }

  .sub {
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .device {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px 8px 8px;
    border-radius: 8px;
    text-align: left;
    color: var(--p-text);
  }

  .device:hover {
    background: var(--p-fill);
  }

  .device.current {
    color: var(--p-accent-bright);
  }

  .tile {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: var(--p-fill);
    color: var(--p-text-muted);
  }

  .device.current .tile {
    background: var(--p-accent);
    color: var(--p-on-accent);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name,
  .kind {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    font-size: 13.5px;
    font-weight: 600;
  }

  .kind {
    font-size: 12px;
    color: var(--p-text-dim);
  }

  @keyframes pop {
    from {
      opacity: 0;
      translate: 0 6px;
    }
  }
</style>
