<script lang="ts">
  // D-28 (owner, 2026-11-25): the card's "output" strip, below the song block: where it plays (device chips) and how
  // loud (a small neutral volume control). When the chips need the room, the volume becomes a chip that opens the
  // slider in their place (`volumeFitsInline`). The wheel anywhere on the notch still changes the volume.
  import type { DeviceSummary, SettingsView } from '../../lib/ipc/engine';
  import { buildNotchDeviceRow } from '../../lib/notch/devices';
  import { volumeFitsInline } from '../../lib/notch/volume';
  import DeviceIcon from './DeviceIcon.svelte';
  import ExpandedDeviceRow from './ExpandedDeviceRow.svelte';
  import ExpandedVolumeRow from './ExpandedVolumeRow.svelte';

  interface Props {
    playback: DeviceSummary[];
    settings: SettingsView | null;
    disconnected: DeviceSummary[];
    volumePercent: number;
    muted: boolean;
    barPercent: number;
    onToggleMute: () => void;
    onSetVolume: (percent: number) => void;
    onSwitchDevice: (deviceId: string) => void;
  }

  let { playback, settings, disconnected, volumePercent, muted, barPercent, onToggleMute, onSetVolume, onSwitchDevice }: Props =
    $props();

  let width = $state(0);
  let sliderOpen = $state(false);

  const chipCount = $derived(buildNotchDeviceRow(playback, settings, disconnected).chips.length);
  const inline = $derived(volumeFitsInline(chipCount, width));

  function onKeyDown(e: KeyboardEvent): void {
    if (e.key === 'Escape' && sliderOpen) {
      e.stopPropagation();
      sliderOpen = false;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="strip" bind:clientWidth={width} onkeydown={onKeyDown}>
  {#if inline}
    <div class="chips"><ExpandedDeviceRow {playback} {settings} {disconnected} {onSwitchDevice} /></div>
    <ExpandedVolumeRow {volumePercent} {muted} {barPercent} {onToggleMute} {onSetVolume} />
  {:else if sliderOpen}
    <div class="open-slider">
      <ExpandedVolumeRow {volumePercent} {muted} {barPercent} variant="wide" {onToggleMute} {onSetVolume} />
      <button class="close" type="button" aria-label="Close volume" onclick={() => (sliderOpen = false)}>
        <DeviceIcon kind="close" size={11} />
      </button>
    </div>
  {:else}
    <div class="chips"><ExpandedDeviceRow {playback} {settings} {disconnected} {onSwitchDevice} /></div>
    <button
      class="vol-chip"
      class:muted
      type="button"
      title="Volume: click to adjust, or scroll over the notch"
      aria-label={muted ? 'Volume muted. Adjust volume' : `Volume ${volumePercent} percent. Adjust volume`}
      onclick={() => (sliderOpen = true)}
    >
      <DeviceIcon kind={muted ? 'speaker-muted' : 'speaker'} size={13} />
      <span class="mini-rail" aria-hidden="true">
        <span class="mini-fill" style:transform={`scaleX(${Math.max(0, Math.min(1, barPercent / 100)).toFixed(4)})`}></span>
      </span>
      <span class="pct">{muted ? 'Muted' : `${volumePercent}%`}</span>
    </button>
  {/if}
</div>

<style>
  .strip {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    min-width: 0;
  }

  .chips {
    flex: 1;
    min-width: 0;
  }

  .open-slider {
    flex: 1;
    min-width: 0;
    height: 28px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 2px 0 4px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-control-bg);
  }

  .vol-chip,
  .close {
    appearance: none;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--ice-control-border);
    border-radius: var(--ice-radius-pill);
    background: var(--ice-control-bg);
    color: var(--ice-fg-muted);
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .vol-chip {
    height: 24px;
    gap: 7px;
    padding: 0 10px 0 9px;
  }

  /* A read-only level in the chip: neutral and 3 px like the volume rail, never the accent (D-28 B1). */
  .mini-rail {
    width: 36px;
    height: 3px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
  }

  .mini-fill {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    transform-origin: left center;
    background: currentColor;
  }

  .vol-chip.muted .mini-fill {
    opacity: 0.45;
  }

  .close {
    width: 22px;
    height: 22px;
    border-color: transparent;
    background: transparent;
  }

  .vol-chip:hover,
  .close:hover {
    background: var(--ice-control-bg-hover);
    color: var(--ice-fg);
  }

  .vol-chip:focus-visible,
  .close:focus-visible {
    outline: 2px solid var(--ice-accent);
    outline-offset: 1px;
  }

  .vol-chip.muted {
    color: var(--ice-danger);
  }

  .pct {
    font-family: var(--ice-mono);
    font-size: 10.5px;
  }
</style>
