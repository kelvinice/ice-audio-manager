<script lang="ts">
  import { audioEngine } from "../lib/audio";
  import {
    launchIceStandalone,
    openIceSettings,
    pingIceBridge,
    stopIceStandalone,
    toggleIceNotch,
  } from "../lib/ipc";
  import type {
    AppPaths,
    EngineSnapshot,
    IceStatus,
    LibraryState,
    PandoraSettings,
  } from "../lib/types";
  import Icon from "./Icon.svelte";
  import ThemePicker from "./ThemePicker.svelte";

  interface Props {
    settings: PandoraSettings;
    library: LibraryState;
    iceStatus: IceStatus;
    iceSnapshot: EngineSnapshot | null;
    appPaths: AppPaths | null;
    onUpdateSettings: (patch: Partial<PandoraSettings>) => void;
    onResetSettings: () => void;
    onSetModePref: (pref: "auto" | "embedded" | "connected") => void;
    onStatusChange: (status: IceStatus) => void;
    onOpenEq: () => void;
    onOpenIceHub: () => void;
    onRescanLocal: () => void;
    onClearCache: () => void;
    onNotify: (msg: string) => void;
  }

  let {
    settings,
    library,
    iceStatus,
    iceSnapshot,
    appPaths,
    onUpdateSettings,
    onResetSettings,
    onSetModePref,
    onStatusChange,
    onOpenEq,
    onOpenIceHub,
    onRescanLocal,
    onClearCache,
    onNotify,
  }: Props = $props();

  const sections = ["Appearance", "Playback", "Ice Audio", "Storage"] as const;
  let section = $state<(typeof sections)[number]>("Appearance");
  let busy = $state(false);
  let confirmingReset = $state(false);
  let bridgeError = $state<string | null>(null);

  let defaultPlayback = $derived(iceSnapshot?.playback.find((d) => d.isDefault));
  let cacheMb = $derived((library.cacheBytes / (1024 * 1024)).toFixed(1));
  let startupVolPct = $derived(Math.round((settings.defaultVolume ?? 0.85) * 100));

  let bridgeLabel = $derived(
    iceStatus.bridgeConnected
      ? "Connected"
      : iceStatus.standaloneRunning
        ? "External"
        : iceStatus.mode === "embedded"
          ? "Built-in"
          : iceStatus.mode === "fake"
            ? "Fake"
            : "Offline",
  );

  async function runBridgeAction(fn: () => Promise<IceStatus | void>, okMsg: string) {
    busy = true;
    bridgeError = null;
    try {
      const res = await fn();
      if (res) onStatusChange(res);
      onNotify(okMsg);
    } catch (e) {
      bridgeError = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="page">
  <header class="hero">
    <div>
      <h1>Settings</h1>
    </div>
    <div class="hero-actions">
      <button class="btn-ghost" onclick={onOpenEq}>
        <Icon name="sliders" size={15} />
        <span>Equalizer</span>
      </button>
      <button class="btn-primary" onclick={onOpenIceHub}>
        <Icon name="snowflake" size={15} />
        <span>Ice Audio</span>
      </button>
    </div>
  </header>

  <nav class="section-nav" aria-label="Settings sections">
    {#each sections as item}
      <button aria-pressed={section === item} onclick={() => { section = item; confirmingReset = false; }}>{item}</button>
    {/each}
  </nav>

  {#if section === "Appearance"}
  <ThemePicker theme={settings.theme} onSelect={(theme) => onUpdateSettings({ theme })} />

    <section class="panel card-glass">
      <div class="panel-head">
        <h2>Appearance &amp; Lyrics</h2>
      </div>

      <label class="toggle-row">
        <div>
          <div class="row-title">Album artwork accents</div>
          <div class="row-sub">Use the playing cover’s colors over your selected theme</div>
        </div>
        <input
          type="checkbox"
          checked={settings.dynamicTheme}
          onchange={(e) =>
            onUpdateSettings({ dynamicTheme: (e.target as HTMLInputElement).checked })}
        />
      </label>

      <label class="toggle-row">
        <div>
          <div class="row-title">Animated Backdrop</div>
          <div class="row-sub">Ambient glow in Now Playing</div>
        </div>
        <input
          type="checkbox"
          checked={settings.animatedCanvas}
          onchange={(e) =>
            onUpdateSettings({ animatedCanvas: (e.target as HTMLInputElement).checked })}
        />
      </label>

      <label class="toggle-row">
        <div>
          <div class="row-title">Lyrics Romanization</div>
          <div class="row-sub">Japanese, Korean, and Cyrillic offline romanization</div>
        </div>
        <input
          type="checkbox"
          checked={settings.showRomaji}
          onchange={(e) =>
            onUpdateSettings({ showRomaji: (e.target as HTMLInputElement).checked })}
        />
      </label>

      <label class="toggle-row">
        <div>
          <div class="row-title">Wavy Progress Bar</div>
          <div class="row-sub">Animated wave along progress bar while playing</div>
        </div>
        <input
          type="checkbox"
          checked={settings.wavyProgress}
          onchange={(e) =>
            onUpdateSettings({ wavyProgress: (e.target as HTMLInputElement).checked })}
        />
      </label>

      <label class="toggle-row">
        <div>
          <div class="row-title">Spinning Vinyl Cover</div>
          <div class="row-sub">Rotate album artwork inside Now Playing</div>
        </div>
        <input
          type="checkbox"
          checked={settings.spinningCover}
          onchange={(e) =>
            onUpdateSettings({ spinningCover: (e.target as HTMLInputElement).checked })}
        />
      </label>

      <label class="toggle-row">
        <div>
          <div class="row-title">Sidebar Audio Card</div>
          <div class="row-sub">Show output device in sidebar</div>
        </div>
        <input
          type="checkbox"
          checked={settings.showIceCardInSidebar}
          onchange={(e) =>
            onUpdateSettings({ showIceCardInSidebar: (e.target as HTMLInputElement).checked })}
        />
      </label>
    </section>

  {:else if section === "Playback"}
    <section class="panel card-glass">
      <div class="panel-head">
        <h2>Playback</h2>
      </div>

      <label class="toggle-row">
        <div>
          <div class="row-title">Autoplay Radio</div>
          <div class="row-sub">Queue similar songs automatically</div>
        </div>
        <input
          type="checkbox"
          checked={settings.autoQueueRelated}
          onchange={(e) =>
            onUpdateSettings({ autoQueueRelated: (e.target as HTMLInputElement).checked })}
        />
      </label>

      <label class="toggle-row">
        <div>
          <div class="row-title">Windows Media Controls</div>
          <div class="row-sub">Show track info in Windows &amp; Ice Notch</div>
        </div>
        <input
          type="checkbox"
          checked={settings.mediaSessionEnabled}
          onchange={(e) =>
            onUpdateSettings({ mediaSessionEnabled: (e.target as HTMLInputElement).checked })}
        />
      </label>

      <div class="slider-block">
        <div class="slider-top">
          <div>
            <div class="row-title">Startup Volume</div>
            <div class="row-sub">Default volume when Pandora opens</div>
          </div>
          <div class="slider-right">
            <strong>{startupVolPct}%</strong>
            <button
              class="btn-mini"
              onclick={() => {
                audioEngine.setVolume(settings.defaultVolume);
                onNotify(`Volume set to ${startupVolPct}%`);
              }}
            >
              Apply
            </button>
          </div>
        </div>
        <input
          type="range"
          aria-label="Startup volume"
          min="0"
          max="100"
          value={startupVolPct}
          oninput={(e) =>
            onUpdateSettings({
              defaultVolume: Number((e.target as HTMLInputElement).value) / 100,
            })}
        />
      </div>
    </section>
  {:else if section === "Ice Audio"}
  <section class="panel card-glass">
    <div class="panel-head">
      <div>
        <h2>Ice Audio Bridge</h2>
      </div>
      <div class="bridge-badges">
        <span class="pill-badge" class:live={iceStatus.bridgeConnected}>{bridgeLabel}</span>
        {#if iceStatus.notchExpanded !== null}
          <span class="meta-chip">
            Notch: {iceStatus.notchExpanded ? "Expanded" : "Collapsed"}
          </span>
        {/if}
      </div>
    </div>

    {#if bridgeError || iceStatus.error}
      <div class="err-box">{bridgeError ?? iceStatus.error}</div>
    {/if}

    <div class="bridge-grid">
      <div class="bridge-col">
        <div class="row-title">Engine Mode</div>
        <div class="pref-row">
          <button
            class="chip"
            class:active={library.iceModePref === "auto"}
            onclick={() => onSetModePref("auto")}
          >
            Auto
          </button>
          <button
            class="chip"
            class:active={library.iceModePref === "embedded"}
            onclick={() => onSetModePref("embedded")}
          >
            Built-in
          </button>
          <button
            class="chip"
            class:active={library.iceModePref === "connected"}
            onclick={() => onSetModePref("connected")}
          >
            External
          </button>
        </div>

        <label class="toggle-row compact">
          <div>
            <div class="row-title">Launch Ice on Startup</div>
            <div class="row-sub">Start Ice Audio Manager automatically</div>
          </div>
          <input
            type="checkbox"
            checked={settings.autoLaunchIceNotch}
            onchange={(e) =>
              onUpdateSettings({ autoLaunchIceNotch: (e.target as HTMLInputElement).checked })}
          />
        </label>
      </div>

      <div class="bridge-col">
        <div class="row-title">Controls</div>
        <div class="row-sub">
          Output: <strong>{defaultPlayback?.name ?? "Default Output"}</strong>
        </div>
        <div class="btn-wrap">
          <button
            class="btn-ghost"
            disabled={busy}
            onclick={() => void runBridgeAction(pingIceBridge, "Refreshed status")}
          >
            <Icon name="refresh" size={14} spin={busy} />
            <span>Refresh</span>
          </button>
          <button
            class="btn-ghost"
            disabled={busy || !iceStatus.bridgeConnected}
            title={iceStatus.bridgeConnected ? "Toggle desktop Notch" : "Launch Ice to control Notch"}
            onclick={() => void runBridgeAction(toggleIceNotch, "Toggled Ice Notch")}
          >
            <Icon name="panel-top" size={14} />
            <span>{iceStatus.notchExpanded ? "Collapse Notch" : "Toggle Notch"}</span>
          </button>
          <button
            class="btn-ghost"
            disabled={busy || !iceStatus.bridgeConnected}
            onclick={() => void runBridgeAction(openIceSettings, "Opened Ice Settings")}
          >
            <Icon name="settings" size={14} />
            <span>Ice Settings</span>
          </button>
          {#if iceStatus.standaloneRunning}
            <button
              class="btn-ghost"
              disabled={busy}
              onclick={() =>
                void runBridgeAction(stopIceStandalone, "Switched to built-in engine")}
            >
              <Icon name="power" size={14} />
              <span>Stop Ice</span>
            </button>
          {:else}
            <button
              class="btn-primary"
              disabled={busy || !iceStatus.standaloneExe}
              onclick={() =>
                void runBridgeAction(launchIceStandalone, "Launched Ice Audio Manager")}
            >
              <Icon name="snowflake" size={15} />
              <span>Launch Ice</span>
            </button>
          {/if}
        </div>
      </div>
    </div>
  </section>
  {:else}
  <section class="panel card-glass">
    <div class="panel-head">
      <div>
        <h2>Storage &amp; Reset</h2>
      </div>
      <div class="mini-actions">
        <button class="btn-ghost" onclick={onRescanLocal}>
          <Icon name="refresh" size={14} />
          <span>Rescan Folders</span>
        </button>
        <button
          class="btn-ghost"
          disabled={library.cachedIds.length === 0}
          onclick={onClearCache}
        >
          <Icon name="trash" size={14} />
          <span>Clear Cache ({cacheMb} MB)</span>
        </button>
        {#if confirmingReset}
          <button
            class="btn-danger"
            onclick={() => {
              confirmingReset = false;
              onResetSettings();
            }}
          >
            Confirm Reset
          </button>
          <button class="btn-ghost" onclick={() => (confirmingReset = false)}>Cancel</button>
        {:else}
          <button class="btn-ghost" onclick={() => (confirmingReset = true)}>
            Reset Defaults
          </button>
        {/if}
      </div>
    </div>

    <div class="stats-row">
      <div class="stat-card">
        <span>Favorites</span>
        <strong>{library.liked.length}</strong>
      </div>
      <div class="stat-card">
        <span>Playlists</span>
        <strong>{library.playlists.length}</strong>
      </div>
      <div class="stat-card">
        <span>Local Tracks</span>
        <strong>{library.localTracks.length}</strong>
      </div>
      <div class="stat-card">
        <span>Cached</span>
        <strong>{library.cachedIds.length} ({cacheMb} MB)</strong>
      </div>
    </div>

    {#if appPaths}
      <details class="diag-details">
        <summary>File Locations</summary>
        <div class="paths-grid">
          <div><span>Library:</span> <code>{appPaths.libraryPath}</code></div>
          <div><span>Cache:</span> <code>{appPaths.cacheDir}</code></div>
          <div><span>Ice Config:</span> <code>{appPaths.iceConfigPath}</code></div>
          <div><span>Bridge Info:</span> <code>{appPaths.iceBridgePath}</code></div>
        </div>
      </details>
    {/if}
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

  .hero {
    padding: 4px 0;
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 16px;

  }

  .hero h1 {
    margin: 0;
    font-size: clamp(28px, 3vw, 40px);
    letter-spacing: -.04em;
  }

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .section-nav { display: flex; flex-wrap: wrap; border-bottom: 1px solid var(--p-border); gap: 16px; }
  .section-nav button { padding: 8px 2px 12px; color: var(--p-text-muted); font-size: 14px; border-bottom: 2px solid transparent; }
  .section-nav button[aria-pressed="true"] { color: var(--p-accent-bright); border-color: var(--p-accent); font-weight: 650; }
  .page { max-width: 1100px; margin-inline: auto; }

  .panel {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .panel-head {
    margin-bottom: 12px;
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

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 0;
    border-top: 1px solid var(--p-border);
    cursor: pointer;
  }

  .toggle-row.compact {
    margin-top: 8px;
  }

  .row-title {
    font-size: 13px;
    font-weight: 600;
  }

  .row-sub {
    font-size: 12px;
    color: var(--p-text-dim);
    margin-top: 3px;
  }

  .slider-block {
    padding: 12px;
    border-radius: 12px;
    background: var(--p-fill);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .slider-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .slider-right {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .slider-block input[type="range"] {
    width: 100%;
    accent-color: var(--p-accent);
  }

  .btn-mini {
    padding: 4px 10px;
    border-radius: 8px;
    font-size: 11px;
    font-weight: 600;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
  }

  .btn-mini:hover {
    background: rgba(255, 255, 255, 0.14);
  }

  .bridge-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .pill-badge.live {
    background: rgba(52, 211, 153, 0.16);
    color: var(--p-success);
    border: 1px solid rgba(52, 211, 153, 0.45);
  }

  .meta-chip {
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--p-fill);
    color: var(--p-text-muted);
  }

  .err-box {
    padding: 10px 14px;
    border-radius: 10px;
    background: rgba(239, 68, 68, 0.14);
    border: 1px solid rgba(239, 68, 68, 0.35);
    color: var(--p-danger);
    font-size: 12px;
  }

  .bridge-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 16px;
  }

  .bridge-col {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .pref-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 4px;
  }

  .chip {
    padding: 6px 13px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease, transform 0.12s ease;
  }

  .chip:hover {
    background: var(--p-fill);
  }

  .chip:active {
    transform: scale(0.95);
  }

  .chip.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    border-color: var(--p-accent);
  }

  .btn-wrap,
  .mini-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 10px;
  }

  .stat-card {
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-card span {
    font-size: 11px;
    color: var(--p-text-dim);
  }

  .stat-card strong {
    font-size: 16px;
  }

  .diag-details {
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .diag-details summary {
    cursor: pointer;
    padding: 4px 0;
    color: var(--p-text-dim);
  }

  .paths-grid {
    margin-top: 8px;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 8px;
    font-size: 11px;
    color: var(--p-text-dim);
  }

  code {
    background: var(--p-fill);
    padding: 2px 6px;
    border-radius: 5px;
    word-break: break-all;
  }
</style>
