<script lang="ts">
  import { getDemoTracks } from "../lib/demoTracks";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { HomeFeed, Track } from "../lib/types";
  import Icon from "./Icon.svelte";
  import TrackCard from "./TrackCard.svelte";

  interface Props {
    feed: HomeFeed;
    loading: boolean;
    demoMode: boolean;
    history: Track[];
    favorites: Track[];
    likedIds: Set<string>;
    currentTrackId: string | null;
    isPlaying: boolean;
    proxyPort: number;
    onPlay: (track: Track, list: Track[]) => void;
    onToggleLike: (track: Track) => void;
    onOpenPlaylistPicker: (track: Track) => void;
    onRefresh: () => void;
    onGoSearch: () => void;
  }

  let {
    feed,
    loading,
    demoMode,
    history,
    favorites,
    likedIds,
    currentTrackId,
    isPlaying,
    proxyPort,
    onPlay,
    onToggleLike,
    onOpenPlaylistPicker,
    onRefresh,
    onGoSearch,
  }: Props = $props();

  const demoTracks = getDemoTracks();

  function art(track: Track): string | null {
    return resolveArtworkUrl(track.artworkUrl, proxyPort);
  }

  function greeting(): string {
    const h = new Date().getHours();
    if (h < 12) return "Good morning";
    if (h < 18) return "Good afternoon";
    return "Good evening";
  }

  let quickPlayConfig = $derived.by(() => {
    if (history.length > 0) {
      return { label: "Resume Listening", track: history[0], list: history };
    }
    if (favorites.length > 0) {
      return { label: "Play Favorites", track: favorites[0], list: favorites };
    }
    if (feed.quickPicks.length > 0) {
      return { label: "Play Quick Picks", track: feed.quickPicks[0], list: feed.quickPicks };
    }
    if (demoMode && demoTracks.length > 0) {
      return { label: "Play Demo", track: demoTracks[0], list: demoTracks };
    }
    return null;
  });
</script>

<div class="page">
  <header class="home-bar">
    <div class="home-greet">
      {#if demoMode}
        <span class="pill-badge">
          <Icon name="sparkles" size={12} />
          <span>Demo Mode</span>
        </span>
      {/if}
      <h1>{greeting()}</h1>
    </div>

    <div class="home-actions">
      {#if quickPlayConfig}
        <button
          class="btn-primary"
          onclick={() => onPlay(quickPlayConfig.track, quickPlayConfig.list)}
        >
          <Icon name="play" size={14} />
          <span>{quickPlayConfig.label}</span>
        </button>
      {/if}
      <button class="btn-ghost" onclick={onGoSearch}>
        <Icon name="search" size={14} />
        <span>Search</span>
      </button>
      <button
        class="icon-circle"
        title="Refresh feed"
        aria-label="Refresh home feed"
        onclick={onRefresh}
        disabled={loading}
      >
        <Icon name="refresh" size={15} spin={loading} />
      </button>
    </div>
  </header>

  {#if demoMode}
    <section class="section">
      <div class="sec-head">
        <h2>Demo Tracks</h2>
        <span class="sec-sub">Offline synthesized WAV + synced LRC test tracks (--demo)</span>
      </div>
      <div class="quick-grid">
        {#each demoTracks as track (track.id)}
          {@const isLiked = likedIds.has(track.id)}
          {@const isActive = currentTrackId === track.id}
          <div
            class="quick-card"
            class:active={isActive}
            role="button"
            tabindex="0"
            onclick={() => onPlay(track, demoTracks)}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onPlay(track, demoTracks);
              }
            }}
          >
            <div class="quick-art">
              {#if art(track)}
                <img src={art(track)} alt={track.title} />
              {:else}
                <Icon name="music" size={20} />
              {/if}
            </div>
            <div class="quick-meta">
              <div class="quick-title">{track.title}</div>
              <div class="quick-sub">{track.artist}</div>
            </div>
            <div class="quick-acts">
              <button
                class="mini-btn"
                class:liked={isLiked}
                title={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                aria-label={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                onclick={(e) => {
                  e.stopPropagation();
                  onToggleLike(track);
                }}
              >
                <Icon name={isLiked ? "heart-filled" : "heart"} size={13} />
              </button>
              <button
                class="quick-fab"
                class:stay={isActive}
                title={`Play ${track.title}`}
                aria-label={`Play ${track.title}`}
                onclick={(e) => {
                  e.stopPropagation();
                  onPlay(track, demoTracks);
                }}
              >
                <Icon name={isActive && isPlaying ? "music" : "play"} size={14} />
              </button>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if history.length > 0}
    <section class="section">
      <div class="sec-head">
        <h2>Jump Back In</h2>
      </div>
      <div class="quick-grid">
        {#each history.slice(0, 6) as track (track.id)}
          {@const isLiked = likedIds.has(track.id)}
          {@const isActive = currentTrackId === track.id}
          <div
            class="quick-card"
            class:active={isActive}
            role="button"
            tabindex="0"
            onclick={() => onPlay(track, history)}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onPlay(track, history);
              }
            }}
          >
            <div class="quick-art">
              {#if art(track)}
                <img src={art(track)} alt={track.title} loading="lazy" />
              {:else}
                <Icon name="music" size={20} />
              {/if}
            </div>
            <div class="quick-meta">
              <div class="quick-title">{track.title}</div>
              <div class="quick-sub">{track.artist}</div>
            </div>
            <div class="quick-acts">
              <button
                class="mini-btn"
                class:liked={isLiked}
                title={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                aria-label={isLiked ? "Remove from Favorites" : "Add to Favorites"}
                onclick={(e) => {
                  e.stopPropagation();
                  onToggleLike(track);
                }}
              >
                <Icon name={isLiked ? "heart-filled" : "heart"} size={13} />
              </button>
              <button
                class="quick-fab"
                class:stay={isActive}
                title={`Play ${track.title}`}
                aria-label={`Play ${track.title}`}
                onclick={(e) => {
                  e.stopPropagation();
                  onPlay(track, history);
                }}
              >
                <Icon name={isActive && isPlaying ? "music" : "play"} size={14} />
              </button>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <section class="section">
    <div class="sec-head">
      <h2>Quick Picks</h2>
      {#if feed.offlineFallback}
        <span class="sec-sub warn">Offline</span>
      {/if}
    </div>

    {#if loading && feed.quickPicks.length === 0}
      <div class="cards-grid">
        {#each Array(6) as _, i (i)}
          <div class="skel-card" aria-hidden="true">
            <div class="skel-art"></div>
            <div class="skel-line lg"></div>
            <div class="skel-line sm"></div>
          </div>
        {/each}
      </div>
    {:else if feed.quickPicks.length > 0}
      <div class="cards-grid">
        {#each feed.quickPicks.slice(0, 12) as track (track.id)}
          <TrackCard
            {track}
            artworkUrl={art(track)}
            isLiked={likedIds.has(track.id)}
            isActive={currentTrackId === track.id}
            {isPlaying}
            onPlay={() => onPlay(track, feed.quickPicks)}
            onToggleLike={() => onToggleLike(track)}
            onOpenPlaylistPicker={() => onOpenPlaylistPicker(track)}
          />
        {/each}
      </div>
    {:else}
      <div class="empty-box card-glass">
        Could not reach YouTube Music home feed right now. Try <strong>Search</strong> or add a
        <strong>Local Music Folder</strong> in Library.
      </div>
    {/if}
  </section>

  {#if feed.trending.length > 0}
    <section class="section">
      <div class="sec-head">
        <h2>Discover &amp; Trending</h2>
      </div>
      <div class="cards-grid">
        {#each feed.trending.slice(0, 12) as track (track.id)}
          <TrackCard
            {track}
            artworkUrl={art(track)}
            isLiked={likedIds.has(track.id)}
            isActive={currentTrackId === track.id}
            {isPlaying}
            onPlay={() => onPlay(track, feed.trending)}
            onToggleLike={() => onToggleLike(track)}
            onOpenPlaylistPicker={() => onOpenPlaylistPicker(track)}
          />
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .page {
    padding: 26px 30px 44px;
    display: flex;
    flex-direction: column;
    gap: 30px;
  }

  .home-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding-bottom: 4px;
  }

  .home-greet h1 {
    margin: 4px 0 0;
    font-size: 28px;
    font-weight: 760;
    letter-spacing: -0.025em;
  }

  .home-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }

  .icon-circle {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--p-border);
    color: var(--p-text-muted);
    display: grid;
    place-items: center;
    transition: background 0.16s ease, color 0.16s ease;
  }

  .icon-circle:hover {
    background: rgba(255, 255, 255, 0.13);
    color: var(--p-text);
  }

  .icon-circle:active,
  .mini-btn:active,
  .quick-fab:active {
    transform: scale(0.92);
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .sec-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }

  .sec-head h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.015em;
  }

  .sec-sub {
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .sec-sub.warn {
    color: #fbbf24;
  }

  .quick-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(238px, 1fr));
    gap: 12px;
  }

  .quick-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px 8px 8px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.07);
    backdrop-filter: blur(12px);
    cursor: pointer;
    transition:
      transform 0.18s ease,
      background 0.18s ease,
      border-color 0.18s ease,
      box-shadow 0.18s ease;
  }

  .quick-card:hover,
  .quick-card:focus-visible {
    outline: none;
    transform: translateY(-2px);
    background: rgba(255, 255, 255, 0.09);
    border-color: rgba(255, 255, 255, 0.16);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.28);
  }

  .quick-card.active {
    background: var(--p-accent-soft);
    border-color: var(--p-accent);
  }

  .quick-art {
    width: 56px;
    height: 56px;
    border-radius: 10px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.06);
    display: grid;
    place-items: center;
    flex-shrink: 0;
    color: var(--p-text-dim);
  }

  .quick-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .quick-meta {
    flex: 1;
    min-width: 0;
  }

  .quick-title {
    font-size: 13.5px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .quick-sub {
    font-size: 12px;
    color: var(--p-text-muted);
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .quick-acts {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .mini-btn {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    color: var(--p-text-muted);
    display: grid;
    place-items: center;
    opacity: 0;
    transition: opacity 0.15s ease, background 0.15s ease, color 0.15s ease;
  }

  .quick-card:hover .mini-btn,
  .quick-card:focus-within .mini-btn,
  .mini-btn.liked {
    opacity: 1;
  }

  .mini-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--p-text);
  }

  .mini-btn.liked {
    color: #fb7185;
  }

  .quick-fab {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--p-accent-bright), var(--p-accent));
    color: #080b12;
    display: grid;
    place-items: center;
    box-shadow: 0 4px 14px var(--p-glow-1);
    opacity: 0;
    transform: scale(0.88);
    transition: opacity 0.16s ease, transform 0.18s ease;
  }

  .quick-card:hover .quick-fab,
  .quick-card:focus-within .quick-fab,
  .quick-fab.stay {
    opacity: 1;
    transform: scale(1);
  }

  .cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
    gap: 16px;
  }

  .skel-card {
    padding: 11px 11px 14px;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .skel-art {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 12px;
    background: linear-gradient(110deg, rgba(255, 255, 255, 0.05) 30%, rgba(255, 255, 255, 0.11) 50%, rgba(255, 255, 255, 0.05) 70%);
    background-size: 200% 100%;
    animation: shimmer 1.4s infinite linear;
  }

  .skel-line {
    height: 11px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.07);
  }

  .skel-line.lg {
    width: 82%;
  }

  .skel-line.sm {
    width: 54%;
  }

  @keyframes shimmer {
    from {
      background-position: 200% 0;
    }
    to {
      background-position: -200% 0;
    }
  }

  .empty-box {
    padding: 20px;
    font-size: 13px;
    color: var(--p-text-muted);
  }
</style>
