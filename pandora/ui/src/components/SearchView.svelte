<script lang="ts">
  import { ytmSearch } from "../lib/ipc";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { Playlist, Track } from "../lib/types";

  interface Props {
    localTracks: Track[];
    likedIds: Set<string>;
    cachedIds: Set<string>;
    playlists: Playlist[];
    currentTrackId: string | null;
    proxyPort: number;
    onPlay: (track: Track, list: Track[]) => void;
    onEnqueue: (track: Track) => void;
    onToggleLike: (track: Track) => void;
    onDownload: (track: Track) => void;
    onAddToPlaylist: (playlistId: string, track: Track) => void;
  }

  let {
    localTracks,
    likedIds,
    cachedIds,
    playlists,
    currentTrackId,
    proxyPort,
    onPlay,
    onEnqueue,
    onToggleLike,
    onDownload,
    onAddToPlaylist,
  }: Props = $props();

  let query = $state("");
  let filter = $state<"songs" | "videos" | "local">("songs");
  let searching = $state(false);
  let results = $state<Track[]>([]);
  let error = $state<string | null>(null);
  let downloadingId = $state<string | null>(null);
  let pickerTrackId = $state<string | null>(null);

  let displayedTracks = $derived.by(() => {
    if (filter === "local") {
      const q = query.trim().toLowerCase();
      return q
        ? localTracks.filter(
            (t) =>
              t.title.toLowerCase().includes(q) ||
              t.artist.toLowerCase().includes(q) ||
              t.album.toLowerCase().includes(q),
          )
        : localTracks;
    }
    return results;
  });

  async function runSearch(e?: Event) {
    e?.preventDefault();
    if (filter === "local") return;
    if (!query.trim()) return;
    searching = true;
    error = null;
    try {
      const res = await ytmSearch(query, filter);
      results = res.tracks;
      error = res.error;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      searching = false;
    }
  }

  async function handleDownload(track: Track) {
    downloadingId = track.id;
    try {
      await onDownload(track);
    } finally {
      downloadingId = null;
    }
  }

  function formatMs(ms: number | null): string {
    if (!ms) return "--:--";
    const sec = Math.floor(ms / 1000);
    return `${Math.floor(sec / 60)}:${String(sec % 60).padStart(2, "0")}`;
  }
</script>

<div class="page">
  <div class="search-head">
    <h1>Search</h1>
    <form class="search-bar" onsubmit={runSearch}>
      <input
        type="text"
        placeholder="Search YouTube Music songs, artists, or local files..."
        bind:value={query}
      />
      <button type="submit" class="btn-primary" disabled={searching}>
        {searching ? "Searching..." : "Search"}
      </button>
    </form>

    <div class="chips">
      <button class="chip" class:active={filter === "songs"} onclick={() => (filter = "songs")}>
        YouTube Songs
      </button>
      <button class="chip" class:active={filter === "videos"} onclick={() => (filter = "videos")}>
        Videos
      </button>
      <button class="chip" class:active={filter === "local"} onclick={() => (filter = "local")}>
        Local Files ({localTracks.length})
      </button>
    </div>
  </div>

  {#if error}
    <div class="err card-glass">{error}</div>
  {/if}

  <div class="track-list card-glass">
    {#if displayedTracks.length === 0}
      <div class="empty">
        {#if filter === "local"}
          No matching local tracks. Add folders in <strong>Library &amp; Local</strong>.
        {:else if searching}
          Querying YouTube Music InnerTube...
        {:else}
          Type a song or artist name and press Enter to search YouTube Music.
        {/if}
      </div>
    {:else}
      {#each displayedTracks as track (track.id)}
        {@const isLiked = likedIds.has(track.id) || track.liked}
        {@const isCached = cachedIds.has(track.id) || track.cached}
        <div class="row" class:playing={currentTrackId === track.id}>
          <button class="row-main" onclick={() => onPlay(track, displayedTracks)}>
            <div class="thumb">
              {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
                <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
              {:else}
                <span>♫</span>
              {/if}
            </div>
            <div class="meta">
              <div class="title">{track.title}</div>
              <div class="sub">
                {track.artist}{track.album ? ` • ${track.album}` : ""}
              </div>
            </div>
          </button>

          <div class="dur">{formatMs(track.durationMs)}</div>

          <div class="actions">
            <button
              class="act"
              class:liked={isLiked}
              title="Like"
              onclick={() => onToggleLike(track)}
            >
              {isLiked ? "♥" : "♡"}
            </button>
            <button class="act" title="Add to queue" onclick={() => onEnqueue(track)}>+Q</button>
            {#if track.source === "ytmusic"}
              <button
                class="act"
                class:cached={isCached}
                disabled={isCached || downloadingId === track.id}
                title={isCached ? "Cached offline" : "Download for offline"}
                onclick={() => handleDownload(track)}
              >
                {downloadingId === track.id ? "…" : isCached ? "✓" : "↓"}
              </button>
            {/if}
            {#if playlists.length > 0}
              <div class="pl-wrap">
                <button
                  class="act"
                  title="Add to playlist"
                  onclick={() => (pickerTrackId = pickerTrackId === track.id ? null : track.id)}
                >
                  ＋
                </button>
                {#if pickerTrackId === track.id}
                  <div class="pl-menu card-glass">
                    {#each playlists as pl (pl.id)}
                      <button
                        onclick={() => {
                          onAddToPlaylist(pl.id, track);
                          pickerTrackId = null;
                        }}
                      >
                        {pl.name}
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .page {
    padding: 24px 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .search-head h1 {
    margin: 0 0 12px;
    font-size: 24px;
  }

  .search-bar {
    display: flex;
    gap: 10px;
  }

  .search-bar input {
    flex: 1;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--p-border);
    border-radius: 999px;
    padding: 11px 18px;
    font-size: 14px;
    outline: none;
  }

  .search-bar input:focus {
    border-color: var(--p-accent);
  }

  .chips {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }

  .chip {
    padding: 6px 14px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.06);
    color: var(--p-text-muted);
    border: 1px solid var(--p-border);
  }

  .chip.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    border-color: var(--p-accent);
  }

  .err {
    padding: 12px 16px;
    color: #fca5a5;
    font-size: 13px;
  }

  .track-list {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .empty {
    padding: 36px 20px;
    text-align: center;
    color: var(--p-text-muted);
    font-size: 14px;
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
    width: 44px;
    height: 44px;
    border-radius: 8px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.07);
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

  .sub {
    font-size: 12px;
    color: var(--p-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dur {
    font-size: 12px;
    color: var(--p-text-dim);
    width: 46px;
    text-align: right;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .act {
    padding: 5px 8px;
    border-radius: 8px;
    font-size: 12px;
    color: var(--p-text-muted);
    background: rgba(255, 255, 255, 0.04);
  }

  .act:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--p-text);
  }

  .act.liked {
    color: #fb7185;
  }

  .act.cached {
    color: var(--p-success);
  }

  .pl-wrap {
    position: relative;
  }

  .pl-menu {
    position: absolute;
    right: 0;
    top: 30px;
    z-index: 20;
    min-width: 150px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pl-menu button {
    text-align: left;
    padding: 7px 10px;
    border-radius: 8px;
    font-size: 12px;
  }

  .pl-menu button:hover {
    background: rgba(255, 255, 255, 0.1);
  }
</style>
