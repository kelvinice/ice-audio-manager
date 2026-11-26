<script lang="ts">
  import { resolveArtworkUrl } from "../lib/theme";
  import type { LibraryState, Track } from "../lib/types";
  import Icon from "./Icon.svelte";

  interface Props {
    library: LibraryState;
    selectedPlaylistId: string | null;
    tab: "liked" | "playlists" | "local" | "offline" | "history";
    likedIds: Set<string>;
    currentTrackId: string | null;
    proxyPort: number;
    onTabChange: (tab: "liked" | "playlists" | "local" | "offline" | "history") => void;
    onPlay: (track: Track, list: Track[]) => void;
    onPlayShuffled: (list: Track[]) => void;
    onToggleLike: (track: Track) => void;
    onAddFolder: (path: string) => void;
    onRemoveFolder: (path: string) => void;
    onRescan: () => void;
    onDeleteCached: (trackId: string) => void;
    onClearCache: () => void;
    onSelectPlaylist: (playlistId: string | null) => void;
    onOpenNewPlaylist: () => void;
    onOpenPlaylistPicker: (track: Track) => void;
    onRenamePlaylist: (playlistId: string, name: string) => Promise<void>;
    onDeletePlaylist: (playlistId: string) => void;
    onRemoveFromPlaylist: (playlistId: string, trackId: string) => void;
  }

  let {
    library,
    selectedPlaylistId,
    tab,
    likedIds,
    currentTrackId,
    proxyPort,
    onTabChange,
    onPlay,
    onPlayShuffled,
    onToggleLike,
    onAddFolder,
    onRemoveFolder,
    onRescan,
    onDeleteCached,
    onClearCache,
    onSelectPlaylist,
    onOpenNewPlaylist,
    onOpenPlaylistPicker,
    onRenamePlaylist,
    onDeletePlaylist,
    onRemoveFromPlaylist,
  }: Props = $props();

  let folderInput = $state("");
  let renaming = $state(false);
  let renameInput = $state("");

  let activePlaylist = $derived(
    selectedPlaylistId ? library.playlists.find((p) => p.id === selectedPlaylistId) ?? null : null,
  );

  let playlistTracks = $derived.by(() => {
    if (!activePlaylist) return [];
    return activePlaylist.trackIds
      .map((id) => library.tracksIndex[id])
      .filter((t): t is Track => Boolean(t));
  });

  let cachedTracks = $derived.by(() =>
    library.cachedIds
      .map((id) => library.tracksIndex[id])
      .filter((t): t is Track => Boolean(t)),
  );

  function submitFolder(e: Event) {
    e.preventDefault();
    if (!folderInput.trim()) return;
    onAddFolder(folderInput.trim());
    folderInput = "";
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

  function playlistCover(trackIds: string[]): string | null {
    for (const id of trackIds) {
      const url = resolveArtworkUrl(library.tracksIndex[id]?.artworkUrl ?? null, proxyPort);
      if (url) return url;
    }
    return null;
  }

  function formatMb(bytes: number): string {
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
</script>

<div class="page">
  {#if activePlaylist}
    <div class="header-row">
      <div>
        <div class="pill-badge">Playlist</div>
        {#if renaming}
          <form class="rename-form" onsubmit={submitRename}>
            <input type="text" bind:value={renameInput} />
            <button type="submit" class="btn-primary">Save</button>
            <button type="button" class="btn-ghost" onclick={() => (renaming = false)}>Cancel</button>
          </form>
        {:else}
          <h1>{activePlaylist.name}</h1>
        {/if}
        <p class="sub">{playlistTracks.length} tracks</p>
      </div>
      <div class="head-actions">
        {#if playlistTracks.length > 0}
          <button class="btn-primary" onclick={() => onPlay(playlistTracks[0], playlistTracks)}>
            <Icon name="play" size={14} />
            <span>Play All</span>
          </button>
          <button class="btn-ghost" onclick={() => onPlayShuffled(playlistTracks)}>
            <Icon name="shuffle" size={14} />
            <span>Shuffle</span>
          </button>
        {/if}
        {#if !renaming}
          <button class="btn-ghost" onclick={startRename}>
            <Icon name="edit" size={14} />
            <span>Rename</span>
          </button>
        {/if}
        <button class="btn-ghost danger" onclick={() => onDeletePlaylist(activePlaylist.id)}>
          <Icon name="trash" size={14} />
          <span>Delete</span>
        </button>
      </div>
    </div>

    <div class="list card-glass">
      {#if playlistTracks.length === 0}
        <div class="empty">Add songs to this playlist using the Add to Playlist button anywhere in Pandora.</div>
      {:else}
        {#each playlistTracks as track (track.id)}
          {@const isLiked = likedIds.has(track.id)}
          <div class="row" class:playing={currentTrackId === track.id}>
            <button class="row-main" onclick={() => onPlay(track, playlistTracks)}>
              <div class="thumb">
                {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
                  <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
                {:else}
                  <Icon name="music" size={18} />
                {/if}
              </div>
              <div class="meta">
                <div class="title">{track.title}</div>
                <div class="sub">{track.artist}</div>
              </div>
            </button>
            <button
              class="act"
              class:liked={isLiked}
              title={isLiked ? "Remove from Favorites" : "Add to Favorites"}
              aria-label={isLiked ? "Remove from Favorites" : "Add to Favorites"}
              onclick={() => onToggleLike(track)}
            >
              <Icon name={isLiked ? "heart-filled" : "heart"} size={14} />
            </button>
            <button
              class="act"
              title="Remove from playlist"
              aria-label="Remove from playlist"
              onclick={() => onRemoveFromPlaylist(activePlaylist.id, track.id)}
            >
              <Icon name="x" size={14} />
            </button>
          </div>
        {/each}
      {/if}
    </div>
  {:else}
    <div class="header-row">
      <div>
        <h1>Your Library</h1>
        <p class="sub">Privacy-first local database in %APPDATA%\Pandora\library.json</p>
      </div>
      <div class="head-actions">
        <button class="btn-primary" onclick={onOpenNewPlaylist}>
          <Icon name="plus" size={14} />
          <span>New Playlist</span>
        </button>
      </div>
    </div>

    <div class="tabs">
      <button class="chip" class:active={tab === "liked"} onclick={() => onTabChange("liked")}>
        <Icon name="heart-filled" size={13} />
        <span>Favorites ({library.liked.length})</span>
      </button>
      <button class="chip" class:active={tab === "playlists"} onclick={() => onTabChange("playlists")}>
        <Icon name="list-music" size={13} />
        <span>Playlists ({library.playlists.length})</span>
      </button>
      <button class="chip" class:active={tab === "local"} onclick={() => onTabChange("local")}>
        <Icon name="folder" size={13} />
        <span>Local Files ({library.localTracks.length})</span>
      </button>
      <button class="chip" class:active={tab === "offline"} onclick={() => onTabChange("offline")}>
        <Icon name="download" size={13} />
        <span>Offline Cache ({library.cachedIds.length})</span>
      </button>
      <button class="chip" class:active={tab === "history"} onclick={() => onTabChange("history")}>
        <Icon name="music" size={13} />
        <span>History ({library.history.length})</span>
      </button>
    </div>

    {#if tab === "liked"}
      {#if library.liked.length > 0}
        <div class="bar-actions">
          <button class="btn-primary" onclick={() => onPlay(library.liked[0], library.liked)}>
            <Icon name="play" size={14} />
            <span>Play All Favorites</span>
          </button>
          <button class="btn-ghost" onclick={() => onPlayShuffled(library.liked)}>
            <Icon name="shuffle" size={14} />
            <span>Shuffle</span>
          </button>
        </div>
      {/if}
      <div class="list card-glass">
        {#if library.liked.length === 0}
          <div class="empty">No favorites yet — click the heart icon on any track to save it here.</div>
        {:else}
          {#each library.liked as track (track.id)}
            <div class="row" class:playing={currentTrackId === track.id}>
              <button class="row-main" onclick={() => onPlay(track, library.liked)}>
                <div class="thumb">
                  {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
                    <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
                  {:else}
                    <Icon name="music" size={18} />
                  {/if}
                </div>
                <div class="meta">
                  <div class="title">{track.title}</div>
                  <div class="sub">{track.artist}</div>
                </div>
              </button>
              <button
                class="act"
                title="Add to Playlist"
                aria-label="Add to Playlist"
                onclick={() => onOpenPlaylistPicker(track)}
              >
                <Icon name="list-plus" size={14} />
              </button>
              <button
                class="act liked"
                title="Remove from Favorites"
                aria-label="Remove from Favorites"
                onclick={() => onToggleLike(track)}
              >
                <Icon name="heart-filled" size={14} />
              </button>
            </div>
          {/each}
        {/if}
      </div>
    {:else if tab === "playlists"}
      {#if library.playlists.length === 0}
        <div class="list card-glass">
          <div class="empty">
            No playlists yet. Click <strong>New Playlist</strong> above to create your first one.
          </div>
        </div>
      {:else}
        <div class="pl-grid">
          {#each library.playlists as pl (pl.id)}
            {@const cover = playlistCover(pl.trackIds)}
            {@const tracks = pl.trackIds.map((id) => library.tracksIndex[id]).filter((t): t is Track => Boolean(t))}
            <div class="pl-card card-glass">
              <button class="pl-cover-btn" onclick={() => onSelectPlaylist(pl.id)}>
                <div class="pl-cover">
                  {#if cover}
                    <img src={cover} alt={pl.name} />
                  {:else}
                    <Icon name="list-music" size={28} />
                  {/if}
                </div>
                <div class="pl-card-title">{pl.name}</div>
                <div class="pl-card-sub">{tracks.length} tracks</div>
              </button>
              <div class="pl-card-actions">
                <button
                  class="btn-ghost"
                  disabled={tracks.length === 0}
                  onclick={() => tracks.length > 0 && onPlay(tracks[0], tracks)}
                >
                  <Icon name="play" size={13} />
                  <span>Play</span>
                </button>
                <button
                  class="act danger"
                  title="Delete playlist"
                  aria-label="Delete playlist"
                  onclick={() => onDeletePlaylist(pl.id)}
                >
                  <Icon name="trash" size={14} />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {:else if tab === "local"}
      <div class="card-glass folder-panel">
        <form class="folder-form" onsubmit={submitFolder}>
          <input
            type="text"
            placeholder="Paste Windows music folder path (e.g. C:\Users\you\Music)"
            bind:value={folderInput}
          />
          <button type="submit" class="btn-primary">Add Folder</button>
          <button type="button" class="btn-ghost" onclick={onRescan}>
            <Icon name="refresh" size={14} />
            <span>Rescan</span>
          </button>
        </form>
        {#if library.localFolders.length > 0}
          <div class="folders">
            {#each library.localFolders as folder (folder)}
              <div class="folder-chip">
                <Icon name="folder" size={13} />
                <span>{folder}</span>
                <button aria-label="Remove folder" onclick={() => onRemoveFolder(folder)}>
                  <Icon name="x" size={12} />
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="list card-glass">
        {#if library.localTracks.length === 0}
          <div class="empty">Add a Windows folder containing MP3, FLAC, WAV, M4A, or OGG files.</div>
        {:else}
          {#each library.localTracks as track (track.id)}
            {@const isLiked = likedIds.has(track.id)}
            <div class="row" class:playing={currentTrackId === track.id}>
              <button class="row-main" onclick={() => onPlay(track, library.localTracks)}>
                <div class="thumb"><Icon name="music" size={18} /></div>
                <div class="meta">
                  <div class="title">{track.title}</div>
                  <div class="sub">{track.artist} • {track.album}</div>
                </div>
              </button>
              <button
                class="act"
                class:liked={isLiked}
                title={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                aria-label={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                onclick={() => onToggleLike(track)}
              >
                <Icon name={isLiked ? "heart-filled" : "heart"} size={14} />
              </button>
              <button
                class="act"
                title="Add to Playlist"
                aria-label="Add to Playlist"
                onclick={() => onOpenPlaylistPicker(track)}
              >
                <Icon name="list-plus" size={14} />
              </button>
            </div>
          {/each}
        {/if}
      </div>
    {:else if tab === "offline"}
      <div class="card-glass cache-bar">
        <div>
          <strong>{library.cachedIds.length}</strong> offline tracks cached •
          <strong>{formatMb(library.cacheBytes)}</strong>
        </div>
        <button class="btn-ghost danger" onclick={onClearCache}>
          <Icon name="trash" size={14} />
          <span>Clear Cache</span>
        </button>
      </div>
      <div class="list card-glass">
        {#if cachedTracks.length === 0}
          <div class="empty">Download tracks from Search for offline playback.</div>
        {:else}
          {#each cachedTracks as track (track.id)}
            {@const isLiked = likedIds.has(track.id)}
            <div class="row" class:playing={currentTrackId === track.id}>
              <button class="row-main" onclick={() => onPlay(track, cachedTracks)}>
                <div class="thumb">
                  {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
                    <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
                  {:else}
                    <Icon name="check" size={18} />
                  {/if}
                </div>
                <div class="meta">
                  <div class="title">{track.title}</div>
                  <div class="sub">{track.artist}</div>
                </div>
              </button>
              <button
                class="act"
                class:liked={isLiked}
                title={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                aria-label={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                onclick={() => onToggleLike(track)}
              >
                <Icon name={isLiked ? "heart-filled" : "heart"} size={14} />
              </button>
              <button
                class="act"
                title="Add to Playlist"
                aria-label="Add to Playlist"
                onclick={() => onOpenPlaylistPicker(track)}
              >
                <Icon name="list-plus" size={14} />
              </button>
              <button
                class="act danger"
                title="Delete cached file"
                aria-label="Delete cached file"
                onclick={() => onDeleteCached(track.id)}
              >
                <Icon name="trash" size={14} />
              </button>
            </div>
          {/each}
        {/if}
      </div>
    {:else}
      <div class="list card-glass">
        {#if library.history.length === 0}
          <div class="empty">Your local playback history will appear here.</div>
        {:else}
          {#each library.history as track (track.id)}
            {@const isLiked = likedIds.has(track.id)}
            <div class="row" class:playing={currentTrackId === track.id}>
              <button class="row-main" onclick={() => onPlay(track, library.history)}>
                <div class="thumb">
                  {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
                    <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
                  {:else}
                    <Icon name="music" size={18} />
                  {/if}
                </div>
                <div class="meta">
                  <div class="title">{track.title}</div>
                  <div class="sub">{track.artist}</div>
                </div>
              </button>
              <button
                class="act"
                class:liked={isLiked}
                title={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                aria-label={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                onclick={() => onToggleLike(track)}
              >
                <Icon name={isLiked ? "heart-filled" : "heart"} size={14} />
              </button>
              <button
                class="act"
                title="Add to Playlist"
                aria-label="Add to Playlist"
                onclick={() => onOpenPlaylistPicker(track)}
              >
                <Icon name="list-plus" size={14} />
              </button>
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .page {
    padding: 24px 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .header-row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
  }

  h1 {
    margin: 4px 0 4px;
    font-size: 24px;
  }

  .sub {
    margin: 0;
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .tabs,
  .head-actions,
  .bar-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .rename-form {
    display: flex;
    gap: 8px;
    margin: 6px 0;
  }

  .rename-form input {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--p-border-strong);
    border-radius: 10px;
    padding: 6px 12px;
    font-size: 16px;
    font-weight: 600;
    outline: none;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--p-border);
    color: var(--p-text-muted);
  }

  .chip.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    border-color: var(--p-accent);
  }

  .pl-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 14px;
  }

  .pl-card {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .pl-cover-btn {
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .pl-cover {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 10px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.06);
    display: grid;
    place-items: center;
    font-size: 28px;
    color: var(--p-text-dim);
  }

  .pl-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .pl-card-title {
    font-size: 14px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pl-card-sub {
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .pl-card-actions {
    display: flex;
    justify-content: space-between;
    gap: 6px;
  }

  .folder-panel,
  .cache-bar {
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .cache-bar {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }

  .folder-form {
    display: flex;
    gap: 10px;
  }

  .folder-form input {
    flex: 1;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--p-border);
    border-radius: 999px;
    padding: 9px 16px;
    font-size: 13px;
    outline: none;
  }

  .folders {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .folder-chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    border-radius: 8px;
    font-size: 12px;
    background: rgba(255, 255, 255, 0.07);
  }

  .list {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .empty {
    padding: 32px 20px;
    text-align: center;
    color: var(--p-text-muted);
    font-size: 13px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-radius: 12px;
  }

  .row:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .row.playing {
    background: var(--p-accent-soft);
  }

  .row-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
  }

  .thumb {
    width: 42px;
    height: 42px;
    border-radius: 8px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.08);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .meta {
    min-width: 0;
  }

  .title {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .act {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    color: var(--p-text-muted);
    background: rgba(255, 255, 255, 0.05);
  }

  .act:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--p-text);
  }

  .act.liked {
    color: #fb7185;
  }

  .danger {
    color: #fca5a5;
  }
</style>
