<script lang="ts">
  import type { Track } from "../lib/types";
  import HeartButton from "./HeartButton.svelte";
  import Icon from "./Icon.svelte";

  /** Compact song row for home shelves (Quick picks): cover with a hover play overlay, title, artist, actions. */
  interface Props {
    track: Track;
    artworkUrl: string | null;
    isLiked: boolean;
    isCurrent: boolean;
    isPlaying: boolean;
    onPlay: () => void;
    onToggleLike: () => void;
    onAddToPlaylist: () => void;
  }

  let { track, artworkUrl, isLiked, isCurrent, isPlaying, onPlay, onToggleLike, onAddToPlaylist }: Props = $props();
</script>

<div class="song" class:current={isCurrent}>
  <button class="main" onclick={onPlay} aria-label={`Play ${track.title}`}>
    <span class="art">
      {#if artworkUrl}
        <img src={artworkUrl} alt="" loading="lazy" />
      {:else}
        <Icon name="music" size={18} />
      {/if}
      <span class="overlay"><Icon name={isCurrent && isPlaying ? "pause" : "play"} size={16} /></span>
    </span>
    <span class="meta">
      <span class="title">{track.title}</span>
      <span class="sub">{track.artist}{track.album ? ` • ${track.album}` : ""}</span>
    </span>
  </button>
  <span class="acts">
    <HeartButton liked={isLiked} size={16} onToggle={onToggleLike} />
    <button class="btn-icon sm" aria-label="Add to playlist" title="Add to playlist" onclick={onAddToPlaylist}>
      <Icon name="list-plus" size={15} />
    </button>
  </span>
</div>

<style>
  .song {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    padding: 6px 8px 6px 6px;
    border-radius: 8px;
  }

  .song:hover,
  .song:focus-within {
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
    width: 48px;
    height: 48px;
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

  .song:hover .overlay,
  .song.current .overlay {
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

  .song.current .title {
    color: var(--p-accent-bright);
  }

  .sub {
    font-size: 12.5px;
    color: var(--p-text-muted);
  }

  .acts {
    display: flex;
    flex-shrink: 0;
  }

  .acts :global(.btn-icon:not(.keep)) {
    opacity: 0;
  }

  .song:hover .acts :global(.btn-icon),
  .song:focus-within .acts :global(.btn-icon) {
    opacity: 1;
  }
</style>
