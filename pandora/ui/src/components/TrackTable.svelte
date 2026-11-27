<script lang="ts">
  import { formatTrackDuration } from "../lib/format";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { Track } from "../lib/types";
  import Icon from "./Icon.svelte";

  /**
   * The one track list used by playlists, collections and the library tabs: `#`, title + artist, album, time,
   * and row actions that appear on hover (favourite stays visible once set).
   */
  interface Props {
    tracks: Track[];
    likedIds: Set<string>;
    cachedIds?: Set<string>;
    currentTrackId: string | null;
    isPlaying: boolean;
    proxyPort: number;
    showAlbum?: boolean;
    /** Show the filter box from this many tracks (0 = never). */
    filterFrom?: number;
    emptyText?: string;
    onPlay: (track: Track, list: Track[]) => void;
    onToggleLike: (track: Track) => void;
    onAddToPlaylist: (track: Track) => void;
    onDownload?: (track: Track) => void;
    onRemove?: (track: Track) => void;
    removeLabel?: string;
  }

  let {
    tracks,
    likedIds,
    cachedIds = new Set<string>(),
    currentTrackId,
    isPlaying,
    proxyPort,
    showAlbum = true,
    filterFrom = 12,
    emptyText = "Nothing here yet.",
    onPlay,
    onToggleLike,
    onAddToPlaylist,
    onDownload,
    onRemove,
    removeLabel = "Remove",
  }: Props = $props();

  let query = $state("");

  let visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return tracks;
    return tracks.filter((t) => `${t.title} ${t.artist} ${t.album}`.toLowerCase().includes(q));
  });
</script>

{#if filterFrom > 0 && tracks.length >= filterFrom}
  <label class="filter">
    <Icon name="search" size={14} />
    <input type="text" placeholder="Filter by title, artist or album" bind:value={query} />
    {#if query}
      <button class="btn-icon sm bare" aria-label="Clear filter" onclick={() => (query = "")}>
        <Icon name="x" size={13} />
      </button>
    {/if}
  </label>
{/if}

{#if tracks.length === 0}
  <p class="empty">{emptyText}</p>
{:else if visible.length === 0}
  <p class="empty">No tracks match “{query}”.</p>
{:else}
  <div class="table" class:no-album={!showAlbum} role="list">
    <div class="row head" aria-hidden="true">
      <span class="num">#</span>
      <span>Title</span>
      {#if showAlbum}<span class="album">Album</span>{/if}
      <span class="time">Time</span>
      <span></span>
    </div>

    {#each visible as track, idx (track.id)}
      {@const liked = likedIds.has(track.id)}
      {@const current = currentTrackId === track.id}
      {@const cached = cachedIds.has(track.id)}
      {@const art = resolveArtworkUrl(track.artworkUrl, proxyPort)}
      <div
        class="row"
        class:current
        role="listitem"
        ondblclick={() => onPlay(track, tracks)}
      >
        <button class="num" aria-label={`Play ${track.title}`} onclick={() => onPlay(track, tracks)}>
          {#if current && isPlaying}
            <span class="bars" aria-hidden="true"><i></i><i></i><i></i></span>
          {:else}
            <span class="idx">{idx + 1}</span>
            <span class="play"><Icon name="play" size={13} /></span>
          {/if}
        </button>

        <button class="main" onclick={() => onPlay(track, tracks)}>
          <span class="thumb">
            {#if art}
              <img src={art} alt="" loading="lazy" />
            {:else}
              <Icon name="music" size={16} />
            {/if}
          </span>
          <span class="meta">
            <span class="title">{track.title}</span>
            <span class="sub">
              {#if cached}<span class="saved" title="Downloaded"><Icon name="download" size={11} /></span>{/if}
              {track.artist}
            </span>
          </span>
        </button>

        {#if showAlbum}<span class="album">{track.album || "—"}</span>{/if}
        <span class="time">{formatTrackDuration(track.durationMs)}</span>

        <span class="acts">
          <button
            class="btn-icon sm bare"
            class:keep={liked}
            class:on={liked}
            title={liked ? "Remove from Favorites" : "Add to Favorites"}
            aria-label={liked ? "Remove from Favorites" : "Add to Favorites"}
            onclick={() => onToggleLike(track)}
          >
            <Icon name={liked ? "heart-filled" : "heart"} size={15} />
          </button>
          <button class="btn-icon sm bare" title="Add to playlist" aria-label="Add to playlist" onclick={() => onAddToPlaylist(track)}>
            <Icon name="list-plus" size={15} />
          </button>
          {#if onDownload && track.source === "ytmusic" && !cached}
            <button class="btn-icon sm bare" title="Download" aria-label="Download" onclick={() => onDownload(track)}>
              <Icon name="download" size={15} />
            </button>
          {/if}
          {#if onRemove}
            <button class="btn-icon sm bare" title={removeLabel} aria-label={removeLabel} onclick={() => onRemove(track)}>
              <Icon name="x" size={15} />
            </button>
          {/if}
        </span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .filter {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    max-width: 340px;
    margin-bottom: 12px;
    padding: 0 6px 0 12px;
    border-radius: 999px;
    background: var(--p-fill);
    color: var(--p-text-dim);
  }

  .filter input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    font-size: 13px;
  }

  .filter:focus-within {
    outline: 2px solid var(--p-accent);
  }

  .empty {
    margin: 0;
    padding: 28px 4px;
    color: var(--p-text-dim);
    font-size: 13px;
  }

  .table {
    --cols: 40px minmax(0, 4fr) minmax(0, 3fr) 56px 112px;
    display: flex;
    flex-direction: column;
  }

  .table.no-album {
    --cols: 40px minmax(0, 1fr) 56px 112px;
  }

  .row {
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    gap: 12px;
    min-height: 56px;
    padding: 0 8px;
    border-radius: 8px;
    font-size: 13px;
  }

  .row:not(.head):hover,
  .row:not(.head):focus-within {
    background: var(--p-fill);
  }

  .row.head {
    min-height: 34px;
    margin-bottom: 6px;
    border-bottom: 1px solid var(--p-border);
    border-radius: 0;
    color: var(--p-text-dim);
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .num {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    color: var(--p-text-dim);
    font-variant-numeric: tabular-nums;
  }

  .num .play {
    display: none;
    color: var(--p-text);
  }

  .row:hover .num .idx {
    display: none;
  }

  .row:hover .num .play {
    display: inline-flex;
  }

  .main {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 6px 0;
    text-align: left;
  }

  .thumb {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    overflow: hidden;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    background: var(--p-fill);
    color: var(--p-text-dim);
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .title,
  .sub,
  .album {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 14px;
    font-weight: 600;
    color: var(--p-text);
  }

  .row.current .title {
    color: var(--p-accent-bright);
  }

  .sub {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--p-text-muted);
    font-size: 12.5px;
  }

  .saved {
    display: inline-grid;
    place-items: center;
    width: 15px;
    height: 15px;
    border-radius: 50%;
    background: var(--p-accent);
    color: var(--p-on-accent);
  }

  .album {
    color: var(--p-text-muted);
  }

  .time {
    color: var(--p-text-dim);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .row.head .time {
    text-align: right;
  }

  .acts {
    display: flex;
    justify-content: flex-end;
    gap: 2px;
  }

  .acts .btn-icon:not(.keep) {
    opacity: 0;
  }

  .row:hover .acts .btn-icon,
  .row:focus-within .acts .btn-icon {
    opacity: 1;
  }

  .bars {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
    height: 13px;
  }

  .bars i {
    width: 3px;
    border-radius: 1px;
    background: var(--p-accent-bright);
    animation: bar 0.8s ease-in-out infinite alternate;
  }

  .bars i:nth-child(2) {
    animation-delay: 0.2s;
  }

  .bars i:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes bar {
    from {
      height: 3px;
    }
    to {
      height: 13px;
    }
  }

  @media (max-width: 860px) {
    .table:not(.no-album) {
      --cols: 40px minmax(0, 1fr) 56px 112px;
    }

    .table .album {
      display: none;
    }
  }
</style>
