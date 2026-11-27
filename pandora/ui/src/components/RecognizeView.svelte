<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    cancelRecognition,
    clearRecognitionHistory,
    deleteRecognitionItem,
    getLibrary,
    onRecognitionProgress,
    recognizeMusic,
  } from "../lib/ipc";
  import { resolveArtworkUrl } from "../lib/theme";
  import type {
    LibraryState,
    RecognitionProgress,
    RecognizedTrack,
    Track,
  } from "../lib/types";
  import HeartButton from "./HeartButton.svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    history: RecognizedTrack[];
    likedIds: Set<string>;
    proxyPort: number;
    onPlay: (track: Track, list?: Track[]) => void;
    onSearchQuery: (query: string) => void;
    onToggleLike: (track: Track) => void;
    onAddToPlaylist: (track: Track) => void;
    onHistoryChange: (nextLib: LibraryState) => void;
    onNotify: (msg: string) => void;
  }

  let {
    history,
    likedIds,
    proxyPort,
    onPlay,
    onSearchQuery,
    onToggleLike,
    onAddToPlaylist,
    onHistoryChange,
    onNotify,
  }: Props = $props();

  let source = $state<"system" | "mic">("system");
  let active = $state(false);
  let progress = $state<RecognitionProgress>({
    state: "done",
    elapsedMs: 0,
    level: 0,
  });
  let latest = $state<RecognizedTrack | null>(null);
  let showLyrics = $state(false);
  let error = $state<string | null>(null);
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    unlisten = await onRecognitionProgress((p) => {
      progress = p;
    });
  });

  onDestroy(() => {
    unlisten?.();
    if (active) {
      void cancelRecognition().catch(() => {});
    }
  });

  async function toggleRecognize() {
    if (active) {
      await cancelRecognition().catch(() => {});
      active = false;
      return;
    }
    active = true;
    error = null;
    showLyrics = false;
    progress = { state: "listening", elapsedMs: 0, level: 0 };
    try {
      const res = await recognizeMusic(source);
      latest = res;
      onNotify(`Identified: ${res.title}`);
      void getLibrary().then(onHistoryChange).catch(() => {});
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (!msg.toLowerCase().includes("cancel")) {
        error = msg;
      }
    } finally {
      active = false;
    }
  }

  async function removeItem(id: string) {
    const next = await deleteRecognitionItem(id);
    onHistoryChange(next);
    if (latest?.id === id) latest = null;
  }

  async function clearAll() {
    const next = await clearRecognitionHistory();
    onHistoryChange(next);
    latest = null;
  }

  function formatAgo(ms: number): string {
    const sec = Math.max(0, Math.floor((Date.now() - ms) / 1000));
    if (sec < 60) return "Just now";
    if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
    if (sec < 86400) return `${Math.floor(sec / 3600)}h ago`;
    return `${Math.floor(sec / 86400)}d ago`;
  }

  let ringScale = $derived(1 + (active ? progress.level * 0.45 : 0));
  let elapsedSec = $derived((progress.elapsedMs / 1000).toFixed(1));
</script>

<div class="page">
  <header class="top-head">
    <h1>Identify Song</h1>
    <div class="source-switch" role="group" aria-label="Audio capture source">
      <button
        class="chip"
        class:active={source === "system"}
        disabled={active}
        onclick={() => (source = "system")}
      >
        <Icon name="speaker" size={14} />
        <span>System Audio</span>
      </button>
      <button
        class="chip"
        class:active={source === "mic"}
        disabled={active}
        onclick={() => (source = "mic")}
      >
        <Icon name="mic" size={14} />
        <span>Microphone</span>
      </button>
    </div>
  </header>

  <section class="stage card-glass">
    <div class="pulse-wrap">
      {#if active}
        <span class="ring r1" style="transform: scale({ringScale})"></span>
        <span class="ring r2" style="transform: scale({1 + progress.level * 0.72})"></span>
        <span class="ring r3"></span>
      {/if}
      <button
        class="listen-btn"
        class:listening={active}
        aria-label={active ? "Stop listening" : "Listen to identify song"}
        onclick={() => void toggleRecognize()}
      >
        <Icon name={active ? "loader" : source === "mic" ? "mic" : "radar"} size={34} spin={active} />
      </button>
    </div>

    <div class="stage-label">
      {#if active}
        <strong>
          {progress.state === "matching" ? "Matching..." : "Listening..."}
        </strong>
        <span>{elapsedSec}s / 10.0s • Tap to stop</span>
      {:else}
        <strong>Tap to identify</strong>
        <span>{source === "system" ? "Uses desktop audio output" : "Uses default microphone"}</span>
      {/if}
    </div>
  </section>

  {#if error}
    <div class="err-box card-glass">{error}</div>
  {/if}

  {#if latest}
    {@const art = resolveArtworkUrl(latest.artworkUrl, proxyPort)}
    <section class="result-card card-glass">
      <div class="result-art">
        {#if art}
          <img src={art} alt={latest.title} />
        {:else}
          <Icon name="music" size={34} />
        {/if}
      </div>
      <div class="result-body">
        <div class="result-meta">
          <div class="badge-row">
            <span class="pill-badge">Match Found</span>
            {#if latest.genre}<span class="meta-pill">{latest.genre}</span>{/if}
            {#if latest.releaseDate}<span class="meta-pill">{latest.releaseDate}</span>{/if}
          </div>
          <h2>{latest.title}</h2>
          <p>{latest.artist}{latest.album ? ` • ${latest.album}` : ""}</p>
        </div>

        <div class="result-actions">
          {#if latest.matchedTrack}
            {@const m = latest.matchedTrack}
            {@const isLiked = likedIds.has(m.id)}
            <button class="btn-primary" onclick={() => onPlay(m, [m])}>
              <Icon name="play" size={14} />
              <span>Play</span>
            </button>
            <HeartButton liked={isLiked} size={16} buttonClass="btn-icon sm" keep onToggle={() => onToggleLike(m)} />
            <button
              class="act-btn"
              title="Add to Playlist"
              aria-label="Add to Playlist"
              onclick={() => onAddToPlaylist(m)}
            >
              <Icon name="list-plus" size={15} />
            </button>
          {/if}
          <button
            class="btn-ghost"
            onclick={() => onSearchQuery(`${latest?.title ?? ""} ${latest?.artist ?? ""}`)}
          >
            <Icon name="search" size={14} />
            <span>Search</span>
          </button>
          {#if latest.lyricsSnippet}
            <button class="btn-ghost" onclick={() => (showLyrics = !showLyrics)}>
              <Icon name="lyrics" size={14} />
              <span>{showLyrics ? "Hide Lyrics" : "Lyrics"}</span>
            </button>
          {/if}
        </div>

        {#if showLyrics && latest.lyricsSnippet}
          <pre class="snippet">{latest.lyricsSnippet}</pre>
        {/if}
      </div>
    </section>
  {/if}

  <section class="history-sec">
    <div class="sec-head">
      <h2>Recent Matches ({history.length})</h2>
      {#if history.length > 0}
        <button class="btn-ghost" onclick={() => void clearAll()}>
          <Icon name="trash" size={14} />
          <span>Clear History</span>
        </button>
      {/if}
    </div>

    {#if history.length === 0}
      <div class="empty card-glass">Identified tracks will appear here.</div>
    {:else}
      <div class="hist-list card-glass">
        {#each history as item (item.id)}
          {@const art = resolveArtworkUrl(item.artworkUrl, proxyPort)}
          <div class="hist-row">
            <div class="hist-thumb">
              {#if art}
                <img src={art} alt={item.title} loading="lazy" />
              {:else}
                <Icon name="music" size={18} />
              {/if}
            </div>
            <div class="hist-meta">
              <div class="hist-title">{item.title}</div>
              <div class="hist-sub">
                {item.artist} • {item.source === "mic" ? "Mic" : "System"} • {formatAgo(item.recognizedAtMs)}
              </div>
            </div>
            <div class="hist-acts">
              {#if item.matchedTrack}
                {@const m = item.matchedTrack}
                {@const isLiked = likedIds.has(m.id)}
                <button
                  class="act-btn"
                  title="Play"
                  aria-label="Play"
                  onclick={() => onPlay(m, [m])}
                >
                  <Icon name="play" size={14} />
                </button>
                <HeartButton liked={isLiked} size={15} buttonClass="btn-icon sm" keep onToggle={() => onToggleLike(m)} />
                <button
                  class="act-btn"
                  title="Add to Playlist"
                  aria-label="Add to Playlist"
                  onclick={() => onAddToPlaylist(m)}
                >
                  <Icon name="list-plus" size={14} />
                </button>
              {/if}
              <button
                class="act-btn"
                title="Search"
                aria-label="Search"
                onclick={() => onSearchQuery(`${item.title} ${item.artist}`)}
              >
                <Icon name="search" size={14} />
              </button>
              <button
                class="act-btn danger"
                title="Delete"
                aria-label="Delete"
                onclick={() => void removeItem(item.id)}
              >
                <Icon name="trash" size={14} />
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .page {
    padding: 24px 30px 44px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  .top-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }

  .top-head h1 {
    margin: 0;
    font-size: 24px;
  }

  .source-switch {
    display: flex;
    gap: 8px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    color: var(--p-text-muted);
    transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease, transform 0.12s ease;
  }

  .chip:hover:not(:disabled) {
    background: var(--p-fill);
    color: var(--p-text);
  }

  .chip.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    border-color: var(--p-accent);
  }

  .chip:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .stage {
    padding: 34px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    background: radial-gradient(circle at 50% 45%, var(--p-surface-tint), var(--p-surface));
  }

  .pulse-wrap {
    position: relative;
    width: 140px;
    height: 140px;
    display: grid;
    place-items: center;
  }

  .ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 1.5px solid var(--p-accent);
    opacity: 0.38;
    pointer-events: none;
    transition: transform 0.09s ease-out;
  }

  .ring.r2 {
    opacity: 0.22;
  }

  .ring.r3 {
    animation: pulseWave 2s ease-out infinite;
  }

  @keyframes pulseWave {
    0% {
      transform: scale(0.85);
      opacity: 0.55;
    }
    100% {
      transform: scale(1.65);
      opacity: 0;
    }
  }

  .listen-btn {
    position: relative;
    z-index: 2;
    width: 102px;
    height: 102px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--p-accent-bright), var(--p-accent));
    color: var(--p-on-accent);
    display: grid;
    place-items: center;
    box-shadow: 0 10px 32px var(--p-glow-1);
    transition: transform 0.18s cubic-bezier(0.22, 1, 0.36, 1), filter 0.15s ease;
  }

  .listen-btn:hover {
    transform: scale(1.06);
    filter: brightness(1.06);
  }

  .listen-btn:active {
    transform: scale(0.95);
  }

  .listen-btn.listening {
    box-shadow: 0 0 40px var(--p-glow-1);
  }

  .stage-label {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    text-align: center;
  }

  .stage-label strong {
    font-size: 16px;
  }

  .stage-label span {
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .err-box {
    padding: 12px 16px;
    color: var(--p-danger);
    font-size: 13px;
    border-color: rgba(239, 68, 68, 0.35);
  }

  .result-card {
    padding: 18px;
    display: flex;
    gap: 18px;
    align-items: flex-start;
  }

  .result-art {
    width: 120px;
    height: 120px;
    border-radius: 14px;
    overflow: hidden;
    background: var(--p-fill);
    display: grid;
    place-items: center;
    flex-shrink: 0;
    color: var(--p-text-dim);
  }

  .result-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .result-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .badge-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 6px;
  }

  .meta-pill {
    font-size: 11px;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--p-fill);
    color: var(--p-text-muted);
  }

  .result-meta h2 {
    margin: 0;
    font-size: 20px;
  }

  .result-meta p {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--p-text-muted);
  }

  .result-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .snippet {
    margin: 4px 0 0;
    padding: 12px;
    border-radius: 12px;
    background: rgba(0, 0, 0, 0.25);
    font-family: inherit;
    font-size: 13px;
    line-height: 1.55;
    white-space: pre-wrap;
    color: var(--p-text-muted);
    max-height: 180px;
    overflow-y: auto;
  }

  .history-sec {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .sec-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .sec-head h2 {
    margin: 0;
    font-size: 17px;
  }

  .empty {
    padding: 26px;
    text-align: center;
    color: var(--p-text-muted);
    font-size: 13px;
  }

  .hist-list {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .hist-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 12px;
  }

  .hist-row:hover {
    background: var(--p-fill);
  }

  .hist-thumb {
    width: 44px;
    height: 44px;
    border-radius: 9px;
    overflow: hidden;
    background: var(--p-fill);
    display: grid;
    place-items: center;
    flex-shrink: 0;
    color: var(--p-text-dim);
  }

  .hist-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .hist-meta {
    flex: 1;
    min-width: 0;
  }

  .hist-title {
    font-size: 13.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hist-sub {
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .hist-acts {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .act-btn {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    color: var(--p-text-muted);
    background: var(--p-fill);
    transition: background 0.14s ease, color 0.14s ease, transform 0.12s ease;
  }

  .act-btn:hover {
    background: rgba(255, 255, 255, 0.13);
    color: var(--p-text);
  }

  .act-btn:active {
    transform: scale(0.92);
  }


  .act-btn.danger:hover {
    color: var(--p-danger);
  }
</style>
