<script lang="ts">
  import { resolveArtworkUrl } from "../lib/theme";
  import type { LibraryState, Playlist, Track } from "../lib/types";
  import ConfirmModal from "./ConfirmModal.svelte";
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
    onOpenImportPlaylist: () => void;
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
    onOpenImportPlaylist,
    onOpenPlaylistPicker,
    onRenamePlaylist,
    onDeletePlaylist,
    onRemoveFromPlaylist,
  }: Props = $props();

  let folderInput = $state("");
  let renaming = $state(false);
  let renameInput = $state("");
  let playlistQuery = $state("");
  let pendingDeletePlaylist = $state<Playlist | null>(null);

  $effect(() => {
    void selectedPlaylistId;
    playlistQuery = "";
    renaming = false;
  });

  let activePlaylist = $derived(
    selectedPlaylistId ? library.playlists.find((p) => p.id === selectedPlaylistId) ?? null : null,
  );

  let playlistTracks = $derived.by(() => {
    if (!activePlaylist) return [];
    return activePlaylist.trackIds
      .map((id) => library.tracksIndex[id])
      .filter((t): t is Track => Boolean(t));
  });

  let filteredPlaylistTracks = $derived.by(() => {
    const q = playlistQuery.trim().toLowerCase();
    if (!q) return playlistTracks;
    return playlistTracks.filter(
      (t) =>
        t.title.toLowerCase().includes(q) ||
        t.artist.toLowerCase().includes(q) ||
        t.album.toLowerCase().includes(q),
    );
  });

  let heroCovers = $derived(
    activePlaylist ? playlistArtworkList(activePlaylist.trackIds) : [],
  );

  let heroTotalDuration = $derived(formatTotalDuration(playlistTracks));

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

  function playlistArtworkList(trackIds: string[]): string[] {
    const seen = new Set<string>();
    const out: string[] = [];
    for (const id of trackIds) {
      const url = resolveArtworkUrl(library.tracksIndex[id]?.artworkUrl ?? null, proxyPort);
      if (url && !seen.has(url)) {
        seen.add(url);
        out.push(url);
        if (out.length === 4) break;
      }
    }
    return out;
  }

  function formatTrackDuration(ms: number | null): string {
    if (!ms || ms <= 0) return "—";
    const totalSec = Math.floor(ms / 1000);
    const min = Math.floor(totalSec / 60);
    const sec = totalSec % 60;
    return `${min}:${sec.toString().padStart(2, "0")}`;
  }

  function formatTotalDuration(tracks: Track[]): string | null {
    const totalMs = tracks.reduce((sum, t) => sum + (t.durationMs ?? 0), 0);
    if (totalMs <= 0) return null;
    const totalMin = Math.round(totalMs / 60_000);
    if (totalMin < 1) return "< 1 min";
    const hrs = Math.floor(totalMin / 60);
    const mins = totalMin % 60;
    return hrs > 0 ? `${hrs} hr ${mins} min` : `${mins} min`;
  }

  function formatMb(bytes: number): string {
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
</script>

<div class="page">
  {#if activePlaylist}
    <div class="pl-top-nav">
      <button
        class="back-btn"
        onclick={() => {
          onSelectPlaylist(null);
          onTabChange("playlists");
        }}
      >
        <Icon name="arrow-left" size={14} />
        <span>All Playlists</span>
      </button>
    </div>

    <section class="pl-hero card-glass">
      <div class="pl-hero-art">
        {#if heroCovers.length >= 4}
          <div class="mosaic">
            {#each heroCovers as src (src)}
              <img {src} alt={activePlaylist.name} loading="lazy" />
            {/each}
          </div>
        {:else if heroCovers.length > 0}
          <img src={heroCovers[0]} alt={activePlaylist.name} />
        {:else}
          <Icon name="list-music" size={44} />
        {/if}
      </div>

      <div class="pl-hero-body">
        <div class="pill-badge">Playlist</div>
        {#if renaming}
          <form class="rename-form" onsubmit={submitRename}>
            <input type="text" bind:value={renameInput} />
            <button type="submit" class="btn-primary">Save</button>
            <button type="button" class="btn-ghost" onclick={() => (renaming = false)}>Cancel</button>
          </form>
        {:else}
          <h1 class="pl-title">{activePlaylist.name}</h1>
        {/if}
        <p class="pl-meta-line">
          {playlistTracks.length} {playlistTracks.length === 1 ? "track" : "tracks"}
          {#if heroTotalDuration}
            • {heroTotalDuration}
          {/if}
        </p>

        <div class="pl-hero-actions">
          {#if playlistTracks.length > 0}
            <button class="btn-primary" onclick={() => onPlay(playlistTracks[0], playlistTracks)}>
              <Icon name="play" size={14} />
              <span>Play</span>
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
          <button
            class="btn-ghost danger"
            onclick={() => (pendingDeletePlaylist = activePlaylist)}
          >
            <Icon name="trash" size={14} />
            <span>Delete</span>
          </button>
        </div>
      </div>
    </section>

    <section class="pl-tracks-panel card-glass">
      {#if playlistTracks.length >= 6}
        <div class="pl-filter-bar">
          <Icon name="search" size={14} />
          <input
            type="text"
            placeholder="Filter tracks in playlist..."
            bind:value={playlistQuery}
          />
          {#if playlistQuery}
            <button
              class="clear-filter"
              aria-label="Clear filter"
              onclick={() => (playlistQuery = "")}
            >
              <Icon name="x" size={13} />
            </button>
          {/if}
        </div>
      {/if}

      {#if playlistTracks.length === 0}
        <div class="empty">This playlist is empty. Add songs from Home, Search, or Identify.</div>
      {:else if filteredPlaylistTracks.length === 0}
        <div class="empty">No tracks match "{playlistQuery}".</div>
      {:else}
        <div class="pl-table-head">
          <span class="col-num">#</span>
          <span>Title</span>
          <span class="col-album">Album</span>
          <span class="col-dur">Time</span>
          <span></span>
        </div>

        {#each filteredPlaylistTracks as track, idx (track.id)}
          {@const isLiked = likedIds.has(track.id)}
          {@const isPlayingTrack = currentTrackId === track.id}
          {@const art = resolveArtworkUrl(track.artworkUrl, proxyPort)}
          <div class="pl-table-row" class:playing={isPlayingTrack}>
            <span class="col-num">
              {#if isPlayingTrack}
                <Icon name="play" size={12} />
              {:else}
                {idx + 1}
              {/if}
            </span>

            <button class="pl-row-main" onclick={() => onPlay(track, playlistTracks)}>
              <div class="thumb">
                {#if art}
                  <img src={art} alt={track.title} loading="lazy" />
                {:else}
                  <Icon name="music" size={17} />
                {/if}
              </div>
              <div class="meta">
                <div class="title">{track.title}</div>
                <div class="sub">{track.artist}</div>
              </div>
            </button>

            <span class="col-album">{track.album || "—"}</span>
            <span class="col-dur">{formatTrackDuration(track.durationMs)}</span>

            <div class="pl-row-acts">
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
                class="act"
                title="Remove from playlist"
                aria-label="Remove from playlist"
                onclick={() => onRemoveFromPlaylist(activePlaylist.id, track.id)}
              >
                <Icon name="x" size={14} />
              </button>
            </div>
          </div>
        {/each}
      {/if}
    </section>
  {:else}
    <div class="header-row">
      <div>
        <h1>Your Library</h1>
      </div>
      <div class="head-actions">
        <button class="btn-ghost" onclick={onOpenImportPlaylist}>
          <Icon name="link" size={14} />
          <span>Import</span>
        </button>
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
          <div class="empty">No favorites yet.</div>
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
          <div class="empty">No playlists yet.</div>
        </div>
      {:else}
        <div class="pl-grid">
          {#each library.playlists as pl (pl.id)}
            {@const covers = playlistArtworkList(pl.trackIds)}
            {@const tracks = pl.trackIds.map((id) => library.tracksIndex[id]).filter((t): t is Track => Boolean(t))}
            <div
              class="pl-card"
              role="button"
              tabindex="0"
              onclick={() => onSelectPlaylist(pl.id)}
              onkeydown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onSelectPlaylist(pl.id);
                }
              }}
            >
              <div class="pl-cover">
                {#if covers.length >= 4}
                  <div class="mosaic">
                    {#each covers as src (src)}
                      <img {src} alt={pl.name} loading="lazy" />
                    {/each}
                  </div>
                {:else if covers.length > 0}
                  <img src={covers[0]} alt={pl.name} loading="lazy" />
                {:else}
                  <Icon name="list-music" size={32} />
                {/if}
                <div class="pl-scrim"></div>
                <button
                  class="pl-del"
                  title="Delete playlist"
                  aria-label="Delete playlist"
                  onclick={(e) => {
                    e.stopPropagation();
                    pendingDeletePlaylist = pl;
                  }}
                >
                  <Icon name="trash" size={13} />
                </button>
                {#if tracks.length > 0}
                  <button
                    class="pl-fab"
                    title={`Play ${pl.name}`}
                    aria-label={`Play ${pl.name}`}
                    onclick={(e) => {
                      e.stopPropagation();
                      onPlay(tracks[0], tracks);
                    }}
                  >
                    <Icon name="play" size={16} />
                  </button>
                {/if}
              </div>
              <div class="pl-meta">
                <div class="pl-card-title">{pl.name}</div>
                <div class="pl-card-sub">{tracks.length} {tracks.length === 1 ? "track" : "tracks"}</div>
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
          <div class="empty">Add a music folder to browse local songs.</div>
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
          <div class="empty">No downloaded songs yet.</div>
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
          <div class="empty">Songs you play will appear here.</div>
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

<ConfirmModal
  open={Boolean(pendingDeletePlaylist)}
  title="Delete Playlist?"
  message={pendingDeletePlaylist
    ? `Delete "${pendingDeletePlaylist.name}" (${pendingDeletePlaylist.trackIds.length} tracks)? Tracks inside it won't be deleted from Favorites or Local Files.`
    : ""}
  confirmLabel="Delete Playlist"
  onCancel={() => (pendingDeletePlaylist = null)}
  onConfirm={() => {
    const target = pendingDeletePlaylist;
    pendingDeletePlaylist = null;
    if (target) onDeletePlaylist(target.id);
  }}
/>

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
    transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease, transform 0.12s ease;
  }

  .chip:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--p-text);
  }

  .chip:active {
    transform: scale(0.95);
  }

  .chip.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    border-color: var(--p-accent);
  }

  .pl-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(176px, 1fr));
    gap: 16px;
  }

  .pl-card {
    padding: 11px 11px 13px;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.035);
    border: 1px solid rgba(255, 255, 255, 0.07);
    backdrop-filter: blur(14px);
    display: flex;
    flex-direction: column;
    gap: 11px;
    cursor: pointer;
    transition:
      transform 0.22s cubic-bezier(0.22, 1, 0.36, 1),
      background 0.22s ease,
      border-color 0.22s ease,
      box-shadow 0.22s ease;
  }

  .pl-card:hover,
  .pl-card:focus-visible {
    outline: none;
    transform: translateY(-3px);
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.16);
    box-shadow: 0 14px 30px rgba(0, 0, 0, 0.34);
  }

  .pl-cover {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: 12px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.06);
    display: grid;
    place-items: center;
    color: var(--p-text-dim);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.36);
  }

  .pl-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: transform 0.35s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .pl-card:hover .pl-cover img {
    transform: scale(1.05);
  }

  .pl-scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(6, 9, 16, 0.14) 0%, transparent 45%, rgba(6, 9, 16, 0.56) 100%);
    opacity: 0;
    transition: opacity 0.22s ease;
    pointer-events: none;
  }

  .pl-card:hover .pl-scrim {
    opacity: 1;
  }

  .pl-del {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: rgba(10, 13, 24, 0.72);
    border: 1px solid rgba(255, 255, 255, 0.14);
    backdrop-filter: blur(12px);
    color: rgba(252, 165, 165, 0.9);
    display: grid;
    place-items: center;
    opacity: 0;
    transform: translateY(-4px);
    transition: opacity 0.18s ease, transform 0.2s ease, background 0.14s ease;
  }

  .pl-card:hover .pl-del,
  .pl-card:focus-within .pl-del {
    opacity: 1;
    transform: translateY(0);
  }

  .pl-del:hover {
    background: rgba(239, 68, 68, 0.28);
    color: var(--p-danger);
  }

  .pl-fab {
    position: absolute;
    right: 10px;
    bottom: 10px;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--p-accent-bright), var(--p-accent));
    color: var(--p-on-accent);
    display: grid;
    place-items: center;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.45);
    opacity: 0;
    transform: translateY(8px) scale(0.92);
    transition: opacity 0.2s ease, transform 0.24s cubic-bezier(0.22, 1, 0.36, 1), filter 0.15s ease;
  }

  .pl-card:hover .pl-fab,
  .pl-card:focus-within .pl-fab {
    opacity: 1;
    transform: translateY(0) scale(1);
  }

  .pl-fab:hover {
    transform: scale(1.07);
    filter: brightness(1.08);
  }

  .pl-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 2px;
  }

  .pl-card-title {
    font-size: 13.5px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pl-card-sub {
    font-size: 12px;
    color: var(--p-text-muted);
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

  .act:active {
    transform: scale(0.91);
  }

  .act.liked {
    color: #fb7185;
  }

  .danger {
    color: var(--p-danger);
  }

  .pl-top-nav {
    display: flex;
    align-items: center;
  }

  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 6px 12px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    color: var(--p-text-muted);
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    transition: color 0.15s ease, border-color 0.15s ease;
  }

  .back-btn:hover {
    color: var(--p-text);
    border-color: var(--p-border-strong);
  }

  .pl-hero {
    padding: 20px;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 22px;
    background: linear-gradient(135deg, var(--p-surface-tint), var(--p-surface));
  }

  .pl-hero-art {
    width: 172px;
    height: 172px;
    border-radius: 16px;
    overflow: hidden;
    flex-shrink: 0;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    display: grid;
    place-items: center;
    color: var(--p-text-dim);
    box-shadow: 0 14px 32px rgba(0, 0, 0, 0.3);
  }

  .pl-hero-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .mosaic {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
  }

  .mosaic img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .pl-hero-body {
    flex: 1;
    min-width: 240px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .pl-title {
    margin: 0;
    font-size: clamp(24px, 3vw, 34px);
    line-height: 1.12;
    letter-spacing: -0.03em;
  }

  .pl-meta-line {
    margin: 0;
    font-size: 13px;
    color: var(--p-text-muted);
  }

  .pl-hero-actions {
    margin-top: 8px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .pl-tracks-panel {
    padding: 10px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .pl-filter-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 2px 4px 8px;
    padding: 7px 12px;
    border-radius: 10px;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    color: var(--p-text-muted);
  }

  .pl-filter-bar input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    outline: none;
    font-size: 13px;
    color: var(--p-text);
  }

  .clear-filter {
    color: var(--p-text-muted);
    display: grid;
    place-items: center;
  }

  .pl-table-head,
  .pl-table-row {
    display: grid;
    grid-template-columns: 36px minmax(0, 2fr) minmax(0, 1.1fr) 56px 108px;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: 10px;
  }

  .pl-table-head {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--p-text-dim);
    border-bottom: 1px solid var(--p-border);
    border-radius: 0;
    padding-bottom: 9px;
    margin-bottom: 2px;
  }

  .pl-table-row:hover {
    background: var(--p-fill);
  }

  .pl-table-row.playing {
    background: var(--p-accent-soft);
  }

  .col-num {
    text-align: center;
    font-size: 12px;
    color: var(--p-text-dim);
    font-variant-numeric: tabular-nums;
    display: flex;
    justify-content: center;
  }

  .pl-table-row.playing .col-num {
    color: var(--p-accent-bright);
  }

  .pl-row-main {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 11px;
    text-align: left;
  }

  .col-album {
    min-width: 0;
    font-size: 12.5px;
    color: var(--p-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .col-dur {
    text-align: right;
    font-size: 12px;
    color: var(--p-text-muted);
    font-variant-numeric: tabular-nums;
  }

  .pl-row-acts {
    display: flex;
    justify-content: flex-end;
    gap: 5px;
  }

  @media (max-width: 900px) {
    .pl-table-head,
    .pl-table-row {
      grid-template-columns: 30px minmax(0, 1fr) 48px 104px;
    }

    .col-album {
      display: none;
    }
  }
</style>
