<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { audioEngine, playerStore } from "../lib/audio";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { Track } from "../lib/types";
  import Icon from "./Icon.svelte";

  interface Props {
    open: boolean;
    proxyPort: number;
    likedIds: Set<string>;
    showRomaji: boolean;
    animatedCanvas: boolean;
    onClose: () => void;
    onToggleLike: (track: Track) => void;
    onAddToPlaylist: (track: Track) => void;
    onToggleRomaji: () => void;
    onToggleCanvas: () => void;
  }

  let {
    open,
    proxyPort,
    likedIds,
    showRomaji,
    animatedCanvas,
    onClose,
    onToggleLike,
    onAddToPlaylist,
    onToggleRomaji,
    onToggleCanvas,
  }: Props = $props();

  let rightTab = $state<"lyrics" | "queue">("lyrics");
  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let lyricsContainer = $state<HTMLDivElement | null>(null);
  let rafId = 0;

  let activeLineIdx = $derived.by(() => {
    const lyr = $playerStore.lyrics;
    if (!lyr || lyr.lines.length === 0) return -1;
    const t = $playerStore.currentTimeMs;
    let idx = -1;
    for (let i = 0; i < lyr.lines.length; i++) {
      if (lyr.lines[i].timeMs <= t) idx = i;
      else break;
    }
    return idx;
  });

  $effect(() => {
    if (!open || rightTab !== "lyrics" || activeLineIdx < 0 || !lyricsContainer) return;
    const el = lyricsContainer.querySelector(`[data-line="${activeLineIdx}"]`) as HTMLElement | null;
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  });

  onMount(() => {
    const draw = () => {
      if (open && canvasEl) {
        const ctx = canvasEl.getContext("2d");
        if (ctx) {
          const bars = audioEngine.getSpectrumBars(28);
          const w = canvasEl.width;
          const h = canvasEl.height;
          ctx.clearRect(0, 0, w, h);
          const accent = getComputedStyle(document.documentElement).getPropertyValue("--p-accent").trim() || "#6d83f2";
          const gap = 4;
          const barW = (w - gap * (bars.length - 1)) / bars.length;
          for (let i = 0; i < bars.length; i++) {
            const val = Math.max(0.06, bars[i]);
            const barH = val * (h - 6);
            ctx.fillStyle = accent;
            ctx.globalAlpha = 0.35 + val * 0.65;
            const x = i * (barW + gap);
            const y = h - barH;
            ctx.beginPath();
            ctx.roundRect(x, y, barW, barH, 3);
            ctx.fill();
          }
          ctx.globalAlpha = 1;
        }
      }
      rafId = requestAnimationFrame(draw);
    };
    rafId = requestAnimationFrame(draw);
  });

  onDestroy(() => {
    if (rafId) cancelAnimationFrame(rafId);
  });

  function formatMs(ms: number): string {
    const total = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
  }

  function handleSeek(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    audioEngine.seekMs(val);
  }
</script>

{#if open && $playerStore.currentTrack}
  {@const track = $playerStore.currentTrack}
  {@const isLiked = likedIds.has(track.id)}
  <div class="overlay">
    {#if animatedCanvas}
      <div class="canvas-backdrop">
        <div class="orb o1"></div>
        <div class="orb o2"></div>
      </div>
    {/if}

    <div class="top-bar">
      <div class="top-left">
        <span class="pill-badge">Now Playing</span>
        <button class="btn-ghost" onclick={onToggleCanvas}>
          <Icon name="sparkles" size={14} />
          <span>Canvas: {animatedCanvas ? "On" : "Off"}</span>
        </button>
      </div>
      <button class="btn-ghost" onclick={onClose} aria-label="Close Now Playing">
        <Icon name="x" size={15} />
        <span>Close</span>
      </button>
    </div>

    <div class="body-grid">
      <div class="left-col">
        <div class="cover-wrap">
          {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
            <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
          {:else}
            <div class="cover-empty"><Icon name="music" size={48} /></div>
          {/if}
        </div>

        <div class="track-head">
          <div class="track-titles">
            <h2>{track.title}</h2>
            <p>{track.artist}{track.album ? ` • ${track.album}` : ""}</p>
          </div>
          <div class="track-btns">
            <button
              class="like-btn"
              class:liked={isLiked}
              title={isLiked ? "Remove from Favorites" : "Add to Favorites"}
              aria-label={isLiked ? "Remove from Favorites" : "Add to Favorites"}
              onclick={() => onToggleLike(track)}
            >
              <Icon name={isLiked ? "heart-filled" : "heart"} size={18} />
            </button>
            <button
              class="like-btn"
              title="Add to Playlist"
              aria-label="Add to Playlist"
              onclick={() => onAddToPlaylist(track)}
            >
              <Icon name="list-plus" size={18} />
            </button>
          </div>
        </div>

        <canvas bind:this={canvasEl} width="420" height="54" class="spectrum"></canvas>

        <div class="scrubber">
          <input
            type="range"
            min="0"
            max={Math.max(1000, $playerStore.durationMs)}
            value={$playerStore.currentTimeMs}
            oninput={handleSeek}
          />
          <div class="times">
            <span>{formatMs($playerStore.currentTimeMs)}</span>
            <span>{formatMs($playerStore.durationMs)}</span>
          </div>
        </div>

        <div class="controls">
          <button
            class="ctrl-sm"
            class:on={$playerStore.shuffle}
            title="Shuffle"
            aria-label="Toggle shuffle"
            onclick={() => audioEngine.toggleShuffle()}
          >
            <Icon name="shuffle" size={17} />
          </button>
          <button
            class="ctrl-md"
            title="Previous track"
            aria-label="Previous track"
            onclick={() => void audioEngine.previous()}
          >
            <Icon name="skip-back" size={19} />
          </button>
          <button
            class="ctrl-lg"
            title={$playerStore.playing ? "Pause" : "Play"}
            aria-label={$playerStore.playing ? "Pause" : "Play"}
            onclick={() => void audioEngine.togglePlayPause()}
          >
            <Icon name={$playerStore.playing ? "pause" : "play"} size={22} />
          </button>
          <button
            class="ctrl-md"
            title="Next track"
            aria-label="Next track"
            onclick={() => void audioEngine.next()}
          >
            <Icon name="skip-forward" size={19} />
          </button>
          <button
            class="ctrl-sm"
            class:on={$playerStore.repeat !== "off"}
            title="Repeat mode"
            aria-label="Cycle repeat mode"
            onclick={() => audioEngine.cycleRepeat()}
          >
            <Icon name={$playerStore.repeat === "one" ? "repeat-1" : "repeat"} size={17} />
          </button>
        </div>
      </div>

      <div class="right-col card-glass">
        <div class="right-head">
          <div class="right-tabs">
            <button class:active={rightTab === "lyrics"} onclick={() => (rightTab = "lyrics")}>
              Synced Lyrics
            </button>
            <button class:active={rightTab === "queue"} onclick={() => (rightTab = "queue")}>
              Up Next ({$playerStore.queue.length})
            </button>
          </div>
          {#if rightTab === "lyrics"}
            <button class="btn-ghost" onclick={onToggleRomaji}>
              Romaji: {showRomaji ? "On" : "Off"}
            </button>
          {/if}
        </div>

        {#if rightTab === "lyrics"}
          <div class="lyrics-scroll" bind:this={lyricsContainer}>
            {#if !$playerStore.lyrics || $playerStore.lyrics.status === "loading"}
              <div class="lyr-state">Searching synced lyrics (LRCLIB / local LRC)...</div>
            {:else if $playerStore.lyrics.status === "synced"}
              {#each $playerStore.lyrics.lines as line, idx (idx)}
                {@const isActive = idx === activeLineIdx}
                <button
                  data-line={idx}
                  class="lyr-line"
                  class:active={isActive}
                  class:past={idx < activeLineIdx}
                  onclick={() => audioEngine.seekMs(line.timeMs)}
                >
                  <div class="lyr-words">
                    {#if line.words.length > 0 && isActive}
                      {#each line.words as w, wi (wi)}
                        {@const wLit = $playerStore.currentTimeMs >= w.startMs}
                        <span class="word" class:lit={wLit}>{w.text}{" "}</span>
                      {/each}
                    {:else}
                      {line.text}
                    {/if}
                  </div>
                  {#if showRomaji && (line.romaji || line.subtitle)}
                    <div class="lyr-sub">{line.romaji ?? line.subtitle}</div>
                  {/if}
                </button>
              {/each}
            {:else if $playerStore.lyrics.plainText}
              <pre class="plain-lyr">{$playerStore.lyrics.plainText}</pre>
            {:else if $playerStore.lyrics.status === "instrumental"}
              <div class="lyr-state">♪ Instrumental track ♪</div>
            {:else}
              <div class="lyr-state">No synced lyrics found for this track.</div>
            {/if}
          </div>
        {:else}
          <div class="queue-scroll">
            {#each $playerStore.queue as qTrack, i (qTrack.id + i)}
              <div class="q-row" class:current={i === $playerStore.currentIndex}>
                <button
                  class="q-main"
                  onclick={() => void audioEngine.playTrack(qTrack, $playerStore.queue)}
                >
                  <span class="q-idx">
                    {#if i === $playerStore.currentIndex}
                      <Icon name="play" size={12} />
                    {:else}
                      {i + 1}
                    {/if}
                  </span>
                  <div class="q-meta">
                    <div class="q-title">{qTrack.title}</div>
                    <div class="q-sub">{qTrack.artist}</div>
                  </div>
                </button>
                {#if i !== $playerStore.currentIndex}
                  <button
                    class="q-rm"
                    aria-label="Remove from queue"
                    onclick={() => audioEngine.removeFromQueue(i)}
                  >
                    <Icon name="x" size={14} />
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgba(9, 11, 18, 0.94);
    backdrop-filter: blur(42px);
    display: flex;
    flex-direction: column;
    padding: 20px 32px 28px;
  }

  .canvas-backdrop {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    z-index: -1;
  }

  .orb {
    position: absolute;
    border-radius: 50%;
    filter: blur(120px);
    opacity: 0.7;
  }

  .o1 {
    width: 560px;
    height: 560px;
    left: -80px;
    top: -100px;
    background: var(--p-glow-1);
    animation: floatOrb 18s ease-in-out infinite alternate;
  }

  .o2 {
    width: 520px;
    height: 520px;
    right: -60px;
    bottom: -90px;
    background: var(--p-glow-2);
    animation: floatOrb 22s ease-in-out infinite alternate-reverse;
  }

  @keyframes floatOrb {
    from { transform: translate(0, 0) scale(1); }
    to { transform: translate(70px, 50px) scale(1.18); }
  }

  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 18px;
  }

  .top-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .body-grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(320px, 440px) 1fr;
    gap: 32px;
    align-items: stretch;
  }

  .left-col {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 14px;
  }

  .cover-wrap {
    width: 100%;
    max-width: 340px;
    aspect-ratio: 1;
    margin: 0 auto;
    border-radius: 22px;
    overflow: hidden;
    box-shadow: 0 26px 64px rgba(0, 0, 0, 0.55), 0 0 40px var(--p-glow-1);
    background: rgba(255, 255, 255, 0.06);
  }

  .cover-wrap img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .cover-empty {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    font-size: 56px;
    color: var(--p-text-dim);
  }

  .track-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .track-titles h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 750;
  }

  .track-titles p {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--p-text-muted);
  }

  .track-btns {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .like-btn {
    font-size: 22px;
    color: var(--p-text-muted);
    padding: 6px;
  }

  .like-btn.liked {
    color: #fb7185;
  }

  .spectrum {
    width: 100%;
    height: 54px;
  }

  .scrubber input {
    width: 100%;
    accent-color: var(--p-accent);
  }

  .times {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--p-text-dim);
  }

  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 20px;
  }

  .ctrl-sm {
    font-size: 16px;
    color: var(--p-text-dim);
  }

  .ctrl-sm.on {
    color: var(--p-accent-bright);
  }

  .ctrl-md {
    font-size: 20px;
    padding: 8px;
  }

  .ctrl-lg {
    width: 54px;
    height: 54px;
    border-radius: 50%;
    background: var(--p-accent);
    color: #080b12;
    font-size: 18px;
    font-weight: 700;
  }

  .right-col {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 18px 22px;
  }

  .right-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }

  .right-tabs {
    display: flex;
    gap: 8px;
  }

  .right-tabs button {
    padding: 7px 14px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 600;
    color: var(--p-text-muted);
    background: rgba(255, 255, 255, 0.05);
  }

  .right-tabs button.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
  }

  .lyrics-scroll,
  .queue-scroll {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-right: 8px;
  }

  .lyr-state {
    margin: auto;
    color: var(--p-text-muted);
    font-size: 15px;
  }

  .lyr-line {
    text-align: left;
    padding: 8px 12px;
    border-radius: 12px;
    font-size: 22px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.36);
    transition: color 0.25s ease, transform 0.25s ease, background 0.25s ease;
  }

  .lyr-line:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .lyr-line.past {
    color: rgba(255, 255, 255, 0.52);
  }

  .lyr-line.active {
    color: #ffffff;
    background: var(--p-accent-soft);
    transform: scale(1.015);
  }

  .word {
    color: rgba(255, 255, 255, 0.45);
    transition: color 0.14s ease;
  }

  .word.lit {
    color: var(--p-accent-bright);
    text-shadow: 0 0 18px var(--p-glow-1);
  }

  .lyr-sub {
    margin-top: 4px;
    font-size: 13px;
    font-weight: 500;
    color: var(--p-text-muted);
  }

  .plain-lyr {
    margin: 0;
    white-space: pre-wrap;
    font-family: inherit;
    font-size: 15px;
    line-height: 1.7;
    color: var(--p-text-muted);
  }

  .q-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 10px;
  }

  .q-row.current {
    background: var(--p-accent-soft);
  }

  .q-main {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
  }

  .q-idx {
    width: 22px;
    text-align: center;
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .q-title {
    font-size: 14px;
    font-weight: 600;
  }

  .q-sub {
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .q-rm {
    padding: 4px 8px;
    color: var(--p-text-dim);
  }
</style>
