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
  import StorageSettings from "./StorageSettings.svelte";
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
    onOpenDownloads: () => void;
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
    onOpenDownloads,
    onNotify,
  }: Props = $props();

  const sections = [
    { id: "Appearance", icon: "sparkles" },
    { id: "Playback", icon: "play" },
    { id: "Ice Audio", icon: "snowflake" },
    { id: "Storage", icon: "folder" },
  ] as const;
  type Section = (typeof sections)[number]["id"];
  let section = $state<Section>("Appearance");
  let sectionIdx = $derived(sections.findIndex((s) => s.id === section));
  /** Which way the new section slides in: from the right when moving right, from the left when moving left. */
  let direction = $state(1);

  function selectSection(idx: number) {
    const next = sections[(idx + sections.length) % sections.length];
    if (next.id === section) return;
    direction = idx > sectionIdx ? 1 : -1;
    section = next.id;
  }

  function onTabKey(e: KeyboardEvent) {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    selectSection(sectionIdx + (e.key === "ArrowRight" ? 1 : -1));
    (e.currentTarget as HTMLElement).querySelector<HTMLElement>(`[data-idx="${sectionIdx}"]`)?.focus();
  }
  let busy = $state(false);
  let bridgeError = $state<string | null>(null);

  let defaultPlayback = $derived(iceSnapshot?.playback.find((d) => d.isDefault));
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

  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div
    class="tabs"
    role="tablist"
    aria-label="Settings sections"
    tabindex="-1"
    style:--count={sections.length}
    style:--index={sectionIdx}
    onkeydown={onTabKey}
  >
    <span class="indicator" aria-hidden="true"></span>
    {#each sections as item, i (item.id)}
      <button
        role="tab"
        data-idx={i}
        aria-selected={section === item.id}
        tabindex={section === item.id ? 0 : -1}
        class:active={section === item.id}
        onclick={() => selectSection(i)}
      >
        <Icon name={item.icon} size={15} />
        <span>{item.id}</span>
      </button>
    {/each}
  </div>

  {#key section}
  <div class="section-body" role="tabpanel" aria-label={section} style:--dir={direction}>
  {#if section === "Appearance"}
  <ThemePicker theme={settings.theme} onSelect={(theme) => onUpdateSettings({ theme })} />

    <section class="panel card-glass">
      <div class="panel-head">
        <h2>Appearance &amp; Lyrics</h2>
      </div>

      <label class="toggle-row">
        <div>
          <div class="row-title">Album artwork accents</div>
          <div class="row-sub">Buttons and highlights take the playing cover’s color instead of your theme’s</div>
        </div>
        <input
          type="checkbox"
          checked={settings.artworkAccents}
          onchange={(e) =>
            onUpdateSettings({ artworkAccents: (e.target as HTMLInputElement).checked })}
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

      <label class="toggle-row">
        <div>
          <div class="row-title">Minimize to System Tray on Close</div>
          <div class="row-sub">Keep music playing in the tray when clicking the close button</div>
        </div>
        <input
          type="checkbox"
          checked={settings.closeToTray}
          onchange={(e) =>
            onUpdateSettings({ closeToTray: (e.target as HTMLInputElement).checked })}
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
    <StorageSettings
      {settings}
      {library}
      {appPaths}
      {onUpdateSettings}
      {onOpenDownloads}
      {onClearCache}
      {onRescanLocal}
      {onResetSettings}
      {onNotify}
    />
  {/if}
  </div>
  {/key}
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

  .page { max-width: 1100px; margin-inline: auto; }

  /* Segmented tabs: one pill slides under the chosen tab. */
  .tabs {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--count), minmax(0, 1fr));
    max-width: 640px;
    padding: 4px;
    border-radius: 999px;
    background: var(--p-fill);
  }

  .indicator {
    position: absolute;
    top: 4px;
    bottom: 4px;
    left: 4px;
    width: calc((100% - 8px) / var(--count));
    border-radius: 999px;
    background: var(--p-surface-elevated);
    box-shadow: 0 1px 6px rgb(0 0 0 / 0.22);
    transform: translateX(calc(var(--index) * 100%));
    transition: transform 0.34s cubic-bezier(0.3, 1.25, 0.5, 1);
  }

  .tabs button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-width: 0;
    height: 38px;
    padding: 0 10px;
    border-radius: 999px;
    color: var(--p-text-muted);
    font-size: 13.5px;
    font-weight: 600;
    white-space: nowrap;
    transition: color 0.2s ease;
  }

  .tabs button:hover {
    color: var(--p-text);
  }

  .tabs button.active {
    color: var(--p-text);
  }

  .tabs button.active :global(.p-icon) {
    color: var(--p-accent-bright);
  }

  .tabs button:focus-visible {
    outline: 2px solid var(--p-accent);
    outline-offset: -2px;
  }

  .section-body {
    display: flex;
    flex-direction: column;
    gap: 18px;
    animation: section-in 0.3s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  @keyframes section-in {
    from {
      opacity: 0;
      transform: translateX(calc(var(--dir) * 22px));
    }
  }

  @media (max-width: 620px) {
    .tabs button span {
      display: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .indicator {
      transition: none;
    }

    .section-body {
      animation: none;
    }
  }

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

  .btn-wrap {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
