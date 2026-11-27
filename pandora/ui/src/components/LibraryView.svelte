<script lang="ts">
  import { formatCount, formatMb, formatTotalDuration } from "../lib/format";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { LibraryState, Playlist, Track } from "../lib/types";
  import ConfirmModal from "./ConfirmModal.svelte";
  import CoverTile from "./CoverTile.svelte";
  import Icon from "./Icon.svelte";
  import MoreMenu from "./MoreMenu.svelte";
  import PageHero from "./PageHero.svelte";
  import PlaylistRecommendations from "./PlaylistRecommendations.svelte";
  import TrackTable from "./TrackTable.svelte";

  type LibraryTab = "liked" | "playlists" | "local" | "offline" | "history";

  interface Props {
    library: LibraryState;
    selectedPlaylistId: string | null;
    tab: LibraryTab;
    likedIds: Set<string>;
    cachedIds: Set<string>;
    currentTrackId: string | null;
    isPlaying: boolean;
    proxyPort: number;
    onTabChange: (tab: LibraryTab) => void;
    onPlay: (track: Track, list: Track[]) => void;
    onPlayShuffled: (list: Track[]) => void;
    onToggleLike: (track: Track) => void;
    onAddFolder: (path: string) => void;
    onRemoveFolder: (path: string) => void;
    onRescan: () => void;
    onDownload: (track: Track) => void;
    onDownloadAll: (tracks: Track[]) => void;
    onDeleteCached: (trackId: string) => void;
    onClearCache: () => void;
    onSelectPlaylist: (playlistId: string | null) => void;
    onOpenNewPlaylist: () => void;
    onOpenImportPlaylist: () => void;
    onOpenPlaylistPicker: (track: Track) => void;
    onRenamePlaylist: (playlistId: string, name: string) => Promise<void>;
    onDeletePlaylist: (playlistId: string) => void;
    onRemoveFromPlaylist: (playlistId: string, trackId: string) => void;
    onAddToThisPlaylist: (playlistId: string, track: Track) => void;
  }

  let {
    library,
    selectedPlaylistId,
    tab,
    likedIds,
    cachedIds,
    currentTrackId,
    isPlaying,
    proxyPort,
    onTabChange,
    onPlay,
    onPlayShuffled,
    onToggleLike,
    onAddFolder,
    onRemoveFolder,
    onRescan,
    onDownload,
    onDownloadAll,
    onDeleteCached,
    onClearCache,
    onSelectPlaylist,
    onOpenNewPlaylist,
    onOpenImportPlaylist,
    onOpenPlaylistPicker,
    onRenamePlaylist,
    onDeletePlaylist,
    onRemoveFromPlaylist,
    onAddToThisPlaylist,
  }: Props = $props();

  const TABS: { id: LibraryTab; label: string }[] = [
    { id: "playlists", label: "Playlists" },
    { id: "liked", label: "Favorites" },
    { id: "offline", label: "Downloads" },
    { id: "local", label: "Local files" },
    { id: "history", label: "History" },
  ];

  let folderInput = $state("");
  let renaming = $state(false);
  let renameInput = $state("");
  let pendingDeletePlaylist = $state<Playlist | null>(null);

  $effect(() => {
    void selectedPlaylistId;
    renaming = false;
  });

  let activePlaylist = $derived(
    selectedPlaylistId ? (library.playlists.find((p) => p.id === selectedPlaylistId) ?? null) : null,
  );
  let playlistTracks = $derived(activePlaylist ? tracksFor(activePlaylist) : []);
  let cachedTracks = $derived(library.cachedIds.map((id) => library.tracksIndex[id]).filter((t): t is Track => Boolean(t)));

  function tracksFor(playlist: Playlist): Track[] {
    return playlist.trackIds.map((id) => library.tracksIndex[id]).filter((t): t is Track => Boolean(t));
  }

  function coversFor(trackIds: string[]): string[] {
    const out: string[] = [];
    for (const id of trackIds) {
      const url = resolveArtworkUrl(library.tracksIndex[id]?.artworkUrl ?? null, proxyPort);
      if (url && !out.includes(url)) out.push(url);
      if (out.length === 4) break;
    }
    return out.length >= 4 ? out : out.slice(0, 1);
  }

  function count(id: LibraryTab): number {
    switch (id) {
      case "liked":
        return library.liked.length;
      case "playlists":
        return library.playlists.length;
      case "local":
        return library.localTracks.length;
      case "offline":
        return library.cachedIds.length;
      default:
        return library.history.length;
    }
  }

  function startRename() {
    if (!activePlaylist) return;
    renameInput = activePlaylist.name;
    renaming = true;
  }

  async function submitRename(e: Event) {
    e.preventDefault();
    if (!activePlaylist || !renameInput.trim()) return;
    await onRenamePlaylist(activePlaylist.id, renameInput.trim());
    renaming = false;
  }

  function submitFolder(e: Event) {
    e.preventDefault();
    if (!folderInput.trim()) return;
    onAddFolder(folderInput.trim());
    folderInput = "";
  }
</script>

{#snippet table(tracks: Track[], emptyText: string, extra: { showAlbum?: boolean; onRemove?: (t: Track) => void; removeLabel?: string } = {})}
  <TrackTable
    {tracks}
    {likedIds}
    {cachedIds}
    {currentTrackId}
    {isPlaying}
    {proxyPort}
    {emptyText}
    showAlbum={extra.showAlbum ?? true}
    {onPlay}
    {onToggleLike}
    onAddToPlaylist={onOpenPlaylistPicker}
    {onDownload}
    onRemove={extra.onRemove}
    removeLabel={extra.removeLabel}
  />
{/snippet}

{#snippet playBar(tracks: Track[])}
  {#if tracks.length > 0}
    <div class="play-bar">
      <button class="btn-play" aria-label="Play all" onclick={() => onPlay(tracks[0], tracks)}>
        <Icon name="play" size={22} />
      </button>
      <button class="btn-icon" title="Shuffle" aria-label="Shuffle" onclick={() => onPlayShuffled(tracks)}>
        <Icon name="shuffle" size={17} />
      </button>
    </div>
  {/if}
{/snippet}

{#if activePlaylist}
  {@const playlist = activePlaylist}

  {#snippet renameForm()}
    <form class="rename" onsubmit={submitRename}>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" bind:value={renameInput} autofocus aria-label="Playlist name" />
      <button type="submit" class="btn-primary">Save</button>
      <button type="button" class="btn-ghost" onclick={() => (renaming = false)}>Cancel</button>
    </form>
  {/snippet}

  <PageHero
    backLabel="Your Library"
    onBack={() => onSelectPlaylist(null)}
    covers={coversFor(playlist.trackIds)}
    label="Playlist"
    title={playlist.name}
    meta={[formatCount(playlistTracks.length, "song"), formatTotalDuration(playlistTracks)].filter(Boolean).join(" • ")}
    titleSlot={renaming ? renameForm : undefined}
  >
    <button class="btn-play" aria-label="Play" disabled={playlistTracks.length === 0} onclick={() => onPlay(playlistTracks[0], playlistTracks)}>
      <Icon name="play" size={22} />
    </button>
    <button class="btn-icon" title="Shuffle" aria-label="Shuffle" disabled={playlistTracks.length === 0} onclick={() => onPlayShuffled(playlistTracks)}>
      <Icon name="shuffle" size={17} />
    </button>
    <MoreMenu
      items={[
        { label: "Rename", icon: "edit", onSelect: startRename },
        {
          label: "Download songs",
          icon: "download",
          disabled: playlistTracks.every((t) => t.source !== "ytmusic" || cachedIds.has(t.id)),
          onSelect: () => onDownloadAll(playlistTracks.filter((t) => t.source === "ytmusic" && !cachedIds.has(t.id))),
        },
        { label: "Delete playlist", icon: "trash", danger: true, onSelect: () => (pendingDeletePlaylist = playlist) },
      ]}
    />
  </PageHero>

  <section class="body">
    {@render table(playlistTracks, "This playlist is empty. Add songs from Home, Search or Identify.", {
      onRemove: (t) => onRemoveFromPlaylist(playlist.id, t.id),
      removeLabel: "Remove from this playlist",
    })}
    {#key playlist.id}
      <PlaylistRecommendations
        tracks={playlistTracks}
        {likedIds}
        {currentTrackId}
        {isPlaying}
        {proxyPort}
        {onPlay}
        {onToggleLike}
        onAdd={(track) => onAddToThisPlaylist(playlist.id, track)}
      />
    {/key}
  </section>
{:else}
  <div class="page">
    <header class="top">
      <h1>Your Library</h1>
      <div class="top-actions">
        <button class="btn-ghost" onclick={onOpenImportPlaylist}>
          <Icon name="link" size={14} />
          <span>Import</span>
        </button>
        <button class="btn-primary" onclick={onOpenNewPlaylist}>
          <Icon name="plus" size={15} />
          <span>New playlist</span>
        </button>
      </div>
    </header>

    <nav class="tabs" aria-label="Library sections">
      {#each TABS as item (item.id)}
        <button class="tab" class:active={tab === item.id} aria-current={tab === item.id} onclick={() => onTabChange(item.id)}>
          {item.label}
          <span class="n">{count(item.id)}</span>
        </button>
      {/each}
    </nav>

    {#if tab === "playlists"}
      {#if library.playlists.length === 0}
        <p class="empty">No playlists yet. Create one, import a link, or save one from Home.</p>
      {:else}
        <div class="grid">
          {#each library.playlists as pl (pl.id)}
            {@const tracks = tracksFor(pl)}
            <CoverTile
              title={pl.name}
              subtitle={formatCount(tracks.length, "song")}
              covers={coversFor(pl.trackIds)}
              onOpen={() => onSelectPlaylist(pl.id)}
              onPlay={tracks.length > 0 ? () => onPlay(tracks[0], tracks) : undefined}
            />
          {/each}
        </div>
      {/if}
    {:else if tab === "liked"}
      {@render playBar(library.liked)}
      {@render table(library.liked, "Songs you favourite show up here.")}
    {:else if tab === "offline"}
      <div class="panel">
        <span><strong>{formatCount(library.cachedIds.length, "song")}</strong> downloaded • {formatMb(library.cacheBytes)}</span>
        <button class="btn-ghost danger" disabled={library.cachedIds.length === 0} onclick={onClearCache}>
          <Icon name="trash" size={14} />
          <span>Remove all</span>
        </button>
      </div>
      {@render playBar(cachedTracks)}
      {@render table(cachedTracks, "Download songs from any list to play them offline.", {
        onRemove: (t) => onDeleteCached(t.id),
        removeLabel: "Remove download",
      })}
    {:else if tab === "local"}
      <div class="panel column">
        <form class="folder-form" onsubmit={submitFolder}>
          <input type="text" placeholder="Music folder path, e.g. C:\Users\you\Music" bind:value={folderInput} />
          <button type="submit" class="btn-primary">Add folder</button>
          <button type="button" class="btn-ghost" onclick={onRescan}>
            <Icon name="refresh" size={14} />
            <span>Rescan</span>
          </button>
        </form>
        {#if library.localFolders.length > 0}
          <div class="folders">
            {#each library.localFolders as folder (folder)}
              <span class="folder">
                <Icon name="folder" size={13} />
                <span>{folder}</span>
                <button class="btn-icon sm" aria-label={`Remove ${folder}`} onclick={() => onRemoveFolder(folder)}>
                  <Icon name="x" size={12} />
                </button>
              </span>
            {/each}
          </div>
        {/if}
      </div>
      {@render playBar(library.localTracks)}
      {@render table(library.localTracks, "Add a music folder to browse your own files.")}
    {:else}
      {@render table(library.history, "Songs you play will show up here.")}
    {/if}
  </div>
{/if}

<ConfirmModal
  open={Boolean(pendingDeletePlaylist)}
  title="Delete playlist?"
  message={pendingDeletePlaylist
    ? `Delete "${pendingDeletePlaylist.name}" (${formatCount(pendingDeletePlaylist.trackIds.length, "song")})? The songs stay in Favorites and Local files.`
    : ""}
  confirmLabel="Delete"
  onCancel={() => (pendingDeletePlaylist = null)}
  onConfirm={() => {
    const target = pendingDeletePlaylist;
    pendingDeletePlaylist = null;
    if (target) onDeletePlaylist(target.id);
  }}
/>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 28px 24px 48px;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 8px;
  }

  h1 {
    margin: 0;
    font-size: 30px;
    font-weight: 800;
    letter-spacing: -0.03em;
  }

  .top-actions,
  .play-bar {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .play-bar {
    gap: 14px;
    padding: 0 8px;
  }

  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 0 8px;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--p-fill);
    font-size: 13px;
    font-weight: 600;
    transition: background 0.14s ease, color 0.14s ease;
  }

  .tab:hover {
    background: color-mix(in srgb, var(--p-text) 14%, var(--p-fill));
  }

  .tab.active {
    background: var(--p-text);
    color: var(--p-bg);
  }

  .n {
    font-size: 11.5px;
    font-weight: 600;
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 4px;
  }

  .empty {
    margin: 0;
    padding: 24px 8px;
    color: var(--p-text-dim);
    font-size: 13px;
  }

  .panel {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin: 0 8px;
    padding: 14px 16px;
    border-radius: 12px;
    background: var(--p-fill);
    font-size: 13px;
    color: var(--p-text-muted);
  }

  .panel.column {
    flex-direction: column;
    align-items: stretch;
  }

  .panel strong {
    color: var(--p-text);
  }

  .folder-form {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .folder-form input,
  .rename input {
    flex: 1;
    min-width: 220px;
    height: 36px;
    padding: 0 14px;
    border-radius: 999px;
    border: 1px solid var(--p-border);
    background: var(--p-surface-solid);
    font-size: 13px;
    outline: none;
  }

  .folder-form input:focus,
  .rename input:focus {
    border-color: var(--p-accent);
  }

  .folders {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .folder {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    padding: 2px 2px 2px 10px;
    border-radius: 999px;
    background: var(--p-surface-solid);
    font-size: 12px;
  }

  .folder span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rename {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    max-width: 560px;
  }

  .body {
    padding: 0 24px 48px;
  }
</style>
