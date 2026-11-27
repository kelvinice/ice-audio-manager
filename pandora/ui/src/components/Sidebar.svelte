<script lang="ts">
  import type { DeviceSummary, IceStatus, Playlist } from "../lib/types";
  import Icon from "./Icon.svelte";
  import MoreMenu from "./MoreMenu.svelte";
  import pandoraMark from "../../../../assets/src/pandora.svg";

  export type NavTab = "home" | "search" | "recognize" | "library" | "ice" | "settings";

  interface Props {
    activeTab: NavTab;
    selectedPlaylistId: string | null;
    playlists: Playlist[];
    iceStatus: IceStatus;
    defaultDevice: DeviceSummary | undefined;
    showIceCard: boolean;
    onNavigate: (tab: NavTab, playlistId?: string | null) => void;
    onOpenNewPlaylist: () => void;
    onOpenImportPlaylist: () => void;
  }

  let {
    activeTab,
    selectedPlaylistId,
    playlists,
    iceStatus,
    defaultDevice,
    showIceCard,
    onNavigate,
    onOpenNewPlaylist,
    onOpenImportPlaylist,
  }: Props = $props();
</script>

<aside class="sidebar">
  <div class="brand">
    <img class="brand-mark" src={pandoraMark} alt="" width="36" height="36" />
    <div>
      <div class="brand-name">Pandora</div>
    </div>
  </div>

  <nav class="nav-list" aria-label="Main navigation">
    <button
      class="nav-item"
      class:active={activeTab === "home"}
      onclick={() => onNavigate("home", null)}
    >
      <span class="nav-ic"><Icon name="home" size={17} /></span>
      <span>Home</span>
    </button>
    <button
      class="nav-item"
      class:active={activeTab === "search"}
      onclick={() => onNavigate("search", null)}
    >
      <span class="nav-ic"><Icon name="search" size={17} /></span>
      <span>Search</span>
    </button>
    <button
      class="nav-item"
      class:active={activeTab === "recognize"}
      onclick={() => onNavigate("recognize", null)}
    >
      <span class="nav-ic"><Icon name="radar" size={17} /></span>
      <span>Identify</span>
    </button>
    <button
      class="nav-item"
      class:active={activeTab === "library" && !selectedPlaylistId}
      onclick={() => onNavigate("library", null)}
    >
      <span class="nav-ic"><Icon name="library" size={17} /></span>
      <span>Library</span>
    </button>
    <button
      class="nav-item"
      class:active={activeTab === "ice"}
      onclick={() => onNavigate("ice", null)}
    >
      <span class="nav-ic"><Icon name="snowflake" size={17} /></span>
      <span>Ice Audio</span>
      <span
        class="mode-dot"
        class:connected={iceStatus.mode === "connected"}
        class:embedded={iceStatus.mode === "embedded" || iceStatus.mode === "fake"}
      ></span>
    </button>
    <button
      class="nav-item"
      class:active={activeTab === "settings"}
      onclick={() => onNavigate("settings", null)}
    >
      <span class="nav-ic"><Icon name="settings" size={17} /></span>
      <span>Settings</span>
    </button>
  </nav>

  <div class="divider"></div>

  <div class="playlists-head">
    <span>Playlists</span>
    <MoreMenu
      label="Add a playlist"
      icon="plus"
      buttonClass="btn-icon sm"
      align="right"
      items={[
        { label: "New playlist", icon: "list-plus", onSelect: onOpenNewPlaylist },
        { label: "Import from a link", icon: "link", onSelect: onOpenImportPlaylist },
      ]}
    />
  </div>

  <div class="playlists-list">
    {#if playlists.length === 0}
      <div class="empty-pl">No playlists yet</div>
    {:else}
      {#each playlists as pl (pl.id)}
        <button
          class="pl-item"
          class:active={activeTab === "library" && selectedPlaylistId === pl.id}
          onclick={() => onNavigate("library", pl.id)}
        >
          <span class="pl-ic"><Icon name="list-music" size={14} /></span>
          <span class="pl-name">{pl.name}</span>
          <span class="pl-count">{pl.trackIds.length}</span>
        </button>
      {/each}
    {/if}
  </div>

  {#if showIceCard}
    <button class="ice-card" onclick={() => onNavigate("ice", null)}>
      <div class="ice-card-top">
        <span class="ice-label">
          <Icon name="snowflake" size={12} />
          <span>ICE AUDIO</span>
        </span>
        <span class="ice-pill" class:live={iceStatus.bridgeConnected}>
          {iceStatus.bridgeConnected
            ? "Live"
            : iceStatus.mode === "connected"
              ? "Connected"
              : iceStatus.mode === "embedded"
                ? "Built-in"
                : iceStatus.mode === "fake"
                  ? "Fake"
                  : "Offline"}
        </span>
      </div>
      <div class="ice-device">
        <Icon name="headphones" size={13} />
        <span>{defaultDevice ? defaultDevice.name : "Default Output"}</span>
      </div>
      <div class="ice-hint">
        {iceStatus.bridgeConnected
          ? "Notch connected"
          : iceStatus.standaloneRunning
            ? "Ice app running"
            : "Built-in engine"}
      </div>
    </button>
  {/if}
</aside>

<style>
  .sidebar {
    min-height: 0;
    overflow-y: auto;
    width: clamp(190px, 18vw, 240px);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    padding: 18px 12px 14px;
    background: var(--p-rail);
    border-right: 1px solid var(--p-border);
    backdrop-filter: blur(24px);
    z-index: 2;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 4px 8px 24px;
  }

  .brand-mark { flex-shrink: 0; }

  .brand-name {
    font-size: 21px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .nav-list {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 9px 12px;
    border-radius: 11px;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--p-text-muted);
    transition: background 0.16s ease, color 0.16s ease;
    text-align: left;
  }

  .nav-item:hover {
    background: var(--p-fill);
    color: var(--p-text);
  }

  .nav-item:active {
    transform: scale(0.98);
  }

  .nav-item.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    font-weight: 600;
  }

  .nav-item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--p-accent-bright);
  }

  .nav-ic {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
  }

  .mode-dot {
    margin-left: auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #64748b;
  }

  .mode-dot.embedded {
    background: var(--p-accent-bright);
    box-shadow: 0 0 8px var(--p-accent);
  }

  .mode-dot.connected {
    background: var(--p-success);
    box-shadow: 0 0 8px var(--p-success);
  }

  .divider {
    height: 1px;
    margin: 14px 8px 12px;
    background: var(--p-fill);
  }

  .playlists-head {
    margin: 0 4px 8px 8px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
    font-weight: 700;
    color: var(--p-text-muted);
  }





  .playlists-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-right: 2px;
  }

  .empty-pl {
    padding: 8px 10px;
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .pl-item {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px 10px;
    border-radius: 10px;
    font-size: 13px;
    color: var(--p-text-muted);
    text-align: left;
  }

  .pl-item:hover {
    background: var(--p-fill);
    color: var(--p-text);
  }

  .pl-item.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
  }

  .pl-ic {
    display: inline-flex;
    opacity: 0.72;
  }

  .pl-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pl-count {
    font-size: 11px;
    color: var(--p-text-dim);
    font-variant-numeric: tabular-nums;
  }

  .ice-card {
    margin-top: 12px;
    padding: 11px 12px;
    border-radius: 13px;
    background: var(--p-surface-tint);
    border: 1px solid var(--p-border);
    text-align: left;
    transition: border-color 0.16s ease, transform 0.16s ease;
  }

  .ice-card:hover {
    border-color: var(--p-border-strong);
    transform: translateY(-1px);
  }

  .ice-card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .ice-label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--p-text-dim);
  }

  .ice-pill {
    font-size: 10px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
  }

  .ice-pill.live {
    background: rgba(52, 211, 153, 0.18);
    color: var(--p-success);
  }

  .ice-device {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    font-weight: 600;
    margin-bottom: 3px;
  }

  .ice-device span {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ice-hint {
    font-size: 11px;
    color: var(--p-text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
