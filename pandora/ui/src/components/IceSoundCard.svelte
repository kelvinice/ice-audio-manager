<script lang="ts">
  import { deviceKind, isHeadphones, SPATIAL_CHOICES, SPATIAL_LABELS } from "../lib/devices";
  import { setIceSpatial, setIceVolume, switchIceDevice, toggleIceMicMute, toggleIceMute } from "../lib/ipc";
  import type { EngineSnapshot, IceConfigPatch, MeterFrame, SpatialMode } from "../lib/types";
  import Icon from "./Icon.svelte";

  /** Ice Audio → Sound: which device plays, how loud, the live level, microphone, spatial sound and call ducking. */
  interface Props {
    snapshot: EngineSnapshot;
    meter: MeterFrame | null;
    onPatch: (patch: IceConfigPatch) => void;
  }

  let { snapshot, meter, onPatch }: Props = $props();

  let switching = $state<string | null>(null);

  let active = $derived(snapshot.playback.find((d) => d.isDefault) ?? null);
  let volume = $derived(snapshot.volume?.percent ?? 0);
  let muted = $derived(snapshot.volume?.muted ?? false);
  let volumeIcon = $derived<"volume-x" | "volume-1" | "volume-2">(
    muted || volume === 0 ? "volume-x" : volume < 50 ? "volume-1" : "volume-2",
  );
  let bands = $derived(
    meter?.bands && meter.bands.length > 0 ? meter.bands : Array.from({ length: 14 }, () => meter?.peak ?? 0),
  );

  async function choose(id: string) {
    if (switching) return;
    switching = id;
    try {
      await switchIceDevice(id);
    } finally {
      switching = null;
    }
  }
</script>

<section class="card card-glass" aria-labelledby="sound-title">
  <h2 id="sound-title">Sound</h2>

  <div class="group-label">Output</div>
  <div class="devices" role="radiogroup" aria-label="Output device">
    {#each snapshot.playback as dev (dev.id)}
      <button
        class="device"
        class:current={dev.isDefault}
        role="radio"
        aria-checked={dev.isDefault}
        disabled={switching !== null}
        onclick={() => void choose(dev.id)}
      >
        <span class="tile"><Icon name={isHeadphones(dev) ? "headphones" : "speaker"} size={17} /></span>
        <span class="text">
          <span class="name">{dev.name}</span>
          <span class="sub">
            {dev.isDefault ? "Playing now" : deviceKind(dev)}
            {#if dev.spatial && dev.spatial !== "off" && dev.spatial !== "keep"} • {SPATIAL_LABELS[dev.spatial]}{/if}
            {#if dev.lyricsDelayMs !== 0} • lyrics +{dev.lyricsDelayMs} ms{/if}
          </span>
        </span>
        {#if switching === dev.id}
          <Icon name="loader" size={15} spin />
        {:else if dev.isDefault}
          <Icon name="check" size={16} />
        {/if}
      </button>
    {/each}
  </div>

  <div class="group-label">Volume</div>
  <div class="volume">
    <button
      class="btn-icon sm"
      title={muted ? "Unmute output" : "Mute output"}
      aria-label={muted ? "Unmute output" : "Mute output"}
      onclick={() => void toggleIceMute()}
    >
      <Icon name={volumeIcon} size={18} />
    </button>
    <input
      type="range"
      min="0"
      max="100"
      aria-label="System volume"
      value={volume}
      oninput={(e) => void setIceVolume(Number((e.target as HTMLInputElement).value))}
    />
    <span class="pct">{volume}%</span>
  </div>
  <div class="meter" aria-hidden="true">
    {#each bands as val, i (i)}
      <span class="bar" style:transform={`scaleY(${Math.max(0.06, Math.min(1, val)).toFixed(3)})`}></span>
    {/each}
  </div>

  <div class="rows">
    {#if snapshot.micMuted !== null}
      <div class="row">
        <div class="text">
          <div class="title">Microphone</div>
          <div class="sub">{snapshot.micMuted ? "Muted" : "On"}</div>
        </div>
        <button class="btn-ghost" class:danger={snapshot.micMuted} onclick={() => void toggleIceMicMute()}>
          <Icon name="mic" size={14} />
          <span>{snapshot.micMuted ? "Unmute" : "Mute"}</span>
        </button>
      </div>
    {/if}

    {#if active}
      <label class="row">
        <div class="text">
          <div class="title">Spatial sound</div>
          <div class="sub">For {active.name}</div>
        </div>
        <select
          value={active.spatial ?? "off"}
          onchange={(e) => void setIceSpatial(active.id, (e.target as HTMLSelectElement).value as SpatialMode)}
        >
          {#each SPATIAL_CHOICES as mode (mode)}
            <option value={mode}>{SPATIAL_LABELS[mode]}</option>
          {/each}
        </select>
      </label>
    {/if}

    <label class="row">
      <div class="text">
        <div class="title">Lower music during calls</div>
        <div class="sub">Ice's call mode turns music down while you talk</div>
      </div>
      <input
        type="checkbox"
        checked={snapshot.settings.callMode.enabled}
        onchange={(e) => onPatch({ callModeEnabled: (e.target as HTMLInputElement).checked })}
      />
    </label>
  </div>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    padding: 20px 22px;
    min-width: 0;
  }

  h2 {
    margin: 0 0 4px;
    font-size: 16px;
  }

  .group-label {
    margin: 14px 0 8px;
    font-size: 11.5px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--p-text-dim);
  }

  .devices {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .device {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 10px;
    text-align: left;
    color: var(--p-text-muted);
    transition: background 0.14s ease;
  }

  .device:hover {
    background: var(--p-fill);
  }

  .device.current {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
  }

  .tile {
    width: 34px;
    height: 34px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: var(--p-fill);
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
  .title {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--p-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    font-size: 12px;
    color: var(--p-text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .volume {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .volume input {
    flex: 1;
    accent-color: var(--p-accent);
  }

  .pct {
    min-width: 38px;
    text-align: right;
    font-size: 13px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }

  .meter {
    height: 34px;
    margin: 10px 0 4px;
    padding: 6px 8px;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    gap: 4px;
    border-radius: 10px;
    background: var(--p-fill);
  }

  .bar {
    border-radius: 3px;
    background: var(--p-accent);
    transform-origin: bottom;
    transition: transform 60ms linear;
  }

  .rows {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 0;
    border-top: 1px solid var(--p-border);
  }

  label.row {
    cursor: pointer;
  }

  select {
    max-width: 52%;
    height: 34px;
    padding: 0 10px;
    border: 1px solid var(--p-border);
    border-radius: 8px;
    background: var(--p-surface-solid);
    font-size: 12.5px;
  }
</style>
