<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    launchIceStandalone,
    patchIceSettings,
    setIceMeterActive,
    setIceModePref,
    setIceSpatial,
    setIceVolume,
    stopIceStandalone,
    switchIceDevice,
    toggleIceMicMute,
    toggleIceMute,
  } from "../lib/ipc";
  import type { EngineSnapshot, IceStatus, MeterFrame, SpatialMode } from "../lib/types";

  interface Props {
    iceStatus: IceStatus;
    iceSnapshot: EngineSnapshot | null;
    iceMeter: MeterFrame | null;
    modePref: "auto" | "embedded" | "connected";
    onStatusChange: (status: IceStatus) => void;
    onSnapshotChange: (snap: EngineSnapshot) => void;
  }

  let {
    iceStatus,
    iceSnapshot,
    iceMeter,
    modePref,
    onStatusChange,
    onSnapshotChange,
  }: Props = $props();

  let busy = $state(false);
  let actionError = $state<string | null>(null);

  onMount(() => {
    void setIceMeterActive(true).catch(() => {});
  });

  onDestroy(() => {
    void setIceMeterActive(false).catch(() => {});
  });

  async function changePref(pref: "auto" | "embedded" | "connected") {
    busy = true;
    actionError = null;
    try {
      const next = await setIceModePref(pref);
      onStatusChange(next);
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function handleLaunchStandalone() {
    busy = true;
    actionError = null;
    try {
      const next = await launchIceStandalone();
      onStatusChange(next);
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function handleStopStandalone() {
    busy = true;
    actionError = null;
    try {
      const next = await stopIceStandalone();
      onStatusChange(next);
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function patch(data: Parameters<typeof patchIceSettings>[0]) {
    try {
      const snap = await patchIceSettings(data);
      onSnapshotChange(snap);
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    }
  }

  const spatialOptions: { label: string; value: SpatialMode }[] = [
    { label: "Off", value: "off" },
    { label: "Windows Sonic", value: "windowsSonic" },
    { label: "Dolby Atmos Headphones", value: "dolbyAtmosHeadphones" },
    { label: "DTS Headphone:X", value: "dtsHeadphoneX" },
  ];

  let defaultPlayback = $derived(iceSnapshot?.playback.find((d) => d.isDefault));
  let bands = $derived(
    iceMeter?.bands && iceMeter.bands.length > 0
      ? iceMeter.bands
      : Array.from({ length: 14 }, () => iceMeter?.peak ?? 0),
  );
</script>

<div class="page">
  <header class="banner card-glass">
    <div class="banner-left">
      <div class="pill-badge">
        Ice Audio Manager • {iceStatus.mode === "connected" ? "Connected (Co-op)" : iceStatus.mode === "embedded" ? "Wrapped (Embedded)" : iceStatus.mode}
      </div>
      <h1>Windows Audio Engine &amp; Notch Bridge</h1>
      <p>
        Pandora reuses <code>ice-core</code> and <code>ice-win</code> directly. When standalone Ice Audio Manager
        is closed, Pandora wraps its Core Audio engine internally; when standalone Ice is running, Pandora
        hands off hotkeys &amp; Notch ownership and syncs live via Core Audio + <code>config.toml</code>.
      </p>
      <div class="meta-line">
        Config: <code>{iceStatus.configPath}</code>
      </div>
    </div>

    <div class="banner-actions">
      <div class="pref-row">
        <button class="chip" class:active={modePref === "auto"} onclick={() => changePref("auto")}>Auto</button>
        <button class="chip" class:active={modePref === "embedded"} onclick={() => changePref("embedded")}>Wrap</button>
        <button class="chip" class:active={modePref === "connected"} onclick={() => changePref("connected")}>Co-op</button>
      </div>
      {#if iceStatus.standaloneRunning}
        <button class="btn-ghost" disabled={busy} onclick={handleStopStandalone}>
          Stop Standalone Ice &amp; Wrap Internally
        </button>
      {:else}
        <button class="btn-primary" disabled={busy || !iceStatus.standaloneExe} onclick={handleLaunchStandalone}>
          ❄ Launch Standalone Ice Notch
        </button>
      {/if}
    </div>
  </header>

  {#if actionError || iceStatus.error}
    <div class="err card-glass">{actionError ?? iceStatus.error}</div>
  {/if}

  {#if iceSnapshot}
    <div class="grid-two">
      <section class="panel card-glass">
        <div class="panel-head">
          <h2>System Master &amp; Live WASAPI Spectrum</h2>
          <div class="mini-actions">
            <button class="btn-ghost" onclick={() => void toggleIceMute()}>
              {iceSnapshot.volume?.muted ? "Unmute Output" : "Mute Output"}
            </button>
            <button class="btn-ghost" onclick={() => void toggleIceMicMute()}>
              {iceSnapshot.micMuted ? "Unmute Mic" : "Mute Mic"}
            </button>
          </div>
        </div>

        <div class="vol-row">
          <span>Windows Volume</span>
          <input
            type="range"
            min="0"
            max="100"
            value={iceSnapshot.volume?.percent ?? 0}
            oninput={(e) => void setIceVolume(Number((e.target as HTMLInputElement).value))}
          />
          <strong>{iceSnapshot.volume?.percent ?? 0}%</strong>
        </div>

        <div class="meter-bars" title="Live 14-band WASAPI Loopback Spectrum from ice-win::MeterHub">
          {#each bands as val, i (i)}
            <div class="meter-col">
              <div class="meter-fill" style="height: {Math.max(8, Math.min(100, val * 100))}%"></div>
            </div>
          {/each}
        </div>

        {#if defaultPlayback}
          <div class="spatial-row">
            <span>Spatial Audio ({defaultPlayback.name})</span>
            <select
              value={defaultPlayback.spatial ?? "off"}
              onchange={(e) =>
                void setIceSpatial(
                  defaultPlayback.id,
                  (e.target as HTMLSelectElement).value as SpatialMode,
                )}
            >
              {#each spatialOptions as opt}
                <option value={opt.value}>{opt.label}</option>
              {/each}
            </select>
          </div>
        {/if}
      </section>

      <section class="panel card-glass">
        <div class="panel-head">
          <h2>Shared Ice Notch &amp; Call Mode Settings</h2>
          <span class="sub">Hot-reloaded by standalone Ice</span>
        </div>

        <div class="settings-grid">
          <label class="field">
            <span>Notch Theme</span>
            <select
              value={iceSnapshot.settings.appearance.theme}
              onchange={(e) => void patch({ theme: (e.target as HTMLSelectElement).value })}
            >
              <option value="ice">Ice</option>
              <option value="ice_light">Ice Light</option>
              <option value="midnight">Midnight</option>
              <option value="fluent">Fluent</option>
              <option value="minimal">Minimal</option>
            </select>
          </label>

          <label class="field">
            <span>Notch Style</span>
            <select
              value={iceSnapshot.settings.appearance.notchStyle}
              onchange={(e) => void patch({ notchStyle: (e.target as HTMLSelectElement).value })}
            >
              <option value="nook">Edge Nook</option>
              <option value="floating">Floating Pill</option>
            </select>
          </label>

          <label class="field">
            <span>Performance Profile</span>
            <select
              value={iceSnapshot.settings.performanceProfile}
              onchange={(e) => void patch({ performanceProfile: (e.target as HTMLSelectElement).value })}
            >
              <option value="instant">Instant</option>
              <option value="balanced">Balanced</option>
              <option value="lite">Lite (Zero WebView)</option>
            </select>
          </label>

          <label class="check">
            <input
              type="checkbox"
              checked={iceSnapshot.settings.callMode.enabled}
              onchange={(e) => void patch({ callModeEnabled: (e.target as HTMLInputElement).checked })}
            />
            <span>Call Mode Ducking (Discord)</span>
          </label>

          <label class="check">
            <input
              type="checkbox"
              checked={iceSnapshot.settings.appearance.movingBackdrop}
              onchange={(e) => void patch({ movingBackdrop: (e.target as HTMLInputElement).checked })}
            />
            <span>Moving Art Backdrop</span>
          </label>

          <label class="check">
            <input
              type="checkbox"
              checked={iceSnapshot.settings.lyrics.enabled}
              onchange={(e) => void patch({ lyricsEnabled: (e.target as HTMLInputElement).checked })}
            />
            <span>Notch Lyrics Extension</span>
          </label>

          <label class="check">
            <input
              type="checkbox"
              checked={iceSnapshot.settings.visualizerSpectrum}
              onchange={(e) => void patch({ visualizerSpectrum: (e.target as HTMLInputElement).checked })}
            />
            <span>WASAPI 14-Band Spectrum</span>
          </label>

          <label class="check">
            <input
              type="checkbox"
              checked={iceSnapshot.settings.appearance.hibernate}
              onchange={(e) => void patch({ hibernate: (e.target as HTMLInputElement).checked })}
            />
            <span>Hibernate Notch (Meeting Mode)</span>
          </label>
        </div>
      </section>
    </div>

    <section class="panel card-glass">
      <div class="panel-head">
        <h2>Playback Devices ({iceSnapshot.playback.length})</h2>
        <span class="sub">Click any device to switch Windows Multimedia + Console default via IPolicyConfig</span>
      </div>
      <div class="device-grid">
        {#each iceSnapshot.playback as dev (dev.id)}
          <button
            class="device-card"
            class:active={dev.isDefault}
            onclick={() => void switchIceDevice(dev.id)}
          >
            <div class="dev-top">
              <span class="dev-name">{dev.name}</span>
              {#if dev.isDefault}
                <span class="pill-badge">Active</span>
              {/if}
            </div>
            <div class="dev-tags">
              {#if dev.isVirtual}<span>Virtual</span>{/if}
              {#if dev.spatial && dev.spatial !== "off" && dev.spatial !== "keep"}
                <span>{dev.spatial}</span>
              {/if}
              {#if dev.lyricsDelayMs !== 0}
                <span>+{dev.lyricsDelayMs}ms BT delay</span>
              {/if}
            </div>
          </button>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .page {
    padding: 24px 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .banner {
    padding: 22px 24px;
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 18px;
    background: linear-gradient(135deg, var(--p-surface-tint), rgba(14, 18, 30, 0.9));
  }

  .banner-left {
    max-width: 680px;
  }

  .banner-left h1 {
    margin: 10px 0 6px;
    font-size: 22px;
  }

  .banner-left p {
    margin: 0 0 8px;
    font-size: 13px;
    color: var(--p-text-muted);
    line-height: 1.5;
  }

  .meta-line {
    font-size: 11px;
    color: var(--p-text-dim);
  }

  code {
    background: rgba(255, 255, 255, 0.08);
    padding: 1px 6px;
    border-radius: 5px;
  }

  .banner-actions {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: center;
    gap: 10px;
  }

  .pref-row {
    display: flex;
    gap: 6px;
  }

  .chip {
    padding: 6px 12px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--p-border);
  }

  .chip.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    border-color: var(--p-accent);
  }

  .err {
    padding: 12px 16px;
    color: #fca5a5;
    font-size: 13px;
  }

  .grid-two {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
    gap: 16px;
  }

  .panel {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .panel-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .panel-head h2 {
    margin: 0;
    font-size: 16px;
  }

  .sub {
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .mini-actions {
    display: flex;
    gap: 8px;
  }

  .vol-row,
  .spatial-row {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }

  .vol-row input {
    flex: 1;
    accent-color: var(--p-accent);
  }

  .spatial-row select,
  .field select {
    background: rgba(15, 19, 32, 0.95);
    border: 1px solid var(--p-border);
    border-radius: 8px;
    padding: 6px 10px;
    font-size: 12px;
    color: var(--p-text);
  }

  .meter-bars {
    height: 76px;
    display: grid;
    grid-template-columns: repeat(14, 1fr);
    gap: 6px;
    padding: 10px;
    border-radius: 12px;
    background: rgba(0, 0, 0, 0.25);
    align-items: end;
  }

  .meter-col {
    height: 100%;
    display: flex;
    align-items: flex-end;
  }

  .meter-fill {
    width: 100%;
    border-radius: 4px;
    background: linear-gradient(180deg, var(--p-accent-bright), var(--p-accent));
    transition: height 60ms linear;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    padding: 8px 10px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.04);
    cursor: pointer;
  }

  .check input {
    accent-color: var(--p-accent);
  }

  .device-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 10px;
  }

  .device-card {
    padding: 12px 14px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--p-border);
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 6px;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .device-card:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .device-card.active {
    background: var(--p-accent-soft);
    border-color: var(--p-accent);
  }

  .dev-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .dev-name {
    font-size: 13px;
    font-weight: 600;
  }

  .dev-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 11px;
    color: var(--p-text-dim);
  }
</style>
