<script lang="ts">
  import { getDemoTracks } from "../lib/demoTracks";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { HomeFeed, Track } from "../lib/types";

  interface Props {
    feed: HomeFeed;
    loading: boolean;
    history: Track[];
    currentTrackId: string | null;
    proxyPort: number;
    onPlay: (track: Track, list: Track[]) => void;
    onRefresh: () => void;
    onGoSearch: () => void;
  }

  let {
    feed,
    loading,
    history,
    currentTrackId,
    proxyPort,
    onPlay,
    onRefresh,
    onGoSearch,
  }: Props = $props();

  const demoTracks = getDemoTracks();

  function art(track: Track): string | null {
    return resolveArtworkUrl(track.artworkUrl, proxyPort);
  }
</script>

<div class="page">
  <header class="hero card-glass">
    <div>
      <div class="pill-badge">Pandora Prototype • Vivi-Style Windows Client</div>
      <h1>Expressive listening, native Windows audio control.</h1>
      <p>
        Dynamic Material You colors from album art, ad-free YouTube Music streaming, synced karaoke lyrics,
        10-band Web Audio EQ, and seamless Ice Audio Manager integration.
      </p>
      <div class="hero-actions">
        <button class="btn-primary" onclick={() => onPlay(demoTracks[0], demoTracks)}>
          ▶ Play Instant Demo
        </button>
        <button class="btn-ghost" onclick={onGoSearch}>Search YouTube Music</button>
        <button class="btn-ghost" onclick={onRefresh} disabled={loading}>
          {loading ? "Refreshing..." : "Refresh Feed"}
        </button>
      </div>
    </div>
  </header>

  <section class="section">
    <div class="sec-head">
      <h2>Prototype Demo Tracks</h2>
      <span class="sec-sub">Zero-latency offline audio + Material You palette + synced lyrics</span>
    </div>
    <div class="demo-grid">
      {#each demoTracks as track (track.id)}
        <button
          class="demo-card card-glass"
          class:playing={currentTrackId === track.id}
          onclick={() => onPlay(track, demoTracks)}
        >
          {#if art(track)}
            <img src={art(track)} alt={track.title} />
          {/if}
          <div class="demo-meta">
            <div class="title">{track.title}</div>
            <div class="sub">{track.artist}</div>
            <div class="tag">Demo WAV • Synced LRC</div>
          </div>
          <span class="play-badge">{currentTrackId === track.id ? "♫" : "▶"}</span>
        </button>
      {/each}
    </div>
  </section>

  {#if history.length > 0}
    <section class="section">
      <div class="sec-head">
        <h2>Recently Played</h2>
        <span class="sec-sub">Saved locally • zero telemetry</span>
      </div>
      <div class="cards-row">
        {#each history.slice(0, 8) as track (track.id)}
          <button
            class="tile card-glass"
            class:playing={currentTrackId === track.id}
            onclick={() => onPlay(track, history)}
          >
            <div class="tile-art">
              {#if art(track)}
                <img src={art(track)} alt={track.title} />
              {:else}
                <div class="art-fallback">♫</div>
              {/if}
            </div>
            <div class="tile-title">{track.title}</div>
            <div class="tile-sub">{track.artist}</div>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <section class="section">
    <div class="sec-head">
      <h2>YouTube Music Quick Picks</h2>
      {#if feed.offlineFallback}
        <span class="sec-sub warn">Offline or blocked — use Demo Tracks, Local Files, or Search</span>
      {:else}
        <span class="sec-sub">Live InnerTube catalog</span>
      {/if}
    </div>
    {#if feed.quickPicks.length > 0}
      <div class="cards-row">
        {#each feed.quickPicks.slice(0, 12) as track (track.id)}
          <button
            class="tile card-glass"
            class:playing={currentTrackId === track.id}
            onclick={() => onPlay(track, feed.quickPicks)}
          >
            <div class="tile-art">
              {#if art(track)}
                <img src={art(track)} alt={track.title} />
              {:else}
                <div class="art-fallback">♫</div>
              {/if}
            </div>
            <div class="tile-title">{track.title}</div>
            <div class="tile-sub">{track.artist}</div>
          </button>
        {/each}
      </div>
    {:else if !loading}
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
      <div class="cards-row">
        {#each feed.trending.slice(0, 12) as track (track.id)}
          <button
            class="tile card-glass"
            class:playing={currentTrackId === track.id}
            onclick={() => onPlay(track, feed.trending)}
          >
            <div class="tile-art">
              {#if art(track)}
                <img src={art(track)} alt={track.title} />
              {:else}
                <div class="art-fallback">♫</div>
              {/if}
            </div>
            <div class="tile-title">{track.title}</div>
            <div class="tile-sub">{track.artist}</div>
          </button>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .page {
    padding: 24px 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }

  .hero {
    padding: 26px 28px;
    background: linear-gradient(135deg, var(--p-surface-tint), rgba(16, 20, 32, 0.84));
  }

  .hero h1 {
    margin: 12px 0 8px;
    font-size: 28px;
    font-weight: 750;
    letter-spacing: -0.025em;
  }

  .hero p {
    margin: 0 0 18px;
    max-width: 680px;
    color: var(--p-text-muted);
    font-size: 14px;
    line-height: 1.55;
  }

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
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
  }

  .sec-sub {
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .sec-sub.warn {
    color: #fbbf24;
  }

  .demo-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 12px;
  }

  .demo-card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px;
    text-align: left;
    transition: transform 0.16s ease, border-color 0.16s ease;
  }

  .demo-card:hover,
  .tile:hover {
    transform: translateY(-2px);
    border-color: var(--p-border-strong);
  }

  .demo-card.playing,
  .tile.playing {
    border-color: var(--p-accent);
    box-shadow: 0 0 0 1px var(--p-accent-soft);
  }

  .demo-card img {
    width: 56px;
    height: 56px;
    border-radius: 10px;
    object-fit: cover;
  }

  .demo-meta {
    flex: 1;
    min-width: 0;
  }

  .demo-meta .title {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .demo-meta .sub {
    font-size: 12px;
    color: var(--p-text-muted);
    margin-top: 2px;
  }

  .demo-meta .tag {
    font-size: 10px;
    color: var(--p-accent-bright);
    margin-top: 4px;
  }

  .play-badge {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    display: grid;
    place-items: center;
    font-size: 12px;
  }

  .cards-row {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(156px, 1fr));
    gap: 14px;
  }

  .tile {
    padding: 12px;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition: transform 0.16s ease, border-color 0.16s ease;
  }

  .tile-art {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 10px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.05);
  }

  .tile-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .art-fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    font-size: 28px;
    color: var(--p-text-dim);
  }

  .tile-title {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tile-sub {
    font-size: 12px;
    color: var(--p-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty-box {
    padding: 20px;
    font-size: 13px;
    color: var(--p-text-muted);
  }
</style>
