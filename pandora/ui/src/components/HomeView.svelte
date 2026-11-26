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
    onOpenLocal: () => void;
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
    onOpenLocal,
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
      return { label: "Recently played", track: history[0], list: history };
    }
    if (favorites.length > 0) {
      return { label: "From your favorites", track: favorites[0], list: favorites };
    }
    if (feed.quickPicks.length > 0) {
      return { label: "Quick pick", track: feed.quickPicks[0], list: feed.quickPicks };
    }
    if (demoMode && demoTracks.length > 0) {
      return { label: "From the demo collection", track: demoTracks[0], list: demoTracks };
    }
    return null;
  });
</script>

<div class="page">
  <header class="home-bar">
    <div class="home-greet">
      {#if quickPlayConfig}
        <p class="feature-label">{quickPlayConfig.label}</p>
        <h1>{quickPlayConfig.track.title}</h1>
        <p class="artist">{quickPlayConfig.track.artist}</p>
      {:else}
        <h1>{greeting()}</h1>
      {/if}
    </div>

    <div class="home-actions">
      {#if quickPlayConfig}
        <button
          class="btn-primary"
          onclick={() => onPlay(quickPlayConfig.track, quickPlayConfig.list)}
        >
          <Icon name="play" size={14} />
          <span>Play now</span>
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
    <div class="sleeve" aria-hidden="true">
      <div class="disc"></div>
      {#if quickPlayConfig && art(quickPlayConfig.track)}
        <img src={art(quickPlayConfig.track)} alt="" />
      {/if}
    </div>
  </header>

  {#if demoMode}
    <section class="section">
      <div class="sec-head">
        <h2>Demo Tracks</h2>

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
        <span>The music feed is unavailable. Your local library is still here.</span>
        <button class="btn-ghost" onclick={onOpenLocal}><Icon name="library" size={15} />Open local library</button>
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
    position: relative;
    isolation: isolate;
    overflow: hidden;
    display: grid;
    align-content: center;
    justify-items: start;
    min-height: 196px;
    gap: 18px;
    padding: 26px clamp(20px, 3vw, 40px);
    padding-right: 35%;
    border-radius: 18px;
    border: 1px solid var(--p-border);
    background: linear-gradient(105deg, var(--p-surface) 6%, var(--p-surface-tint) 90%);
  }

  .home-greet { min-width: 0; max-width: 100%; }
  .home-greet h1 {
    margin: 0;
    font-size: clamp(28px, 3.2vw, 48px);
    line-height: 1.06;
    letter-spacing: -.045em;
    overflow-wrap: anywhere;
  }
  .artist { margin: 6px 0 0; color: var(--p-text-muted); }
  .feature-label { margin: 0 0 6px; font-size: 13px; font-weight: 650; color: var(--p-accent-bright); }
  .sleeve { position: absolute; z-index: -1; right: 0; top: 0; bottom: 0; width: 33%; overflow: hidden; }
  .disc { position: absolute; width: clamp(160px, 22vw, 310px); aspect-ratio: 1; right: -9%; top: 50%;
    transform: translateY(-50%); border-radius: 50%; border: 2px solid #525065;
    background: radial-gradient(circle, #13111b 0 2%, var(--p-seed) 2.5% 23%, transparent 23.5%),
      conic-gradient(from 30deg, transparent, #d4c7ed55 12%, transparent 28% 50%, #d4c7ed55 64%, transparent 78%),
      repeating-radial-gradient(circle, #23212c 0 2px, #393645 2.5px 3.5px, #1c1a24 4px 5px);
  }
  .sleeve img { position: absolute; height: 78%; max-width: 71%; aspect-ratio: 1; object-fit: cover;
    left: 0; top: 11%; border-radius: 4px; transform: rotate(-6deg); box-shadow: 8px 10px 24px #100b2238; }
  @media (max-width: 720px) { .home-bar { padding-right: 22%; } .sleeve { width: 21%; } .sleeve img { display: none; } }

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
    background: var(--p-fill);
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
    color: var(--p-warning);
  }

  .quick-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 245px), 1fr));
    gap: 12px;
  }

  .quick-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px 8px 8px;
    border-radius: 14px;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
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
    background: var(--p-fill);
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
    background: var(--p-fill);
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
    background: var(--p-fill);
    color: var(--p-text);
  }

  .mini-btn.liked {
    color: var(--p-danger);
  }

  .quick-fab {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--p-accent-bright), var(--p-accent));
    color: var(--p-on-accent);
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
    background: var(--p-fill);
    border: 1px solid var(--p-border);
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
    background: var(--p-fill);
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
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 20px;
    font-size: 13px;
    color: var(--p-text-muted);
  }
</style>
