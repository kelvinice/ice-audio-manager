<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    launchIceStandalone,
    openIceSettings,
    patchIceSettings,
    pingIceBridge,
    setIceMeterActive,
    setIceModePref,
    stopIceStandalone,
    toggleIceNotch,
  } from "../lib/ipc";
  import type { EngineSnapshot, IceConfigPatch, IceStatus, MeterFrame } from "../lib/types";
  import Icon from "./Icon.svelte";
  import IceNotchCard from "./IceNotchCard.svelte";
  import IceSoundCard from "./IceSoundCard.svelte";

  /**
   * The Ice Audio page: who drives the audio (status and the main actions up top), Sound (devices, volume, mic,
   * spatial, call ducking) beside Notch (Ice's notch look and content), and Connection (which engine Pandora uses).
   */
  type ModePref = "auto" | "embedded" | "connected";

  interface Props {
    iceStatus: IceStatus;
    iceSnapshot: EngineSnapshot | null;
    iceMeter: MeterFrame | null;
    modePref: ModePref;
    onStatusChange: (status: IceStatus) => void;
    onSnapshotChange: (snap: EngineSnapshot) => void;
    onModePrefChange: (pref: ModePref) => void;
  }

  let { iceStatus, iceSnapshot, iceMeter, modePref, onStatusChange, onSnapshotChange, onModePrefChange }: Props =
    $props();

  let busy = $state(false);
  let actionError = $state<string | null>(null);

  const MODES: { id: ModePref; label: string; hint: string }[] = [
    { id: "auto", label: "Auto", hint: "Uses Ice Audio Manager when it is running, otherwise Pandora's built-in engine." },
    { id: "embedded", label: "Built-in", hint: "Pandora always runs its own engine. Ice's notch and hotkeys need Ice running." },
    { id: "connected", label: "External", hint: "Pandora always talks to a running Ice Audio Manager." },
  ];

  onMount(() => void setIceMeterActive(true).catch(() => {}));
  onDestroy(() => void setIceMeterActive(false).catch(() => {}));

  let status = $derived.by(() => {
    if (iceStatus.bridgeConnected)
      return { live: true, text: `Connected to Ice Audio Manager${iceStatus.iceVersion ? ` ${iceStatus.iceVersion}` : ""}` };
    if (iceStatus.standaloneRunning) return { live: true, text: "Ice Audio Manager is running (older version: basic control)" };
    if (iceStatus.mode === "embedded") return { live: true, text: "Built-in engine: Pandora controls your devices" };
    if (iceStatus.mode === "fake") return { live: true, text: "Test devices (--fake-engine)" };
    return { live: false, text: "Audio engine unavailable" };
  });
  let modeHint = $derived(MODES.find((m) => m.id === modePref)?.hint ?? "");

  /** Runs one Ice action with the busy state and error banner; a returned status replaces the current one. */
  async function run(fn: () => Promise<IceStatus | void>) {
    busy = true;
    actionError = null;
    try {
      const next = await fn();
      if (next) onStatusChange(next);
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function changeMode(pref: ModePref) {
    if (pref === modePref) return;
    void run(async () => {
      const next = await setIceModePref(pref);
      onModePrefChange(pref);
      return next;
    });
  }

  async function patch(data: IceConfigPatch) {
    try {
      onSnapshotChange(await patchIceSettings(data));
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="page">
  <header class="top">
    <div>
      <h1>Ice Audio</h1>
      <p class="status"><span class="dot" class:live={status.live}></span>{status.text}</p>
    </div>
    <div class="actions">
      <button class="btn-icon solid" title="Refresh status" aria-label="Refresh status" disabled={busy} onclick={() => void run(pingIceBridge)}>
        <Icon name="refresh" size={16} spin={busy} />
      </button>
      {#if iceStatus.bridgeConnected}
        <button class="btn-ghost" disabled={busy} onclick={() => void run(openIceSettings)}>
          <Icon name="settings" size={14} />
          <span>Ice settings</span>
        </button>
      {/if}
      {#if iceStatus.standaloneRunning}
        <button class="btn-ghost" disabled={busy} onclick={() => void run(stopIceStandalone)}>
          <Icon name="power" size={14} />
          <span>Stop Ice</span>
        </button>
      {:else}
        <button
          class="btn-primary"
          disabled={busy || !iceStatus.standaloneExe}
          title={iceStatus.standaloneExe ? "Start Ice Audio Manager" : "Ice Audio Manager wasn't found; run it once"}
          onclick={() => void run(launchIceStandalone)}
        >
          <Icon name="snowflake" size={15} />
          <span>Start Ice</span>
        </button>
      {/if}
    </div>
  </header>

  {#if actionError || iceStatus.error}
    <div class="err" role="alert">{actionError ?? iceStatus.error}</div>
  {/if}

  {#if iceSnapshot}
    <div class="cards">
      <IceSoundCard snapshot={iceSnapshot} meter={iceMeter} onPatch={(p) => void patch(p)} />
      <IceNotchCard
        settings={iceSnapshot.settings}
        canToggle={iceStatus.bridgeConnected}
        notchExpanded={iceStatus.notchExpanded}
        {busy}
        onPatch={(p) => void patch(p)}
        onToggleNotch={() => void run(toggleIceNotch)}
      />
    </div>
  {:else}
    <section class="card card-glass empty">
      <Icon name="snowflake" size={22} />
      <div>
        <div class="title">No audio engine yet</div>
        <div class="sub">Start Ice Audio Manager, or choose Built-in below, to control your devices from here.</div>
      </div>
    </section>
  {/if}

  <section class="card card-glass connection" aria-labelledby="conn-title">
    <div class="text">
      <h2 id="conn-title">Connection</h2>
      <p class="sub">{modeHint}</p>
    </div>
    <div class="segmented" role="radiogroup" aria-label="Audio engine">
      {#each MODES as mode (mode.id)}
        <button role="radio" aria-checked={modePref === mode.id} class:active={modePref === mode.id} disabled={busy} onclick={() => changeMode(mode.id)}>
          {mode.label}
        </button>
      {/each}
    </div>
  </section>
</div>

<style>
  .page {
    max-width: 1100px;
    margin-inline: auto;
    padding: 24px 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  h1 {
    margin: 0;
    font-size: clamp(28px, 3vw, 40px);
    letter-spacing: -0.04em;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0 0;
    font-size: 13px;
    color: var(--p-text-muted);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--p-text-dim);
  }

  .dot.live {
    background: var(--p-success);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--p-success) 22%, transparent);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .err {
    padding: 10px 14px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--p-danger) 14%, var(--p-fill));
    color: var(--p-danger);
    font-size: 13px;
  }

  /* Sound is the busier card, so it gets a little more room; one column on narrow windows. */
  .cards {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }

  @media (max-width: 960px) {
    .cards {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .card {
    padding: 20px 22px;
  }

  .empty {
    display: flex;
    align-items: center;
    gap: 14px;
    color: var(--p-text-muted);
  }

  .title {
    font-size: 14px;
    font-weight: 650;
    color: var(--p-text);
  }

  .sub {
    margin: 3px 0 0;
    font-size: 12.5px;
    color: var(--p-text-dim);
  }

  .connection {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }

  .connection h2 {
    margin: 0;
    font-size: 16px;
  }

  .segmented {
    display: inline-flex;
    padding: 4px;
    gap: 2px;
    border-radius: 999px;
    background: var(--p-fill);
  }

  .segmented button {
    height: 32px;
    padding: 0 16px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 600;
    color: var(--p-text-muted);
    transition: background 0.18s ease, color 0.18s ease;
  }

  .segmented button:hover {
    color: var(--p-text);
  }

  .segmented button.active {
    background: var(--p-surface-elevated);
    color: var(--p-text);
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.2);
  }
</style>
