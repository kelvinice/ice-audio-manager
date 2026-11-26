<script lang="ts">
  import { resolveArtworkUrl } from "../lib/theme";
  import type { LibraryState, Track } from "../lib/types";

  interface Props {
    library: LibraryState;
    selectedPlaylistId: string | null;
    currentTrackId: string | null;
    proxyPort: number;
    onPlay: (track: Track, list: Track[]) => void;
    onToggleLike: (track: Track) => void;
    onAddFolder: (path: string) => void;
    onRemoveFolder: (path: string) => void;
    onRescan: () => void;
    onDeleteCached: (trackId: string) => void;
    onClearCache: () => void;
    onDeletePlaylist: (playlistId: string) => void;
    onRemoveFromPlaylist: (playlistId: string, trackId: string) => void;
  }

  let {
    library,
    selectedPlaylistId,
    currentTrackId,
    proxyPort,
    onPlay,
    onToggleLike,
    onAddFolder,
    onRemoveFolder,
    onRescan,
    onDeleteCached,
    onClearCache,
    onDeletePlaylist,
    onRemoveFromPlaylist,
  }: Props = $props();

  let tab = $state<"liked" | "local" | "offline" | "history">("liked");
  let folderInput = $state("");

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

  function formatMb(bytes: number): string {
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
</script>

<div class="page">
  {#if activePlaylist}
    <div class="header-row">
      <div>
        <div class="pill-badge">Playlist</div>
        <h1>{activePlaylist.name}</h1>
        <p class="sub">{playlistTracks.length} tracks</p>
      </div>
      <div class="head-actions">
        {#if playlistTracks.length > 0}
          <button class="btn-primary" onclick={() => onPlay(playlistTracks[0], playlistTracks)}>
            ▶ Play All
          </button>
        {/if}
        <button class="btn-ghost danger" onclick={() => onDeletePlaylist(activePlaylist.id)}>
          Delete Playlist
        </button>
      </div>
    </div>

    <div class="list card-glass">
      {#if playlistTracks.length === 0}
        <div class="empty">Add songs to this playlist from Search.</div>
      {:else}
        {#each playlistTracks as track (track.id)}
          <div class="row" class:playing={currentTrackId === track.id}>
            <button class="row-main" onclick={() => onPlay(track, playlistTracks)}>
              <div class="thumb">
                {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
                  <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
                {:else}
                  <span>♫</span>
                {/if}
              </div>
              <div class="meta">
                <div class="title">{track.title}</div>
                <div class="sub">{track.artist}</div>
              </div>
            </button>
            <button
              class="btn-ghost"
              onclick={() => onRemoveFromPlaylist(activePlaylist.id, track.id)}
            >
              Remove
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
      <div class="tabs">
        <button class="chip" class:active={tab === "liked"} onclick={() => (tab = "liked")}>
          Liked ({library.liked.length})
        </button>
        <button class="chip" class:active={tab === "local"} onclick={() => (tab = "local")}>
          Local Files ({library.localTracks.length})
        </button>
        <button class="chip" class:active={tab === "offline"} onclick={() => (tab = "offline")}>
          Offline Cache ({library.cachedIds.length})
        </button>
        <button class="chip" class:active={tab === "history"} onclick={() => (tab = "history")}>
          History ({library.history.length})
        </button>
      </div>
    </div>

    {#if tab === "liked"}
      <div class="list card-glass">
        {#if library.liked.length === 0}
          <div class="empty">No liked songs yet.</div>
        {:else}
          {#each library.liked as track (track.id)}
            <div class="row" class:playing={currentTrackId === track.id}>
              <button class="row-main" onclick={() => onPlay(track, library.liked)}>
                <div class="thumb">
                  {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
                    <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
                  {:else}
                    <span>♫</span>
                  {/if}
                </div>
                <div class="meta">
                  <div class="title">{track.title}</div>
                  <div class="sub">{track.artist}</div>
                </div>
              </button>
              <button class="btn-ghost" onclick={() => onToggleLike(track)}>♥ Unlike</button>
            </div>
          {/each}
        {/if}
      </div>
    {:else if tab === "local"}
      <div class="card-glass folder-panel">
        <form class="folder-form" onsubmit={submitFolder}>
          <input
            type="text"
            placeholder="Paste Windows music folder path (e.g. C:\Users\you\Music)"
            bind:value={folderInput}
          />
          <button type="submit" class="btn-primary">Add Folder</button>
          <button type="button" class="btn-ghost" onclick={onRescan}>Rescan</button>
        </form>
        {#if library.localFolders.length > 0}
          <div class="folders">
            {#each library.localFolders as folder (folder)}
              <div class="folder-chip">
                <span>{folder}</span>
                <button onclick={() => onRemoveFolder(folder)}>×</button>
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
            <div class="row" class:playing={currentTrackId === track.id}>
              <button class="row-main" onclick={() => onPlay(track, library.localTracks)}>
                <div class="thumb"><span>♪</span></div>
                <div class="meta">
                  <div class="title">{track.title}</div>
                  <div class="sub">{track.artist} • {track.album}</div>
                </div>
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
        <button class="btn-ghost danger" onclick={onClearCache}>Clear Cache</button>
      </div>
      <div class="list card-glass">
        {#if cachedTracks.length === 0}
          <div class="empty">Download tracks from Search using the ↓ button for offline playback.</div>
        {:else}
          {#each cachedTracks as track (track.id)}
            <div class="row" class:playing={currentTrackId === track.id}>
              <button class="row-main" onclick={() => onPlay(track, cachedTracks)}>
                <div class="thumb">
                  {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
                    <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
                  {:else}
                    <span>✓</span>
                  {/if}
                </div>
                <div class="meta">
                  <div class="title">{track.title}</div>
                  <div class="sub">{track.artist}</div>
                </div>
              </button>
              <button class="btn-ghost" onclick={() => onDeleteCached(track.id)}>Delete</button>
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
            <div class="row" class:playing={currentTrackId === track.id}>
              <button class="row-main" onclick={() => onPlay(track, library.history)}>
                <div class="thumb">
                  {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
                    <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
                  {:else}
                    <span>♫</span>
                  {/if}
                </div>
                <div class="meta">
                  <div class="title">{track.title}</div>
                  <div class="sub">{track.artist}</div>
                </div>
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
  .head-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .chip {
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
    gap: 12px;
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

  .danger {
    color: #fca5a5;
  }
</style>
