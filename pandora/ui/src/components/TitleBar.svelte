<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import pandoraMark from "../../../../assets/src/pandora.svg";
  import { isTauriEnv, quitApp } from "../lib/ipc";
  import Icon from "./Icon.svelte";
  import MoreMenu from "./MoreMenu.svelte";

  /**
   * Custom window frame (the native one is off in tauri.conf.json): app menu and history on the left, home and the
   * app-wide search in the middle, Ice and the window buttons on the right. Empty space drags the window and a
   * double-click there maximizes, like a native caption.
   */
  interface Props {
    query: string;
    onHome: boolean;
    onSearchPage: boolean;
    canBack: boolean;
    canForward: boolean;
    iceLive: boolean;
    onBack: () => void;
    onForward: () => void;
    onGoHome: () => void;
    onSearch: (query: string) => void;
    onSubmitSearch: () => void;
    onIdentify: () => void;
    onOpenIce: () => void;
    onOpenSettings: () => void;
    onOpenEq: () => void;
    onNewPlaylist: () => void;
    onImportPlaylist: () => void;
  }

  let {
    query,
    onHome,
    onSearchPage,
    canBack,
    canForward,
    iceLive,
    onBack,
    onForward,
    onGoHome,
    onSearch,
    onSubmitSearch,
    onIdentify,
    onOpenIce,
    onOpenSettings,
    onOpenEq,
    onNewPlaylist,
    onImportPlaylist,
  }: Props = $props();

  let input: HTMLInputElement | undefined = $state();
  let maximized = $state(false);
  const appWindow = isTauriEnv() ? getCurrentWindow() : null;

  onMount(() => {
    if (!appWindow) return;
    const sync = () => void appWindow.isMaximized().then((m) => (maximized = m)).catch(() => {});
    sync();
    const pending = appWindow.onResized(sync);
    return () => void pending.then((unlisten) => unlisten());
  });

  function isTyping(target: EventTarget | null): boolean {
    return target instanceof HTMLElement && (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName));
  }

  function onKey(e: KeyboardEvent) {
    const focusSearch = ((e.ctrlKey || e.metaKey) && (e.key === "k" || e.key === "l")) || (e.key === "/" && !isTyping(e.target));
    if (focusSearch) {
      e.preventDefault();
      input?.focus();
      input?.select();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<header class="bar" data-tauri-drag-region>
  <div class="side left" data-tauri-drag-region>
    <img class="mark" src={pandoraMark} alt="Pandora" width="24" height="24" data-tauri-drag-region />
    <MoreMenu
      label="Menu"
      buttonClass="btn-icon sm"
      items={[
        { label: "New playlist", icon: "list-plus", onSelect: onNewPlaylist },
        { label: "Import playlist from a link", icon: "link", onSelect: onImportPlaylist },
        { label: "Equalizer", icon: "sliders", onSelect: onOpenEq },
        { label: "Settings", icon: "settings", onSelect: onOpenSettings },
        { label: "Quit Pandora", icon: "power", onSelect: () => void quitApp() },
      ]}
    />
    <button class="btn-icon sm" aria-label="Back" title="Back (Alt+Left)" disabled={!canBack} onclick={onBack}>
      <Icon name="chevron-left" size={18} />
    </button>
    <button class="btn-icon sm" aria-label="Forward" title="Forward (Alt+Right)" disabled={!canForward} onclick={onForward}>
      <Icon name="chevron-right" size={18} />
    </button>
  </div>

  <div class="center">
    <button class="btn-icon solid home" class:here={onHome} aria-label="Home" title="Home" onclick={onGoHome}>
      <Icon name="home" size={19} />
    </button>
    <form
      class="search"
      class:here={onSearchPage}
      role="search"
      onsubmit={(e) => {
        e.preventDefault();
        onSubmitSearch();
      }}
    >
      <Icon name="search" size={19} />
      <input
        bind:this={input}
        type="text"
        value={query}
        placeholder="What do you want to play?"
        aria-label="Search songs, artists and albums"
        spellcheck="false"
        oninput={(e) => onSearch((e.target as HTMLInputElement).value)}
        onkeydown={(e) => {
          if (e.key === "Escape") {
            if (query) onSearch("");
            else input?.blur();
          }
        }}
      />
      {#if query}
        <button type="button" class="btn-icon sm" aria-label="Clear search" onclick={() => { onSearch(""); input?.focus(); }}>
          <Icon name="x" size={15} />
        </button>
      {/if}
      <span class="divider" aria-hidden="true"></span>
      <button type="button" class="btn-icon sm" aria-label="Identify a song" title="Identify a song" onclick={onIdentify}>
        <Icon name="radar" size={18} />
      </button>
    </form>
  </div>

  <div class="side right" data-tauri-drag-region>
    <button class="btn-icon solid ice" aria-label="Ice Audio" title={iceLive ? "Show the Ice notch" : "Start Ice and show its notch"} onclick={onOpenIce}>
      <Icon name="snowflake" size={18} />
    </button>
    {#if appWindow}
      <div class="win">
        <button aria-label="Minimize" onclick={() => void appWindow.minimize()}>
          <Icon name="win-min" size={15} strokeWidth={1.4} />
        </button>
        <button aria-label={maximized ? "Restore" : "Maximize"} onclick={() => void appWindow.toggleMaximize()}>
          <Icon name={maximized ? "win-restore" : "win-max"} size={14} strokeWidth={1.4} />
        </button>
        <button class="close" aria-label="Close" onclick={() => void appWindow.close()}>
          <Icon name="x" size={16} strokeWidth={1.4} />
        </button>
      </div>
    {/if}
  </div>
</header>

<style>
  .bar {
    position: relative;
    z-index: 20;
    flex-shrink: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    height: 60px;
    padding-left: 14px;
    background: var(--p-rail);
  }

  .side {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 100%;
    min-width: 0;
  }

  .right {
    justify-content: flex-end;
    gap: 10px;
  }

  .mark {
    margin-right: 6px;
    flex-shrink: 0;
  }

  .center {
    display: flex;
    align-items: center;
    gap: 8px;
    width: min(560px, 44vw);
  }

  .home {
    width: 46px;
    height: 46px;
  }

  .search {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 46px;
    padding: 0 8px 0 14px;
    border-radius: 999px;
    background: var(--p-fill);
    color: var(--p-text-muted);
    transition: background 0.14s ease, box-shadow 0.14s ease;
  }

  .search:hover {
    background: color-mix(in srgb, var(--p-text) 9%, var(--p-fill));
  }

  .search:focus-within,
  .search.here {
    box-shadow: inset 0 0 0 2px var(--p-text);
    color: var(--p-text);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    font-size: 15px;
    color: var(--p-text);
  }

  .search input::placeholder {
    color: var(--p-text-dim);
  }

  /* The search field draws its own focus ring. */
  .search input:focus-visible {
    outline: none;
  }

  .divider {
    width: 1px;
    height: 24px;
    background: var(--p-border-strong);
  }

  .ice {
    position: relative;
    color: var(--p-companion);
  }

  .win {
    display: flex;
    align-self: stretch;
  }

  .win button {
    width: 46px;
    display: grid;
    place-items: center;
    color: var(--p-text-muted);
    transition: background 0.12s ease, color 0.12s ease;
  }

  .win button:hover {
    background: var(--p-fill);
    color: var(--p-text);
  }

  /* Windows' own caption close red, so the control reads as the system one. */
  .win .close:hover {
    background: #c42b1c;
    color: #fff;
  }

  @media (max-width: 1000px) {
    .mark {
      display: none;
    }
  }
</style>
