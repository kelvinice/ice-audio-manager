<script lang="ts">
  import { audioEngine, playerStore, SHUFFLE_TITLES } from "../lib/audio";
  import { SpectrumShaper } from "../lib/spectrum";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { BackdropMotion, LyricsStyle, Track } from "../lib/types";
  import HeartButton from "./HeartButton.svelte";
  import Icon from "./Icon.svelte";
  import LyricsFisheye from "./LyricsFisheye.svelte";
  import LyricsRain from "./LyricsRain.svelte";
  import MoreMenu from "./MoreMenu.svelte";
  import NowPlayingBackdrop from "./NowPlayingBackdrop.svelte";
  import SeekBar from "./SeekBar.svelte";

  interface Props {
    open: boolean;
    proxyPort: number;
    likedIds: Set<string>;
    showRomaji: boolean;
    animatedCanvas: boolean;
    backdropMotion: BackdropMotion;
    wavyProgress?: boolean;
    spinningCover?: boolean;
    onClose: () => void;
    onToggleLike: (track: Track) => void;
    onAddToPlaylist: (track: Track) => void;
    onToggleRomaji: () => void;
    lyricsStyle: LyricsStyle;
    onSetLyricsStyle: (style: LyricsStyle) => void;
    onToggleCanvas: () => void;
  }

  let {
    open,
    proxyPort,
    likedIds,
    showRomaji,
    animatedCanvas,
    backdropMotion,
    wavyProgress = true,
    spinningCover = false,
    onClose,
    onToggleLike,
    onAddToPlaylist,
    onToggleRomaji,
    lyricsStyle,
    onSetLyricsStyle,
    onToggleCanvas,
  }: Props = $props();

  let rightTab = $state<"lyrics" | "queue">("lyrics");

  const LYRIC_STYLES: { id: LyricsStyle; label: string; hint: string }[] = [
    { id: "default", label: "Default", hint: "Scrolling list" },
    { id: "fisheye", label: "Fisheye", hint: "A lens on the current line" },
    { id: "rain", label: "Rain", hint: "Letters drop in and fall away" },
  ];
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

  /**
   * Lyrics follow the song (current line kept mid-pane) until the listener scrolls them with the wheel or touch; then
   * they stay where they were put, however the song moves on, until the Sync button is pressed. A different song or
   * reopening Now Playing starts in follow mode again.
   */
  let following = $state(true);
  // Only the id: reading the whole store here re-ran this on every time update and switched following back on.
  let trackId = $derived($playerStore.currentTrack?.id ?? null);

  $effect(() => {
    void trackId;
    void open;
    following = true;
  });

  function centerCurrentLine(behavior: ScrollBehavior) {
    const box = lyricsContainer;
    if (!box || activeLineIdx < 0) return;
    const el = box.querySelector(`[data-line="${activeLineIdx}"]`) as HTMLElement | null;
    if (!el) return;
    const top = el.offsetTop - box.clientHeight / 2 + el.offsetHeight / 2;
    box.scrollTo({ top: Math.max(0, top), behavior: reduceMotion ? "auto" : behavior });
  }

  function syncLyrics() {
    following = true;
    centerCurrentLine("smooth");
  }

  // Scrolls only the lyrics pane (`scrollIntoView` also moved the page), and never sideways.
  $effect(() => {
    void activeLineIdx;
    if (!open || rightTab !== "lyrics" || !following || !lyricsContainer) return;
    centerCurrentLine("smooth");
  });

  // Spectrum, shaped like Ice's notch pill (`lib/spectrum.ts`): thin round bars grown from the middle, each band on
  // its own automatic range and spring, a low textured resting line when quiet, all in the accent colour
  // (the seek bar below shows progress). At most 30 Hz, only while playing
  // and shown; after a pause it runs just until the bars have settled onto the resting line.
  const SPECTRUM_FRAME_MS = 1000 / 30;
  const shaper = new SpectrumShaper();
  let bandDb = new Float32Array(48);

  $effect(() => {
    const canvas = canvasEl;
    if (!open || !canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const live = playing && !reduceMotion;
    let frame = 0;
    let lastPaint = 0;
    let accent = "";

    function readColors() {
      accent = getComputedStyle(canvas!).color;
    }

    function paint(dt: number): boolean {
      if (!canvas || !ctx) return false;
      const dpr = window.devicePixelRatio || 1;
      const width = Math.round(canvas.clientWidth * dpr);
      const height = Math.round(canvas.clientHeight * dpr);
      if (canvas.width !== width || canvas.height !== height || !accent) {
        canvas.width = width;
        canvas.height = height;
        readColors();
      }
      const barW = Math.max(2, Math.round(3 * dpr));
      const gap = Math.max(3, Math.round(4 * dpr));
      const count = Math.max(14, Math.min(48, Math.floor((width + gap) / (barW + gap))));
      if (bandDb.length < count) bandDb = new Float32Array(count);
      const hasAudio = live && audioEngine.getSpectrumDb(count, bandDb);
      const { levels, moving } = shaper.step(hasAudio ? bandDb : null, count, dt);
      const x0 = Math.round((width - (count * barW + (count - 1) * gap)) / 2);
      const mid = height / 2;
      ctx.clearRect(0, 0, width, height);
      ctx.fillStyle = accent;
      for (let i = 0; i < count; i++) {
        const level = levels[i];
        const h = Math.max(barW, height * (0.08 + 0.92 * level));
        ctx.globalAlpha = 0.55 + 0.45 * level;
        ctx.beginPath();
        ctx.roundRect(x0 + i * (barW + gap), mid - h / 2, barW, h, barW / 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;
      return moving;
    }

    function tick(now: number) {
      frame = 0;
      if (now - lastPaint >= SPECTRUM_FRAME_MS) {
        const dt = lastPaint === 0 ? SPECTRUM_FRAME_MS / 1000 : Math.min(0.1, (now - lastPaint) / 1000);
        lastPaint = now;
        if (!paint(dt) && !live) return;
      }
      frame = requestAnimationFrame(tick);
    }

    frame = requestAnimationFrame(tick);
    const resize = new ResizeObserver(() => {
      accent = "";
      if (!frame) paint(0.1);
    });
    resize.observe(canvas);
    return () => {
      cancelAnimationFrame(frame);
      resize.disconnect();
    };
  });

</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && onClose()} />

{#if open && $playerStore.currentTrack}
  {@const track = $playerStore.currentTrack}
  {@const isLiked = likedIds.has(track.id)}
  <div class="overlay">
    <NowPlayingBackdrop
      artworkUrl={resolveArtworkUrl(track.artworkUrl, proxyPort)}
      animated={animatedCanvas}
      motion={backdropMotion}
      playing={$playerStore.playing}
    />

    <div class="top-bar">
      <span class="top-label">Now playing</span>
      <div class="top-actions">
        <MoreMenu
          label={`Lyric style: ${LYRIC_STYLES.find((s) => s.id === lyricsStyle)?.label ?? "Default"}`}
          heading="Lyric style"
          icon="lyrics"
          align="right"
          active={lyricsStyle !== "default"}
          items={LYRIC_STYLES.map((style) => ({
            label: style.label,
            hint: style.hint,
            checked: lyricsStyle === style.id,
            onSelect: () => onSetLyricsStyle(style.id),
          }))}
        />
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
            <HeartButton liked={isLiked} size={22} buttonClass="btn-icon" keep onToggle={() => onToggleLike(track)} />
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
            class:on={$playerStore.shuffle !== "off"}
            title={SHUFFLE_TITLES[$playerStore.shuffle]}
            aria-label={SHUFFLE_TITLES[$playerStore.shuffle]}
            onclick={() => audioEngine.cycleShuffle()}
          >
            <Icon name={$playerStore.shuffle === "magic" ? "shuffle-magic" : "shuffle"} size={17} />
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

        {#if rightTab === "lyrics" && lyricsStyle === "fisheye" && $playerStore.lyrics?.status === "synced"}
          <LyricsFisheye
            lines={$playerStore.lyrics.lines}
            activeIdx={activeLineIdx}
            timeMs={$playerStore.currentTimeMs}
            {showRomaji}
            onSeek={(ms) => audioEngine.seekMs(ms)}
          />
        {:else if rightTab === "lyrics" && lyricsStyle === "rain" && $playerStore.lyrics?.status === "synced"}
          <LyricsRain
            lines={$playerStore.lyrics.lines}
            activeIdx={activeLineIdx}
            timeMs={$playerStore.currentTimeMs}
            {showRomaji}
          />
        {:else if rightTab === "lyrics"}
          <div
            class="lyrics-scroll"
            role="region"
            aria-label="Lyrics"
            bind:this={lyricsContainer}
            onwheel={() => (following = false)}
            ontouchmove={() => (following = false)}
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
          {#if !following && $playerStore.lyrics?.status === "synced" && activeLineIdx >= 0}
            <button class="sync" onclick={syncLyrics}>
              <Icon name="refresh" size={15} />
              <span>Sync</span>
            </button>
          {/if}
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
    isolation: isolate;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    padding: 20px 32px 28px;
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

  /* Cover, titles, spectrum, seek bar and controls share the cover's width, so their edges line up. */
  .left-col {
    --col: clamp(200px, calc(100dvh - 380px), 360px);
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 14px;
  }

  .left-col > :global(*) {
    width: min(100%, var(--col));
  }

  .cover-wrap {
    position: relative;
    width: min(100%, var(--col));
    aspect-ratio: 1;
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


  .spectrum {
    display: block;
    width: min(100%, var(--col));
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
    position: relative;
  }

  /* Bottom-middle pill shown once the listener scrolls away from the current line. */
  .sync {
    position: absolute;
    left: 50%;
    bottom: 24px;
    translate: -50% 0;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 18px;
    border-radius: 999px;
    background: var(--p-text);
    color: var(--p-bg);
    font-size: 13px;
    font-weight: 700;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.35);
    animation: sync-in 0.18s ease both;
    transition: scale 0.12s ease;
  }

  .sync:hover {
    scale: 1.04;
  }

  .sync:active {
    scale: 0.97;
  }

  @keyframes sync-in {
    from {
      opacity: 0;
      translate: -50% 8px;
    }
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
    align-items: center;
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
    color: var(--p-text);
    opacity: 0.4;
    transition: color 0.3s ease, opacity 0.3s ease;
  }

  .lyr-line:hover {
    opacity: 0.9;
  }

  .lyr-line.past {
    opacity: 0.28;
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
