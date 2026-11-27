<script lang="ts">
  import { untrack } from "svelte";
  import { ytmRecommend } from "../lib/ipc";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { Track } from "../lib/types";
  import HeartButton from "./HeartButton.svelte";
  import Icon from "./Icon.svelte";

  /**
   * "Recommended songs" under a playlist: songs YouTube Music suggests for the songs in it (ranked in Rust,
   * `recommend.rs`). Loads once per playlist (key this component by playlist id); adding a song removes it from the
   * list without asking again, and Refresh asks about other songs of the playlist and skips everything shown so far.
   */
  interface Props {
    tracks: Track[];
    likedIds: Set<string>;
    currentTrackId: string | null;
    isPlaying: boolean;
    proxyPort: number;
    onPlay: (track: Track, list: Track[]) => void;
    onToggleLike: (track: Track) => void;
    onAdd: (track: Track) => void;
  }

  let { tracks, likedIds, currentTrackId, isPlaying, proxyPort, onPlay, onToggleLike, onAdd }: Props = $props();

  let recs = $state<Track[]>([]);
  let loading = $state(false);
  let failed = $state(false);
  let started = false;
  let round = 0;
  let request = 0;
  /** Everything shown so far, so Refresh brings new songs. */
  const shown = new Set<string>();

  const canRecommend = $derived(tracks.some((t) => t.source === "ytmusic"));

  $effect(() => {
    if (canRecommend && !started) {
      started = true;
      untrack(() => void load());
    }
  });

  async function load() {
    const id = ++request;
    loading = true;
    failed = false;
    try {
      const next = await ytmRecommend(tracks, round, [...shown]);
      if (id !== request) return;
      next.forEach((t) => shown.add(t.id));
      recs = next;
    } catch {
      if (id === request) failed = true;
    } finally {
      if (id === request) loading = false;
    }
  }

  function refresh() {
    round += 1;
    void load();
  }

  function add(track: Track) {
    recs = recs.filter((t) => t.id !== track.id);
    onAdd(track);
  }
</script>

{#if canRecommend}
  <section class="recs" aria-labelledby="recs-title">
    <header>
      <div>
        <h2 id="recs-title">Recommended songs</h2>
        <p>Based on the songs in this playlist</p>
      </div>
      <button class="btn-ghost" disabled={loading} onclick={refresh}>
        <Icon name={loading ? "loader" : "refresh"} size={14} spin={loading} />
        <span>Refresh</span>
      </button>
    </header>

    {#if failed}
      <p class="note">Couldn’t load recommendations. Check your connection and try Refresh.</p>
    {:else if !loading && recs.length === 0}
      <p class="note">No new suggestions right now. Try Refresh.</p>
    {:else}
      <div class="list" role="list" aria-busy={loading}>
        {#each recs as track (track.id)}
          {@const art = resolveArtworkUrl(track.artworkUrl, proxyPort)}
          {@const current = currentTrackId === track.id}
          <div class="row" class:current role="listitem">
            <button class="main" aria-label={`Play ${track.title}`} onclick={() => onPlay(track, recs)}>
              <span class="art">
                {#if art}<img src={art} alt="" loading="lazy" />{:else}<Icon name="music" size={16} />{/if}
                <span class="overlay"><Icon name={current && isPlaying ? "pause" : "play"} size={14} /></span>
              </span>
              <span class="meta">
                <span class="title">{track.title}</span>
                <span class="sub">{track.artist}{track.album ? ` • ${track.album}` : ""}</span>
              </span>
            </button>
            <HeartButton liked={likedIds.has(track.id)} size={16} onToggle={() => onToggleLike(track)} />
            <button class="btn-ghost add" title="Add to this playlist" onclick={() => add(track)}>
              <Icon name="plus" size={14} />
              <span>Add</span>
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>
{/if}

<style>
  .recs {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 32px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 8px;
  }

  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 750;
    letter-spacing: -0.02em;
  }

  header p,
  .note {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--p-text-dim);
  }

  .note {
    padding: 0 8px;
  }

  .list {
    display: flex;
    flex-direction: column;
  }

  .list[aria-busy="true"] {
    opacity: 0.55;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 6px 8px;
    border-radius: 8px;
  }

  .row:hover,
  .row:focus-within {
    background: var(--p-fill);
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
  }

  .art {
    position: relative;
    width: 44px;
    height: 44px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 6px;
    background: var(--p-fill);
    color: var(--p-text-dim);
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.5);
    color: #fff;
    opacity: 0;
    transition: opacity 0.14s ease;
  }

  .row:hover .overlay,
  .row.current .overlay {
    opacity: 1;
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .title,
  .sub {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 14px;
    font-weight: 600;
  }

  .row.current .title {
    color: var(--p-accent-bright);
  }

  .sub {
    font-size: 12.5px;
    color: var(--p-text-muted);
  }

  .add {
    flex-shrink: 0;
  }
</style>
