<script lang="ts">
  import { getDemoTracks } from "../lib/demoTracks";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { CollectionCard, HomeFeed, Track } from "../lib/types";
  import CoverTile from "./CoverTile.svelte";
  import Icon from "./Icon.svelte";
  import Shelf from "./Shelf.svelte";
  import SongRow from "./SongRow.svelte";

  interface Props {
    feed: HomeFeed;
    loading: boolean;
    demoMode: boolean;
    history: Track[];
    likedIds: Set<string>;
    currentTrackId: string | null;
    isPlaying: boolean;
    proxyPort: number;
    onPlay: (track: Track, list: Track[]) => void;
    onToggleLike: (track: Track) => void;
    onOpenPlaylistPicker: (track: Track) => void;
    onOpenCollection: (card: CollectionCard) => void;
    onPlayCollection: (card: CollectionCard) => void;
    onRefresh: () => void;
    onOpenLocal: () => void;
  }

  let {
    feed,
    loading,
    demoMode,
    history,
    likedIds,
    currentTrackId,
    isPlaying,
    proxyPort,
    onPlay,
    onToggleLike,
    onOpenPlaylistPicker,
    onOpenCollection,
    onPlayCollection,
    onRefresh,
    onOpenLocal,
  }: Props = $props();

  const demoTracks = getDemoTracks();
  const SONG_COLUMN = "max(300px, 32%)";

  let recents = $derived(history.slice(0, 8));

  function art(url: string | null): string | null {
    return resolveArtworkUrl(url, proxyPort);
  }

  function greeting(): string {
    const h = new Date().getHours();
    if (h < 5) return "Good night";
    if (h < 12) return "Good morning";
    if (h < 18) return "Good afternoon";
    return "Good evening";
  }

  function kindLabel(card: CollectionCard): string {
    return card.subtitle || card.kind[0].toUpperCase() + card.kind.slice(1);
  }
</script>

{#snippet songShelf(title: string, strapline: string | null, tracks: Track[])}
  <Shelf {title} {strapline} colWidth={SONG_COLUMN} rows={Math.min(4, tracks.length)}>
    {#each tracks as track (track.id)}
      <SongRow
        {track}
        artworkUrl={art(track.artworkUrl)}
        isLiked={likedIds.has(track.id)}
        isCurrent={currentTrackId === track.id}
        {isPlaying}
        onPlay={() => onPlay(track, tracks)}
        onToggleLike={() => onToggleLike(track)}
        onAddToPlaylist={() => onOpenPlaylistPicker(track)}
      />
    {/each}
  </Shelf>
{/snippet}

<div class="page">
  <header class="top">
    <h1>{greeting()}</h1>
    <button class="btn-icon" title="Refresh" aria-label="Refresh home feed" onclick={onRefresh} disabled={loading}>
      <Icon name="refresh" size={16} spin={loading} />
    </button>
  </header>

  {#if recents.length > 0}
    <div class="recents">
      {#each recents as track (track.id)}
        {@const cover = art(track.artworkUrl)}
        <div class="recent" class:current={currentTrackId === track.id}>
          <button class="recent-main" onclick={() => onPlay(track, history)}>
            <span class="recent-art">
              {#if cover}<img src={cover} alt="" loading="lazy" />{:else}<Icon name="music" size={18} />{/if}
            </span>
            <span class="recent-title">{track.title}</span>
          </button>
          <button class="btn-play recent-play" aria-label={`Play ${track.title}`} onclick={() => onPlay(track, history)}>
            <Icon name={currentTrackId === track.id && isPlaying ? "pause" : "play"} size={15} />
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if demoMode}
    {@render songShelf("Demo tracks", "Offline", demoTracks)}
  {/if}

  {#if loading && feed.shelves.length === 0}
    {#each [0, 1] as row (row)}
      <div class="skeleton" aria-hidden="true">
        <div class="skel-title"></div>
        <div class="skel-tiles">
          {#each Array(6) as _, i (i)}<div class="skel-tile"></div>{/each}
        </div>
      </div>
    {/each}
  {:else if feed.shelves.length > 0}
    {#each feed.shelves as shelf, i (`${i}-${shelf.title}`)}
      {#if shelf.tracks.length > 0}
        {@render songShelf(shelf.title, shelf.strapline, shelf.tracks)}
      {/if}
      {#if shelf.collections.length > 0}
        <Shelf title={shelf.title} strapline={shelf.tracks.length > 0 ? null : shelf.strapline}>
          {#each shelf.collections as card (card.browseId)}
            {@const cover = art(card.artworkUrl)}
            <CoverTile
              title={card.title}
              subtitle={kindLabel(card)}
              covers={cover ? [cover] : []}
              round={card.kind === "artist"}
              onOpen={() => onOpenCollection(card)}
              onPlay={card.kind === "artist" ? undefined : () => onPlayCollection(card)}
            />
          {/each}
        </Shelf>
      {/if}
    {/each}
  {:else}
    <div class="offline">
      <Icon name="music" size={22} />
      <div class="offline-text">
        <strong>Couldn't load your feed</strong>
        <span>{feed.error ?? "YouTube Music didn't return anything."} Your library and local files still work.</span>
      </div>
      <button class="btn-primary" onclick={onRefresh}>Try again</button>
      <button class="btn-ghost" onclick={onOpenLocal}>Local files</button>
    </div>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 34px;
    padding: 28px 22px 48px;
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 10px;
  }

  h1 {
    margin: 0;
    font-size: 30px;
    font-weight: 800;
    letter-spacing: -0.03em;
  }

  .recents {
    display: grid;
    /* Four columns on a wide window, so eight recents make two even rows. */
    grid-template-columns: repeat(auto-fill, minmax(min(100%, max(240px, 23%)), 1fr));
    gap: 10px;
    padding: 0 10px;
    margin-top: -14px;
  }

  .recent {
    position: relative;
    display: flex;
    align-items: center;
    height: 60px;
    overflow: hidden;
    border-radius: 8px;
    background: var(--p-fill);
    transition: background 0.15s ease;
  }

  .recent:hover {
    background: color-mix(in srgb, var(--p-text) 10%, var(--p-fill));
  }

  .recent-main {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding-right: 52px;
    text-align: left;
  }

  .recent-art {
    width: 60px;
    height: 60px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    background: var(--p-surface-elevated);
    color: var(--p-text-dim);
    box-shadow: 4px 0 14px rgb(0 0 0 / 0.22);
  }

  .recent-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .recent-title {
    font-size: 13.5px;
    font-weight: 700;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .recent.current .recent-title {
    color: var(--p-accent-bright);
  }

  .recent-play {
    position: absolute;
    right: 10px;
    width: 36px;
    height: 36px;
    opacity: 0;
    transition: opacity 0.15s ease, background 0.14s ease;
  }

  .recent:hover .recent-play,
  .recent:focus-within .recent-play,
  .recent.current .recent-play {
    opacity: 1;
  }

  .skeleton {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 0 10px;
  }

  .skel-title {
    width: 200px;
    height: 20px;
    border-radius: 6px;
    background: var(--p-fill);
  }

  .skel-tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    grid-template-rows: auto;
    grid-auto-rows: 0;
    overflow: hidden;
    gap: 20px;
  }

  .skel-tile {
    aspect-ratio: 1;
    border-radius: 8px;
    background: var(--p-fill);
    animation: pulse 1.3s ease-in-out infinite alternate;
  }

  @keyframes pulse {
    to {
      opacity: 0.45;
    }
  }

  .offline {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
    margin: 0 10px;
    padding: 18px 20px;
    border-radius: 12px;
    background: var(--p-fill);
    color: var(--p-text-muted);
  }

  .offline-text {
    flex: 1;
    min-width: 220px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
  }

  .offline-text strong {
    color: var(--p-text);
    font-size: 14px;
  }
</style>
