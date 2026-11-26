<script lang="ts">
  import type { DeviceSummary, IceStatus, Playlist } from "../lib/types";

  export type NavTab = "home" | "search" | "library" | "ice";

  interface Props {
    activeTab: NavTab;
    selectedPlaylistId: string | null;
    playlists: Playlist[];
    iceStatus: IceStatus;
    defaultDevice: DeviceSummary | undefined;
    onNavigate: (tab: NavTab, playlistId?: string | null) => void;
    onCreatePlaylist: (name: string) => void;
  }

  let {
    activeTab,
    selectedPlaylistId,
    playlists,
    iceStatus,
    defaultDevice,
    onNavigate,
    onCreatePlaylist,
  }: Props = $props();

  let creating = $state(false);
  let newName = $state("");

  function submitPlaylist(e: Event) {
    e.preventDefault();
    if (!newName.trim()) return;
    onCreatePlaylist(newName.trim());
    newName = "";
    creating = false;
  }
</script>

<aside class="sidebar">
  <div class="brand">
    <div class="logo-ring">
      <div class="logo-diamond"></div>
    </div>
    <div>
      <div class="brand-name">Pandora</div>
      <div class="brand-sub">Vivi-Inspired • Windows</div>
    </div>
  </div>

  <nav class="nav-list">
    <button
      class="nav-item"
      class:active={activeTab === "home"}
      onclick={() => onNavigate("home", null)}
    >
      <span class="nav-icon">⌂</span>
      <span>Home</span>
    </button>
    <button
      class="nav-item"
      class:active={activeTab === "search"}
      onclick={() => onNavigate("search", null)}
    >
      <span class="nav-icon">⌕</span>
      <span>Search</span>
    </button>
    <button
      class="nav-item"
      class:active={activeTab === "library" && !selectedPlaylistId}
      onclick={() => onNavigate("library", null)}
    >
      <span class="nav-icon">♫</span>
      <span>Library &amp; Local</span>
    </button>
    <button
      class="nav-item ice-nav"
      class:active={activeTab === "ice"}
      onclick={() => onNavigate("ice", null)}
    >
      <span class="nav-icon">❄</span>
      <span>Ice Audio Hub</span>
      <span class="mode-dot" class:connected={iceStatus.mode === "connected"} class:embedded={iceStatus.mode === "embedded" || iceStatus.mode === "fake"}></span>
    </button>
  </nav>

  <div class="playlists-head">
    <span>Playlists</span>
    <button class="icon-add" title="New playlist" onclick={() => (creating = !creating)}>+</button>
  </div>

  {#if creating}
    <form class="create-form" onsubmit={submitPlaylist}>
      <input
        type="text"
        placeholder="Playlist name..."
        bind:value={newName}
      />
      <button type="submit">Add</button>
    </form>
  {/if}

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
          <span class="pl-name">{pl.name}</span>
          <span class="pl-count">{pl.trackIds.length}</span>
        </button>
      {/each}
    {/if}
  </div>

  <button class="ice-card" onclick={() => onNavigate("ice", null)}>
    <div class="ice-card-top">
      <span class="ice-label">ICE AUDIO ENGINE</span>
      <span class="ice-pill">
        {iceStatus.mode === "connected"
          ? "Connected"
          : iceStatus.mode === "embedded"
            ? "Wrapped"
            : iceStatus.mode === "fake"
              ? "Fake"
              : "Offline"}
      </span>
    </div>
    <div class="ice-device">
      {defaultDevice ? defaultDevice.name : "Default Windows Output"}
    </div>
    <div class="ice-hint">
      {iceStatus.standaloneRunning
        ? "Standalone Ice Notch active"
        : "Wrapped inside Pandora • Click to manage"}
    </div>
  </button>
</aside>

<style>
  .sidebar {
    width: 244px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    padding: 18px 14px 14px;
    background: rgba(12, 15, 25, 0.78);
    border-right: 1px solid var(--p-border);
    backdrop-filter: blur(22px);
    z-index: 2;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 8px 18px;
  }

  .logo-ring {
    width: 36px;
    height: 36px;
    border-radius: 12px;
    background: linear-gradient(135deg, var(--p-accent), #38bdf8);
    display: grid;
    place-items: center;
    box-shadow: 0 6px 20px var(--p-glow-1);
    transition: background 0.6s ease;
  }

  .logo-diamond {
    width: 14px;
    height: 14px;
    background: #0b0d14;
    transform: rotate(45deg);
    border-radius: 3px;
  }

  .brand-name {
    font-size: 17px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .brand-sub {
    font-size: 11px;
    color: var(--p-text-dim);
  }

  .nav-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 12px;
    font-size: 14px;
    font-weight: 500;
    color: var(--p-text-muted);
    transition: background 0.16s ease, color 0.16s ease;
    text-align: left;
  }

  .nav-item:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--p-text);
  }

  .nav-item.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    font-weight: 600;
  }

  .nav-icon {
    width: 20px;
    text-align: center;
    font-size: 15px;
  }

  .ice-nav {
    margin-top: 4px;
  }

  .mode-dot {
    margin-left: auto;
    width: 8px;
    height: 8px;
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

  .playlists-head {
    margin: 22px 8px 8px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--p-text-dim);
  }

  .icon-add {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--p-text);
    font-size: 15px;
    line-height: 1;
  }

  .icon-add:hover {
    background: rgba(255, 255, 255, 0.14);
  }

  .create-form {
    display: flex;
    gap: 6px;
    margin: 4px 4px 10px;
  }

  .create-form input {
    flex: 1;
    min-width: 0;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--p-border);
    border-radius: 8px;
    padding: 6px 8px;
    font-size: 12px;
    outline: none;
  }

  .create-form button {
    background: var(--p-accent);
    color: #090c14;
    border-radius: 8px;
    padding: 0 10px;
    font-size: 12px;
    font-weight: 600;
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
    padding: 8px;
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .pl-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border-radius: 10px;
    font-size: 13px;
    color: var(--p-text-muted);
    text-align: left;
  }

  .pl-item:hover {
    background: rgba(255, 255, 255, 0.05);
    color: var(--p-text);
  }

  .pl-item.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
  }

  .pl-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pl-count {
    font-size: 11px;
    color: var(--p-text-dim);
  }

  .ice-card {
    margin-top: 12px;
    padding: 12px;
    border-radius: 14px;
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

  .ice-device {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 3px;
  }

  .ice-hint {
    font-size: 11px;
    color: var(--p-text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
