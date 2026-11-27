<script lang="ts">
  import { audioEngine, playerStore } from "../lib/audio";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { Track } from "../lib/types";
  import Icon from "./Icon.svelte";
  import SeekBar from "./SeekBar.svelte";

  interface Props {
    open: boolean;
    proxyPort: number;
    likedIds: Set<string>;
    showRomaji: boolean;
    animatedCanvas: boolean;
    wavyProgress?: boolean;
    spinningCover?: boolean;
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
    wavyProgress = true,
    spinningCover = false,
    onClose,
    onToggleLike,
    onAddToPlaylist,
    onToggleRomaji,
    onToggleCanvas,
  }: Props = $props();

  let rightTab = $state<"lyrics" | "queue">("lyrics");
  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let lyricsContainer = $state<HTMLDivElement | null>(null);
  let playing = $derived($playerStore.playing);

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

  const reduceMotion =
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  /** After the listener scrolls the lyrics, leave them where they are for this long before following the song. */
  const MANUAL_SCROLL_HOLD_MS = 3_000;
  let manualScrollAt = 0;

  // Keep the current line in the middle of the lyrics pane. Scrolls only that pane (`scrollIntoView` also moved the
  // page), and never sideways.
  $effect(() => {
    const idx = activeLineIdx;
    const box = lyricsContainer;
    if (!open || rightTab !== "lyrics" || idx < 0 || !box) return;
    if (performance.now() - manualScrollAt < MANUAL_SCROLL_HOLD_MS) return;
    const el = box.querySelector(`[data-line="${idx}"]`) as HTMLElement | null;
    if (!el) return;
    const top = el.offsetTop - box.clientHeight / 2 + el.offsetHeight / 2;
    box.scrollTo({ top: Math.max(0, top), behavior: reduceMotion ? "auto" : "smooth" });
  });

  // Spectrum: thin round bars grown from the middle, as many as fit, sharp at any display scale. They rise at once
  // and fall back gently (meter ballistics). Redrawn at most 30 times a second, and only while playing and shown.
  const SPECTRUM_FRAME_MS = 1000 / 30;
  const SPECTRUM_FALL = 0.86;

  $effect(() => {
    const canvas = canvasEl;
    if (!open || !canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const animate = playing && !reduceMotion;
    let frame = 0;
    let lastPaint = 0;
    let levels: number[] = [];
    let color = getComputedStyle(canvas).color;

    function draw(now = performance.now()) {
      if (!canvas || !ctx) return;
      if (animate) frame = requestAnimationFrame(draw);
      if (now - lastPaint < SPECTRUM_FRAME_MS) return;
      lastPaint = now;
      const dpr = window.devicePixelRatio || 1;
      const width = Math.round(canvas.clientWidth * dpr);
      const height = Math.round(canvas.clientHeight * dpr);
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
        color = getComputedStyle(canvas).color;
      }
      const barW = Math.max(2, Math.round(3 * dpr));
      const gap = Math.max(2, Math.round(3 * dpr));
      const count = Math.max(8, Math.floor((width + gap) / (barW + gap)));
      const raw = audioEngine.getSpectrumBars(count);
      if (levels.length !== count) levels = new Array(count).fill(0);
      const x0 = Math.round((width - (count * barW + (count - 1) * gap)) / 2);
      const mid = height / 2;
      ctx.clearRect(0, 0, width, height);
      ctx.fillStyle = color;
      for (let i = 0; i < count; i++) {
        levels[i] = Math.max(raw[i] ?? 0, levels[i] * SPECTRUM_FALL);
        const h = Math.max(barW, levels[i] * height);
        ctx.globalAlpha = 0.4 + 0.6 * levels[i];
        ctx.beginPath();
        ctx.roundRect(x0 + i * (barW + gap), mid - h / 2, barW, h, barW / 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;
    }

    function redraw() {
      cancelAnimationFrame(frame);
      lastPaint = 0;
      draw();
    }
    redraw();
    const resize = new ResizeObserver(redraw);
    resize.observe(canvas);
    document.addEventListener("visibilitychange", redraw);
    return () => {
      cancelAnimationFrame(frame);
      resize.disconnect();
      document.removeEventListener("visibilitychange", redraw);
    };
  });

</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && onClose()} />

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
      <span class="top-label">Now playing</span>
      <div class="top-actions">
        <button
          class="btn-icon"
          class:on={animatedCanvas}
          aria-pressed={animatedCanvas}
          title={animatedCanvas ? "Animated background: on" : "Animated background: off"}
          aria-label="Animated background"
          onclick={onToggleCanvas}
        >
          <Icon name="sparkles" size={17} />
        </button>
        <button class="btn-icon solid" aria-label="Collapse Now Playing" title="Collapse (Esc)" onclick={onClose}>
          <Icon name="collapse" size={17} />
        </button>
      </div>
    </div>

    <div class="body-grid">
      <div class="left-col">
        <div
          class="cover-wrap"
          class:spinning={spinningCover}
          class:paused={!$playerStore.playing}
        >
          {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
            <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
          {:else}
            <div class="cover-empty"><Icon name="music" size={48} /></div>
          {/if}
          {#if spinningCover}
            <span class="vinyl-hole" aria-hidden="true"></span>
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

        <canvas bind:this={canvasEl} class="spectrum" aria-hidden="true"></canvas>

        <SeekBar
          positionMs={$playerStore.currentTimeMs}
          durationMs={$playerStore.durationMs}
          playing={$playerStore.playing}
          wavy={wavyProgress}
          size="large"
          times="below"
          onSeek={(ms) => audioEngine.seekMs(ms)}
        />

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

      <div class="right-col">
        <div class="right-head">
          <div class="right-tabs" role="tablist">
            <button role="tab" aria-selected={rightTab === "lyrics"} class:active={rightTab === "lyrics"} onclick={() => (rightTab = "lyrics")}>
              Lyrics
            </button>
            <button role="tab" aria-selected={rightTab === "queue"} class:active={rightTab === "queue"} onclick={() => (rightTab = "queue")}>
              Up next <span class="n">{$playerStore.queue.length}</span>
            </button>
          </div>
          {#if rightTab === "lyrics"}
            <button class="chip-toggle" class:on={showRomaji} aria-pressed={showRomaji} onclick={onToggleRomaji}>
              Romaji
            </button>
          {/if}
        </div>

        {#if rightTab === "lyrics"}
          <div
            class="lyrics-scroll"
            role="region"
            aria-label="Lyrics"
            bind:this={lyricsContainer}
            onwheel={() => (manualScrollAt = performance.now())}
            ontouchmove={() => (manualScrollAt = performance.now())}
          >
            {#if !$playerStore.lyrics || $playerStore.lyrics.status === "loading"}
              <div class="lyr-state">Loading lyrics…</div>
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
              <div class="lyr-state">Instrumental — no lyrics.</div>
            {:else}
              <div class="lyr-state">No lyrics available.</div>
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
    background: var(--p-surface-solid);
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

  .top-label {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--p-text-dim);
  }

  .top-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .body-grid {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    grid-template-columns: minmax(320px, 440px) 1fr;
    /* One row exactly as tall as the overlay, so the lyrics pane scrolls inside it instead of the page growing. */
    grid-template-rows: minmax(0, 1fr);
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
    position: relative;
    width: 100%;
    max-width: clamp(140px,  calc(100dvh - 368px), 340px);
    aspect-ratio: 1;
    margin: 0 auto;
    border-radius: 22px;
    overflow: hidden;
    box-shadow: 0 26px 64px rgba(0, 0, 0, 0.55), 0 0 40px var(--p-glow-1);
    background: var(--p-fill);
    transition: border-radius 0.35s ease;
  }

  .cover-wrap.spinning {
    border-radius: 50%;
    border: 3px solid rgba(255, 255, 255, 0.14);
    animation: vinylSpin 20s linear infinite;
  }

  .cover-wrap.spinning.paused {
    animation-play-state: paused;
  }

  .vinyl-hole {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 34px;
    height: 34px;
    margin: -17px 0 0 -17px;
    border-radius: 50%;
    background: #0b0d14;
    border: 4px solid rgba(255, 255, 255, 0.28);
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.65);
    pointer-events: none;
  }

  @keyframes vinylSpin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
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
    color: var(--p-danger);
  }

  .spectrum {
    display: block;
    width: 100%;
    height: clamp(28px, 7vh, 64px);
    color: var(--p-accent);
  }

  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 20px;
  }

  .ctrl-sm {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--p-text-dim);
    transition: background 0.15s ease, color 0.15s ease, transform 0.12s ease;
  }

  .ctrl-sm:hover {
    background: var(--p-fill);
    color: var(--p-text);
  }

  .ctrl-sm.on {
    color: var(--p-accent-bright);
  }

  .ctrl-md {
    width: 42px;
    height: 42px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    transition: background 0.15s ease, transform 0.12s ease;
  }

  .ctrl-md:hover {
    background: var(--p-fill);
  }

  .ctrl-lg {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--p-accent);
    color: var(--p-on-accent);
    transition: transform 0.14s ease, background 0.14s ease;
  }

  .ctrl-lg:hover {
    transform: scale(1.05);
    background: var(--p-accent-bright);
  }

  .ctrl-sm:active,
  .ctrl-md:active,
  .ctrl-lg:active {
    transform: scale(0.92);
  }

  .right-col {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    padding: 18px 8px 0 24px;
    border-radius: 16px;
    background: color-mix(in srgb, var(--p-bg) 55%, transparent);
  }

  .right-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-right: 16px;
    margin-bottom: 6px;
  }

  .right-tabs {
    display: flex;
    gap: 8px;
  }

  .right-tabs button,
  .chip-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 14px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 650;
    background: var(--p-fill);
    color: var(--p-text-muted);
    transition: background 0.14s ease, color 0.14s ease;
  }

  .right-tabs button:hover,
  .chip-toggle:hover {
    color: var(--p-text);
  }

  .right-tabs button.active {
    background: var(--p-text);
    color: var(--p-bg);
  }

  .chip-toggle.on {
    background: var(--p-accent);
    color: var(--p-on-accent);
  }

  .n {
    font-size: 11.5px;
    opacity: 0.65;
    font-variant-numeric: tabular-nums;
  }

  .queue-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 16px 24px 0;
  }

  /* Lyrics: one column that only scrolls up and down. Lines wrap (never overflow sideways); the current line is set
     apart by colour, not size, so nothing grows past the pane. The edges fade so lines drift in and out. */
  .lyrics-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: none;
    /* Room above and below so the first and last lines can sit mid-pane (vh: % padding follows the width). */
    padding: 40vh 16px 40vh 0;
    mask-image: linear-gradient(transparent, #000 14%, #000 82%, transparent);
  }

  .lyrics-scroll::-webkit-scrollbar {
    display: none;
  }

  .lyr-state {
    margin-top: -20%;
    color: var(--p-text-muted);
    font-size: 16px;
  }

  .lyr-line {
    display: block;
    width: 100%;
    margin: 0 0 6px;
    padding: 6px 10px 6px 0;
    text-align: left;
    font-size: clamp(22px, 2.3vw, 34px);
    font-weight: 800;
    line-height: 1.22;
    letter-spacing: -0.02em;
    overflow-wrap: anywhere;
    color: var(--p-text-dim);
    opacity: 0.55;
    transition: color 0.3s ease, opacity 0.3s ease;
  }

  .lyr-line:hover {
    opacity: 0.9;
  }

  .lyr-line.past {
    opacity: 0.4;
  }

  .lyr-line.active {
    color: var(--p-text);
    opacity: 1;
  }

  .word {
    color: color-mix(in srgb, var(--p-text) 45%, transparent);
    transition: color 0.14s ease;
  }

  .word.lit {
    color: var(--p-text);
  }

  .lyr-sub {
    margin-top: 4px;
    font-size: 0.5em;
    font-weight: 600;
    letter-spacing: 0;
    color: var(--p-text-muted);
  }

  .plain-lyr {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-family: inherit;
    font-size: 20px;
    font-weight: 700;
    line-height: 1.6;
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
