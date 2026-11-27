<script lang="ts">
  import { untrack } from "svelte";
  import { ytmSearch } from "../lib/ipc";
  import type { Track } from "../lib/types";
  import Icon from "./Icon.svelte";
  import TrackTable from "./TrackTable.svelte";

  /** Results for the title bar's search field. Typing searches after a short pause; Enter is immediate. */
  interface Props {
    query: string;
    /** Bumped by Enter in the search field to skip the typing pause. */
    submitSeq: number;
    localTracks: Track[];
    likedIds: Set<string>;
    cachedIds: Set<string>;
    currentTrackId: string | null;
    isPlaying: boolean;
    proxyPort: number;
    onPlay: (track: Track, list: Track[]) => void;
    onEnqueue: (track: Track) => void;
    onToggleLike: (track: Track) => void;
    onDownload: (track: Track) => void;
    onOpenPlaylistPicker: (track: Track) => void;
  }

  let {
    query,
    submitSeq,
    localTracks,
    likedIds,
    cachedIds,
    currentTrackId,
    isPlaying,
    proxyPort,
    onPlay,
    onEnqueue,
    onToggleLike,
    onDownload,
    onOpenPlaylistPicker,
  }: Props = $props();

  type Filter = "songs" | "videos" | "local";
  const FILTERS: { id: Filter; label: string }[] = [
    { id: "songs", label: "Songs" },
    { id: "videos", label: "Videos" },
    { id: "local", label: "Local files" },
  ];
  const TYPING_PAUSE_MS = 350;

  let filter = $state<Filter>("songs");
  let searching = $state(false);
  let results = $state<Track[]>([]);
  let error = $state<string | null>(null);
  let searchedFor = $state("");
  let request = 0;
  let lastSubmit = 0;

  let trimmed = $derived(query.trim());

  let localMatches = $derived.by(() => {
    const q = trimmed.toLowerCase();
    if (!q) return localTracks;
    return localTracks.filter((t) => `${t.title} ${t.artist} ${t.album}`.toLowerCase().includes(q));
  });

  async function run(q: string, kind: Filter) {
    const seq = ++request;
    searching = true;
    error = null;
    try {
      const res = await ytmSearch(q, kind);
      if (seq !== request) return;
      results = res.tracks;
      error = res.error;
      searchedFor = q;
    } catch (e) {
      if (seq === request) error = e instanceof Error ? e.message : String(e);
    } finally {
      if (seq === request) searching = false;
    }
  }

  // Remote search follows the query and filter, after a pause (or at once on Enter).
  $effect(() => {
    const q = trimmed;
    const kind = filter;
    const immediate = submitSeq !== lastSubmit;
    lastSubmit = submitSeq;
    if (kind === "local" || !q) {
      request++;
      searching = false;
      if (!q) results = [];
      return;
    }
    // Results and errors are read without tracking, so a failed search does not re-run itself.
    if (!immediate && untrack(() => q === searchedFor && !error)) return;
    const timer = window.setTimeout(() => void run(q, kind), immediate ? 0 : TYPING_PAUSE_MS);
    return () => window.clearTimeout(timer);
  });

  let shown = $derived(filter === "local" ? localMatches : results);
</script>

<div class="page">
  <nav class="filters" aria-label="Search in">
    {#each FILTERS as item (item.id)}
      <button
        class="filter"
        class:active={filter === item.id}
        aria-pressed={filter === item.id}
        onclick={() => {
          filter = item.id;
          searchedFor = "";
        }}
      >
        {item.label}
        {#if item.id === "local"}<span class="n">{localMatches.length}</span>{/if}
      </button>
    {/each}
    {#if searching}
      <span class="status"><Icon name="loader" size={14} spin /> Searching…</span>
    {/if}
  </nav>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if !trimmed && filter !== "local"}
    <div class="hint">
      <Icon name="search" size={28} />
      <strong>Search YouTube Music</strong>
      <span>Type in the bar above — songs, artists or albums. Ctrl+K jumps there from anywhere.</span>
    </div>
  {:else}
    <TrackTable
      tracks={shown}
      {likedIds}
      {cachedIds}
      {currentTrackId}
      {isPlaying}
      {proxyPort}
      filterFrom={0}
      emptyText={searching ? "Searching…" : filter === "local" ? "No local songs match." : `No results for “${trimmed}”.`}
      {onPlay}
      {onToggleLike}
      onAddToPlaylist={onOpenPlaylistPicker}
      {onEnqueue}
      {onDownload}
    />
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 22px 24px 48px;
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
  }

  .filter {
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

  .filter:hover {
    background: color-mix(in srgb, var(--p-text) 14%, var(--p-fill));
  }

  .filter.active {
    background: var(--p-text);
    color: var(--p-bg);
  }

  .n {
    font-size: 11.5px;
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: 6px;
    font-size: 12.5px;
    color: var(--p-text-dim);
  }

  .error {
    margin: 0 8px;
    color: var(--p-danger);
    font-size: 13px;
  }

  .hint {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 72px 20px;
    color: var(--p-text-dim);
    font-size: 13px;
    text-align: center;
  }

  .hint strong {
    color: var(--p-text);
    font-size: 18px;
  }
</style>
