<script lang="ts">
  import { formatCount } from "../lib/format";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { DeviceSummary, IceStatus, LibraryState, Playlist } from "../lib/types";
  import Icon from "./Icon.svelte";
  import MoreMenu from "./MoreMenu.svelte";

  export type NavTab = "home" | "search" | "recognize" | "library" | "ice" | "settings";
  type LibraryTab = "liked" | "playlists" | "local" | "offline" | "history";

  /** Two panels: page navigation, then "Your Library" (pinned Favorites / Downloads and every playlist with its cover). */
  interface Props {
    activeTab: NavTab;
    libraryTab: LibraryTab;
    selectedPlaylistId: string | null;
    library: LibraryState;
    proxyPort: number;
    iceStatus: IceStatus;
    defaultDevice: DeviceSummary | undefined;
    showIceCard: boolean;
    onNavigate: (tab: NavTab, playlistId?: string | null) => void;
    onOpenLibraryTab: (tab: LibraryTab) => void;
    onOpenNewPlaylist: () => void;
    onOpenImportPlaylist: () => void;
  }

  let {
    activeTab,
    libraryTab,
    selectedPlaylistId,
    library,
    proxyPort,
    iceStatus,
    defaultDevice,
    showIceCard,
    onNavigate,
    onOpenLibraryTab,
    onOpenNewPlaylist,
    onOpenImportPlaylist,
  }: Props = $props();

  const NAV: { tab: NavTab; label: string; icon: "home" | "search" | "radar" | "snowflake" | "settings" }[] = [
    { tab: "home", label: "Home", icon: "home" },
    { tab: "search", label: "Search", icon: "search" },
    { tab: "recognize", label: "Identify", icon: "radar" },
    { tab: "ice", label: "Ice Audio", icon: "snowflake" },
    { tab: "settings", label: "Settings", icon: "settings" },
  ];

  let inLibrary = $derived(activeTab === "library" && !selectedPlaylistId);

  function cover(pl: Playlist): string | null {
    for (const id of pl.trackIds) {
      const url = resolveArtworkUrl(library.tracksIndex[id]?.artworkUrl ?? null, proxyPort);
      if (url) return url;
    }
    return null;
  }

  let iceLine = $derived(
    iceStatus.bridgeConnected
      ? "Connected to the Ice app"
      : iceStatus.standaloneRunning
        ? "Ice app running"
        : iceStatus.mode === "embedded" || iceStatus.mode === "fake"
          ? "Built-in engine"
          : "Ice is offline",
  );
</script>

<aside class="sidebar">
  <nav class="panel nav" aria-label="Main navigation">
    {#each NAV as item (item.tab)}
      <button
        class="nav-item"
        class:active={activeTab === item.tab}
        aria-current={activeTab === item.tab ? "page" : undefined}
        onclick={() => onNavigate(item.tab, null)}
      >
        <Icon name={item.icon} size={20} strokeWidth={activeTab === item.tab ? 2.3 : 1.9} />
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>

  <section class="panel library" aria-label="Your Library">
    <header class="lib-head">
      <button class="lib-title" class:active={inLibrary} onclick={() => onOpenLibraryTab("playlists")}>
        <Icon name="library" size={20} strokeWidth={inLibrary ? 2.3 : 1.9} />
        <span>Your Library</span>
      </button>
      <MoreMenu
        label="Add a playlist"
        icon="plus"
        buttonClass="btn-icon bare sm"
        align="right"
        items={[
          { label: "New playlist", icon: "list-plus", onSelect: onOpenNewPlaylist },
          { label: "Import from a link", icon: "link", onSelect: onOpenImportPlaylist },
        ]}
      />
    </header>

    <div class="items">
      <button
        class="item"
        class:active={activeTab === "library" && !selectedPlaylistId && libraryTab === "liked"}
        onclick={() => onOpenLibraryTab("liked")}
      >
        <span class="tile liked"><Icon name="heart-filled" size={18} /></span>
        <span class="text">
          <span class="name">Favorites</span>
          <span class="sub">Playlist • {formatCount(library.liked.length, "song")}</span>
        </span>
      </button>
      <button
        class="item"
        class:active={activeTab === "library" && !selectedPlaylistId && libraryTab === "offline"}
        onclick={() => onOpenLibraryTab("offline")}
      >
        <span class="tile saved"><Icon name="download" size={18} /></span>
        <span class="text">
          <span class="name">Downloads</span>
          <span class="sub">{formatCount(library.cachedIds.length, "song")}</span>
        </span>
      </button>

      {#each library.playlists as pl (pl.id)}
        {@const art = cover(pl)}
        <button
          class="item"
          class:active={activeTab === "library" && selectedPlaylistId === pl.id}
          onclick={() => onNavigate("library", pl.id)}
        >
          <span class="tile">
            {#if art}<img src={art} alt="" loading="lazy" />{:else}<Icon name="list-music" size={18} />{/if}
          </span>
          <span class="text">
            <span class="name">{pl.name}</span>
            <span class="sub">Playlist • {formatCount(pl.trackIds.length, "song")}</span>
          </span>
        </button>
      {/each}

      {#if library.playlists.length === 0}
        <div class="empty">
          <span>Make your first playlist — it shows up here.</span>
          <button class="btn-ghost" onclick={onOpenNewPlaylist}>New playlist</button>
        </div>
      {/if}
    </div>
  </section>

  {#if showIceCard}
    <button class="panel ice" class:active={activeTab === "ice"} onclick={() => onNavigate("ice", null)}>
      <Icon name="headphones" size={18} />
      <span class="text">
        <span class="name">{defaultDevice ? defaultDevice.name : "Default output"}</span>
        <span class="sub">{iceLine}</span>
      </span>
    </button>
  {/if}
</aside>

<style>
  .sidebar {
    width: clamp(220px, 20vw, 300px);
    flex-shrink: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 0 0 8px 8px;
  }

  .panel {
    border-radius: 10px;
    background: var(--p-bg);
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
  }

  .nav-item,
  .lib-title {
    display: flex;
    align-items: center;
    gap: 14px;
    height: 40px;
    padding: 0 12px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 650;
    color: var(--p-text-muted);
    text-align: left;
    transition: background 0.14s ease, color 0.14s ease;
  }

  .nav-item:hover,
  .lib-title:hover {
    color: var(--p-text);
  }

  .nav-item.active {
    background: var(--p-fill);
    color: var(--p-text);
  }

  .lib-title.active {
    color: var(--p-text);
  }

  .library {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 8px 6px 8px 8px;
  }

  .lib-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    padding-right: 4px;
    margin-bottom: 6px;
  }

  .items {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .item,
  .ice {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 6px;
    border-radius: 8px;
    text-align: left;
    transition: background 0.14s ease;
  }

  .item:hover {
    background: var(--p-fill);
  }

  .item.active {
    background: color-mix(in srgb, var(--p-text) 12%, var(--p-fill));
  }

  .tile {
    width: 48px;
    height: 48px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 6px;
    background: var(--p-fill);
    color: var(--p-text-dim);
  }

  .tile img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .tile.liked {
    background: var(--p-accent);
    color: var(--p-on-accent);
  }

  .tile.saved {
    background: var(--p-companion);
    color: var(--p-on-accent);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .name,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    font-size: 14px;
    font-weight: 600;
    color: var(--p-text);
  }

  .item.active .name {
    color: var(--p-accent-bright);
  }

  .sub {
    font-size: 12.5px;
    color: var(--p-text-dim);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    margin-top: 8px;
    padding: 14px;
    border-radius: 8px;
    background: var(--p-fill);
    font-size: 13px;
    color: var(--p-text-muted);
  }

  .ice {
    padding: 12px 14px;
    color: var(--p-companion);
  }

  .ice:hover,
  .ice.active {
    background: color-mix(in srgb, var(--p-text) 6%, var(--p-bg));
  }
</style>
