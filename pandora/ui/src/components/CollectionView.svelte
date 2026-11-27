<script lang="ts">
  import { formatCount, formatTotalDuration } from "../lib/format";
  import { ytmCollection } from "../lib/ipc";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { CollectionCard, CollectionPage, Track } from "../lib/types";
  import Icon from "./Icon.svelte";
  import MoreMenu from "./MoreMenu.svelte";
  import PageHero from "./PageHero.svelte";
  import TrackTable from "./TrackTable.svelte";

  /** A YouTube Music playlist, album or artist opened from the home feed. */
  interface Props {
    card: CollectionCard;
    likedIds: Set<string>;
    cachedIds: Set<string>;
    currentTrackId: string | null;
    isPlaying: boolean;
    proxyPort: number;
    onBack: () => void;
    onPlay: (track: Track, list: Track[]) => void;
    onPlayShuffled: (list: Track[]) => void;
    onToggleLike: (track: Track) => void;
    onAddToPlaylist: (track: Track) => void;
    onDownload: (track: Track) => void;
    onDownloadAll: (tracks: Track[]) => void;
    onSave: (name: string, tracks: Track[]) => void;
  }

  let {
    card,
    likedIds,
    cachedIds,
    currentTrackId,
    isPlaying,
    proxyPort,
    onBack,
    onPlay,
    onPlayShuffled,
    onToggleLike,
    onAddToPlaylist,
    onDownload,
    onDownloadAll,
    onSave,
  }: Props = $props();

  let page = $state<CollectionPage | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let request = 0;

  async function load(browseId: string) {
    const seq = ++request;
    loading = true;
    error = null;
    page = null;
    try {
      const next = await ytmCollection(browseId);
      if (seq === request) page = next;
    } catch (e) {
      if (seq === request) error = e instanceof Error ? e.message : String(e);
    } finally {
      if (seq === request) loading = false;
    }
  }

  $effect(() => {
    void load(card.browseId);
  });

  let tracks = $derived(page?.tracks ?? []);
  let cover = $derived(resolveArtworkUrl(page?.artworkUrl ?? card.artworkUrl, proxyPort));
  let label = $derived(card.kind === "album" ? "Album" : card.kind === "artist" ? "Artist" : "Playlist");
  let meta = $derived.by(() => {
    if (!page) return card.subtitle;
    const parts = [page.subtitle || card.subtitle];
    if (!/\bsongs?\b/i.test(parts[0])) parts.push(formatCount(tracks.length, "song"));
    const total = formatTotalDuration(tracks);
    if (total && !/\b(minutes?|hours?)\b/i.test(parts[0])) parts.push(total);
    return parts.filter(Boolean).join(" • ");
  });
</script>

<button class="back btn-ghost" onclick={onBack}>
  <Icon name="arrow-left" size={14} />
  <span>Home</span>
</button>

<PageHero covers={cover ? [cover] : []} round={card.kind === "artist"} {label} title={page?.title ?? card.title} {meta}>
  <button
    class="btn-play"
    aria-label="Play"
    disabled={tracks.length === 0}
    onclick={() => onPlay(tracks[0], tracks)}
  >
    <Icon name="play" size={22} />
  </button>
  <button class="btn-icon" title="Shuffle" aria-label="Shuffle" disabled={tracks.length === 0} onclick={() => onPlayShuffled(tracks)}>
    <Icon name="shuffle" size={17} />
  </button>
  <button class="btn-ghost" disabled={tracks.length === 0} onclick={() => onSave(page?.title ?? card.title, tracks)}>
    <Icon name="plus" size={15} />
    <span>Save to library</span>
  </button>
  <MoreMenu
    items={[
      {
        label: `Download ${formatCount(tracks.filter((t) => !cachedIds.has(t.id)).length, "song")}`,
        icon: "download",
        disabled: tracks.every((t) => cachedIds.has(t.id)),
        onSelect: () => onDownloadAll(tracks.filter((t) => !cachedIds.has(t.id))),
      },
    ]}
  />
</PageHero>

<section class="body">
  {#if loading}
    <div class="loading"><Icon name="loader" size={18} spin /> Loading tracks…</div>
  {:else if error}
    <div class="error">
      <span>{error}</span>
      <button class="btn-ghost" onclick={() => void load(card.browseId)}>Try again</button>
    </div>
  {:else}
    <TrackTable
      {tracks}
      {likedIds}
      {cachedIds}
      {currentTrackId}
      {isPlaying}
      {proxyPort}
      showAlbum={card.kind !== "album"}
      {onPlay}
      {onToggleLike}
      {onAddToPlaylist}
      {onDownload}
    />
  {/if}
</section>

<style>
  .back {
    margin: 20px 32px 0;
    height: 32px;
    padding: 0 14px;
  }

  .body {
    padding: 0 24px 48px;
  }

  .loading,
  .error {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 24px 8px;
    color: var(--p-text-muted);
    font-size: 13px;
  }

  .error {
    color: var(--p-danger);
  }
</style>
