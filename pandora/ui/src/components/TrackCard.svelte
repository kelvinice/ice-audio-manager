<script lang="ts">
  import type { Track } from "../lib/types";
  import Icon from "./Icon.svelte";

  interface Props {
    track: Track;
    artworkUrl: string | null;
    isLiked: boolean;
    isActive: boolean;
    isPlaying: boolean;
    onPlay: () => void;
    onToggleLike: () => void;
    onOpenPlaylistPicker: () => void;
  }

  let {
    track,
    artworkUrl,
    isLiked,
    isActive,
    isPlaying,
    onPlay,
    onToggleLike,
    onOpenPlaylistPicker,
  }: Props = $props();
</script>

<div
  class="card"
  class:active={isActive}
  role="button"
  tabindex="0"
  onclick={onPlay}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onPlay();
    }
  }}
>
  <div class="art-box">
    {#if artworkUrl}
      <img src={artworkUrl} alt={track.title} loading="lazy" />
    {:else}
      <div class="art-fallback">
        <Icon name="music" size={32} />
      </div>
    {/if}

    <div class="art-scrim"></div>

    <div class="top-pill" class:visible={isLiked}>
      <button
        class="pill-btn"
        class:liked={isLiked}
        title={isLiked ? "Remove from Favorites" : "Add to Favorites"}
        aria-label={isLiked ? "Remove from Favorites" : "Add to Favorites"}
        onclick={(e) => {
          e.stopPropagation();
          onToggleLike();
        }}
      >
        <Icon name={isLiked ? "heart-filled" : "heart"} size={13} />
      </button>
      <button
        class="pill-btn"
        title="Add to Playlist"
        aria-label="Add to Playlist"
        onclick={(e) => {
          e.stopPropagation();
          onOpenPlaylistPicker();
        }}
      >
        <Icon name="list-plus" size={13} />
      </button>
    </div>

    <button
      class="fab-play"
      class:stay={isActive}
      title={isActive && isPlaying ? "Now Playing" : `Play ${track.title}`}
      aria-label={`Play ${track.title}`}
      onclick={(e) => {
        e.stopPropagation();
        onPlay();
      }}
    >
      {#if isActive && isPlaying}
        <span class="eq-bars" aria-hidden="true">
          <span></span>
          <span></span>
          <span></span>
        </span>
      {:else}
        <Icon name="play" size={16} />
      {/if}
    </button>
  </div>

  <div class="card-meta">
    <div class="card-title">{track.title}</div>
    <div class="card-sub">{track.artist}</div>
  </div>
</div>

<style>
  .card {
    position: relative;
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

  .card:hover,
  .card:focus-visible {
    outline: none;
    transform: translateY(-3px);
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.16);
    box-shadow: 0 14px 30px rgba(0, 0, 0, 0.34);
  }

  .card.active {
    background: linear-gradient(180deg, var(--p-surface-tint), rgba(255, 255, 255, 0.05));
    border-color: var(--p-accent);
    box-shadow: 0 10px 26px var(--p-glow-1);
  }

  .art-box {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: 12px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.05);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.36);
  }

  .art-box img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: transform 0.35s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .card:hover .art-box img {
    transform: scale(1.05);
  }

  .art-fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--p-text-dim);
  }

  .art-scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(6, 9, 16, 0.15) 0%, transparent 45%, rgba(6, 9, 16, 0.58) 100%);
    opacity: 0;
    transition: opacity 0.22s ease;
    pointer-events: none;
  }

  .card:hover .art-scrim,
  .card.active .art-scrim {
    opacity: 1;
  }

  .top-pill {
    position: absolute;
    top: 8px;
    right: 8px;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    border-radius: 999px;
    background: rgba(10, 13, 24, 0.72);
    border: 1px solid rgba(255, 255, 255, 0.14);
    backdrop-filter: blur(12px);
    opacity: 0;
    transform: translateY(-4px);
    transition: opacity 0.18s ease, transform 0.2s ease;
  }

  .card:hover .top-pill,
  .card:focus-within .top-pill,
  .top-pill.visible {
    opacity: 1;
    transform: translateY(0);
  }

  .pill-btn {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: rgba(243, 245, 252, 0.85);
    transition: background 0.14s ease, color 0.14s ease;
  }

  .pill-btn:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #ffffff;
  }

  .pill-btn.liked {
    color: #fb7185;
  }

  .fab-play {
    position: absolute;
    right: 10px;
    bottom: 10px;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--p-accent-bright), var(--p-accent));
    color: #080b12;
    display: grid;
    place-items: center;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.45);
    opacity: 0;
    transform: translateY(8px) scale(0.92);
    transition:
      opacity 0.2s ease,
      transform 0.24s cubic-bezier(0.22, 1, 0.36, 1),
      filter 0.15s ease;
  }

  .card:hover .fab-play,
  .card:focus-within .fab-play,
  .fab-play.stay {
    opacity: 1;
    transform: translateY(0) scale(1);
  }

  .fab-play:hover {
    transform: scale(1.07);
    filter: brightness(1.08);
  }

  .eq-bars {
    display: inline-flex;
    align-items: flex-end;
    gap: 2.5px;
    height: 14px;
  }

  .eq-bars span {
    width: 3px;
    border-radius: 2px;
    background: #080b12;
    animation: cardEq 0.75s ease-in-out infinite alternate;
  }

  .eq-bars span:nth-child(2) {
    animation-delay: 0.22s;
  }

  .eq-bars span:nth-child(3) {
    animation-delay: 0.44s;
  }

  @keyframes cardEq {
    from {
      height: 4px;
    }
    to {
      height: 14px;
    }
  }

  .card-meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 0 2px;
  }

  .card-title {
    font-size: 13.5px;
    font-weight: 650;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-sub {
    font-size: 12px;
    color: var(--p-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
