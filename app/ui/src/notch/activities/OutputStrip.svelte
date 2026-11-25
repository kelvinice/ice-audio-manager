<script lang="ts">
  // D-28 (owner, 2026-11-25): the card's "output" strip, below the song block: where it plays (device chips) and how
  // loud (a small neutral volume control). When the chips need the room, the volume becomes a chip (icon + %) that
  // grows into a full-strip slider when clicked (owner, 2026-11-26: animated from the chip, back into it on close).
  // The wheel anywhere on the notch still changes the volume.
  import { tick } from 'svelte';
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
    /** Reduce motion or preset `off`: open and close without animating. */
    instant?: boolean;
    onToggleMute: () => void;
    onSetVolume: (percent: number) => void;
    onSwitchDevice: (deviceId: string) => void;
  }

  let {
    playback,
    settings,
    disconnected,
    volumePercent,
    muted,
    barPercent,
    instant = false,
    onToggleMute,
    onSetVolume,
    onSwitchDevice,
  }: Props = $props();

  let width = $state(0);
  let chipWidth = $state(0);
  let sliderOpen = $state(false);
  let panelEl = $state<HTMLDivElement | null>(null);
  let chipEl = $state<HTMLButtonElement | null>(null);

  const chipCount = $derived(buildNotchDeviceRow(playback, settings, disconnected).chips.length);
  const inline = $derived(volumeFitsInline(chipCount, width));

  $effect(() => {
    if (inline) sliderOpen = false;
  });

  async function open(): Promise<void> {
    sliderOpen = true;
    await tick();
    panelEl?.querySelector<HTMLElement>('[role="slider"]')?.focus();
  }

  async function close(): Promise<void> {
    sliderOpen = false;
    await tick();
    chipEl?.focus();
  }

  function onKeyDown(e: KeyboardEvent): void {
    if (e.key === 'Escape' && sliderOpen) {
      e.stopPropagation();
      void close();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="strip" class:instant bind:clientWidth={width} onkeydown={onKeyDown}>
  {#if inline}
    <div class="chips"><ExpandedDeviceRow {playback} {settings} {disconnected} {onSwitchDevice} /></div>
    <ExpandedVolumeRow {volumePercent} {muted} {barPercent} {onToggleMute} {onSetVolume} />
  {:else}
    <div class="chips" class:away={sliderOpen} inert={sliderOpen}>
      <ExpandedDeviceRow {playback} {settings} {disconnected} {onSwitchDevice} />
    </div>
    <button
      bind:this={chipEl}
      bind:offsetWidth={chipWidth}
      class="vol-chip"
      class:muted
      class:away={sliderOpen}
      type="button"
      inert={sliderOpen}
      title="Volume: click to adjust, or scroll over the notch"
      aria-label={muted ? 'Volume muted. Adjust volume' : `Volume ${volumePercent} percent. Adjust volume`}
      aria-expanded={sliderOpen}
      onclick={() => void open()}
    >
      <DeviceIcon kind={muted ? 'speaker-muted' : 'speaker'} size={13} />
      <span class="pct">{muted ? 'Muted' : `${volumePercent}%`}</span>
    </button>
    <!-- Always mounted so it can grow out of the chip: clipped to the chip's box while closed. -->
    <div
      bind:this={panelEl}
      class="open-slider"
      class:open={sliderOpen}
      inert={!sliderOpen}
      style:--chip-w={`${chipWidth}px`}
    >
      <div class="panel-content">
        <ExpandedVolumeRow {volumePercent} {muted} {barPercent} variant="wide" {onToggleMute} {onSetVolume} />
        <button class="close" type="button" aria-label="Close volume" onclick={() => void close()}>
          <DeviceIcon kind="close" size={11} />
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .strip {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    min-width: 0;
  }

  .chips {
    flex: 1;
    min-width: 0;
    transition:
      opacity 160ms ease,
      transform 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .chips.away {
    opacity: 0;
    transform: translateX(-8px);
  }

  /* Closed: clipped to exactly the chip (right edge, 24 px tall); open: the whole 28 px strip. Compositor-only. */
  .open-slider {
    position: absolute;
    inset: 0;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-control-bg);
    clip-path: inset(2px 0 2px calc(100% - var(--chip-w)) round var(--ice-radius-pill));
    opacity: 0;
    pointer-events: none;
    transition:
      clip-path 280ms cubic-bezier(0.22, 1, 0.36, 1),
      opacity 120ms ease 140ms;
  }

  .open-slider.open {
    clip-path: inset(0 0 0 0 round var(--ice-radius-pill));
    opacity: 1;
    pointer-events: auto;
    transition:
      clip-path 280ms cubic-bezier(0.22, 1, 0.36, 1),
      opacity 80ms ease;
  }

  .panel-content {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 2px 0 4px;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .open-slider.open .panel-content {
    opacity: 1;
    transition: opacity 160ms ease 90ms;
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
    transition:
      background-color 120ms ease,
      color 120ms ease,
      opacity 120ms ease;
  }

  .vol-chip {
    height: 24px;
    gap: 5px;
    padding: 0 9px 0 8px;
  }

  .vol-chip.away {
    opacity: 0;
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

  /* Reduce motion / preset off: instant, no fade. */
  .strip.instant .chips,
  .strip.instant .open-slider,
  .strip.instant .panel-content,
  .strip.instant .vol-chip {
    transition: none;
  }
</style>
