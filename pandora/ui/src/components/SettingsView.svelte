<script lang="ts">
  import { audioEngine } from "../lib/audio";
  import {
    launchIceStandalone,
    openIceSettings,
    pingIceBridge,
    showIceNotch,
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

  let busy = $state(false);
  let confirmingReset = $state(false);
  let bridgeError = $state<string | null>(null);

  let defaultPlayback = $derived(iceSnapshot?.playback.find((d) => d.isDefault));
  let cacheMb = $derived((library.cacheBytes / (1024 * 1024)).toFixed(1));
  let startupVolPct = $derived(Math.round((settings.defaultVolume ?? 0.85) * 100));

  let bridgeLabel = $derived(
    iceStatus.bridgeConnected
      ? `Bridge Live • Port ${iceStatus.bridgePort ?? "?"}`
      : iceStatus.standaloneRunning
        ? "Connected (File Sync)"
        : iceStatus.mode === "embedded"
          ? "Wrapped (Embedded)"
          : iceStatus.mode === "fake"
            ? "Fake Engine"
            : "Disconnected",
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
  <header class="hero card-glass">
    <div>
      <div class="pill-badge">Pandora Preferences</div>
      <h1>Settings &amp; Ice Audio Bridge</h1>
      <p>
        Configure Pandora's adaptive theme, Vivi-style Up Next playback behavior, Windows SMTC media
        broadcasting, and live IPC bridging with standalone Ice Audio Manager.
      </p>
    </div>
    <div class="hero-actions">
      <button class="btn-ghost" onclick={onOpenEq}>
        <Icon name="sliders" size={15} />
        <span>Audio EQ &amp; Effects</span>
      </button>
      <button class="btn-primary" onclick={onOpenIceHub}>
        <Icon name="snowflake" size={15} />
        <span>Open Ice Audio Hub</span>
      </button>
    </div>
  </header>

  <div class="grid-two">
    <section class="panel card-glass">
      <div class="panel-head">
        <h2>Appearance &amp; Lyrics</h2>
        <span class="sub">Saved in library.json</span>
      </div>

      <label class="toggle-row">
        <div>
          <div class="row-title">Dynamic Material You Theme</div>
          <div class="row-sub">Extract accent &amp; ambient glow colors from the active album art</div>
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
          <div class="row-title">Animated Now Playing Canvas</div>
          <div class="row-sub">Show flowing multi-orb ambient backdrop inside Now Playing</div>
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
          <div class="row-title">Show Romaji under Japanese Lyrics</div>
          <div class="row-sub">Powered by ice-win offline Japanese phonetic analyzer</div>
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
          <div class="row-title">Show Ice Engine Card in Sidebar</div>
          <div class="row-sub">Quick status badge for Wrapped / Bridge Live output device</div>
        </div>
        <input
          type="checkbox"
          checked={settings.showIceCardInSidebar}
          onchange={(e) =>
            onUpdateSettings({ showIceCardInSidebar: (e.target as HTMLInputElement).checked })}
        />
      </label>
    </section>

    <section class="panel card-glass">
      <div class="panel-head">
        <h2>Playback &amp; Media Integration</h2>
        <span class="sub">Web Audio DSP &amp; Windows SMTC</span>
      </div>

      <label class="toggle-row">
        <div>
          <div class="row-title">Auto-Queue Related Tracks (Up Next Radio)</div>
          <div class="row-sub">Automatically fetch similar InnerTube tracks near queue end</div>
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
          <div class="row-title">Broadcast to Windows SMTC / Ice Notch</div>
          <div class="row-sub">Send track title, art, and timeline so Ice Notch displays Now Playing</div>
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
            <div class="row-title">Default Startup Player Volume</div>
            <div class="row-sub">Applied when Pandora initializes playback</div>
          </div>
          <div class="slider-right">
            <strong>{startupVolPct}%</strong>
            <button
              class="btn-mini"
              onclick={() => {
                audioEngine.setVolume(settings.defaultVolume);
                onNotify(`Applied ${startupVolPct}% volume to player`);
              }}
            >
              Apply Now
            </button>
          </div>
        </div>
        <input
          type="range"
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
  </div>

  <section class="panel card-glass">
    <div class="panel-head">
      <div>
        <h2>Ice Audio Manager Bridge</h2>
        <div class="row-sub">
          Zero-latency localhost JSON-line bridge (`127.0.0.1`) with automatic fallback to Core Audio +
          <code>config.toml</code>
        </div>
      </div>
      <div class="bridge-badges">
        <span class="pill-badge" class:live={iceStatus.bridgeConnected}>{bridgeLabel}</span>
        {#if iceStatus.iceVersion}
          <span class="meta-chip">Ice v{iceStatus.iceVersion} • PID {iceStatus.bridgePid}</span>
        {/if}
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
        <div class="row-title">Ownership Mode Preference</div>
        <div class="row-sub">Auto wraps Ice internally unless standalone Ice is running</div>
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
            Wrap Inside Pandora
          </button>
          <button
            class="chip"
            class:active={library.iceModePref === "connected"}
            onclick={() => onSetModePref("connected")}
          >
            Connected Co-op
          </button>
        </div>

        <label class="toggle-row compact">
          <div>
            <div class="row-title">Auto-Launch Standalone Ice Notch on Startup</div>
            <div class="row-sub">Starts Ice Audio Manager when Pandora opens</div>
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
        <div class="row-title">Live Standalone Bridge Actions</div>
        <div class="row-sub">
          Active Output: <strong>{defaultPlayback?.name ?? "Default Windows Output"}</strong>
        </div>
        <div class="btn-wrap">
          <button
            class="btn-ghost"
            disabled={busy}
            onclick={() => void runBridgeAction(pingIceBridge, "Refreshed Ice bridge status")}
          >
            <Icon name="refresh" size={14} spin={busy} />
            <span>Ping Bridge</span>
          </button>
          <button
            class="btn-ghost"
            disabled={busy || !iceStatus.bridgeConnected}
            title={iceStatus.bridgeConnected ? "Toggle desktop Notch" : "Requires standalone Ice bridge"}
            onclick={() => void runBridgeAction(toggleIceNotch, "Toggled Ice desktop Notch")}
          >
            <Icon name="panel-top" size={14} />
            <span>{iceStatus.notchExpanded ? "Collapse Notch" : "Expand / Toggle Notch"}</span>
          </button>
          <button
            class="btn-ghost"
            disabled={busy || !iceStatus.bridgeConnected}
            onclick={() => void runBridgeAction(showIceNotch, "Revealed Ice desktop Notch")}
          >
            <span>Show Notch</span>
          </button>
          <button
            class="btn-ghost"
            disabled={busy || !iceStatus.bridgeConnected}
            onclick={() => void runBridgeAction(openIceSettings, "Opened Ice Audio Manager Settings")}
          >
            <Icon name="settings" size={14} />
            <span>Open Ice Settings</span>
          </button>
          {#if iceStatus.standaloneRunning}
            <button
              class="btn-ghost"
              disabled={busy}
              onclick={() =>
                void runBridgeAction(stopIceStandalone, "Stopped standalone Ice and wrapped engine")}
            >
              <Icon name="power" size={14} />
              <span>Stop Standalone Ice</span>
            </button>
          {:else}
            <button
              class="btn-primary"
              disabled={busy || !iceStatus.standaloneExe}
              onclick={() =>
                void runBridgeAction(launchIceStandalone, "Launched standalone Ice Audio Manager")}
            >
              <Icon name="snowflake" size={15} />
              <span>Launch Standalone Ice</span>
            </button>
          {/if}
        </div>
      </div>
    </div>
  </section>

  <section class="panel card-glass">
    <div class="panel-head">
      <div>
        <h2>Storage, Local Library &amp; Maintenance</h2>
        <div class="row-sub">All data stays local on Windows • zero analytics</div>
      </div>
      <div class="mini-actions">
        <button class="btn-ghost" onclick={onRescanLocal}>
          <Icon name="refresh" size={14} />
          <span>Rescan Local Folders</span>
        </button>
        <button
          class="btn-ghost"
          disabled={library.cachedIds.length === 0}
          onclick={onClearCache}
        >
          <Icon name="trash" size={14} />
          <span>Clear Offline Cache ({cacheMb} MB)</span>
        </button>
        {#if confirmingReset}
          <button
            class="btn-danger"
            onclick={() => {
              confirmingReset = false;
              onResetSettings();
            }}
          >
            Confirm Reset Settings &amp; EQ
          </button>
          <button class="btn-ghost" onclick={() => (confirmingReset = false)}>Cancel</button>
        {:else}
          <button class="btn-ghost" onclick={() => (confirmingReset = true)}>
            Reset Settings &amp; EQ
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
        <span>Local Folders / Tracks</span>
        <strong>{library.localFolders.length} / {library.localTracks.length}</strong>
      </div>
      <div class="stat-card">
        <span>Offline Cached Tracks</span>
        <strong>{library.cachedIds.length} ({cacheMb} MB)</strong>
      </div>
    </div>

    {#if appPaths}
      <div class="paths-grid">
        <div><span>Pandora DB:</span> <code>{appPaths.libraryPath}</code></div>
        <div><span>Offline Cache:</span> <code>{appPaths.cacheDir}</code></div>
        <div><span>Ice Config:</span> <code>{appPaths.iceConfigPath}</code></div>
        <div><span>Ice Bridge Discovery:</span> <code>{appPaths.iceBridgePath}</code></div>
      </div>
    {/if}
  </section>
</div>

<style>
  .page {
    padding: 24px 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .hero {
    padding: 22px 24px;
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    background: linear-gradient(135deg, var(--p-surface-tint), rgba(14, 18, 30, 0.9));
  }

  .hero h1 {
    margin: 10px 0 6px;
    font-size: 22px;
  }

  .hero p {
    margin: 0;
    max-width: 660px;
    font-size: 13px;
    color: var(--p-text-muted);
    line-height: 1.5;
  }

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
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

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 10px 12px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.05);
    cursor: pointer;
  }

  .toggle-row.compact {
    margin-top: 8px;
  }

  .toggle-row input {
    width: 18px;
    height: 18px;
    accent-color: var(--p-accent);
    flex-shrink: 0;
  }

  .row-title {
    font-size: 13px;
    font-weight: 600;
  }

  .row-sub {
    font-size: 12px;
    color: var(--p-text-dim);
    margin-top: 2px;
  }

  .slider-block {
    padding: 12px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
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
    background: rgba(255, 255, 255, 0.08);
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
    color: #6ee7b7;
    border: 1px solid rgba(52, 211, 153, 0.45);
  }

  .meta-chip {
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--p-text-muted);
  }

  .err-box {
    padding: 10px 14px;
    border-radius: 10px;
    background: rgba(239, 68, 68, 0.14);
    border: 1px solid rgba(239, 68, 68, 0.35);
    color: #fecaca;
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

  .btn-wrap,
  .mini-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .btn-danger {
    padding: 8px 14px;
    border-radius: 10px;
    font-size: 12px;
    font-weight: 600;
    background: rgba(239, 68, 68, 0.25);
    border: 1px solid rgba(239, 68, 68, 0.55);
    color: #fecaca;
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 10px;
  }

  .stat-card {
    padding: 12px 14px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.05);
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

  .paths-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 8px;
    font-size: 11px;
    color: var(--p-text-dim);
  }

  code {
    background: rgba(255, 255, 255, 0.07);
    padding: 2px 6px;
    border-radius: 5px;
    word-break: break-all;
  }
</style>
