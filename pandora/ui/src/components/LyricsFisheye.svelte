<script lang="ts">
  import { joinsWithoutSpaces, timedWords } from "../lib/lyricWords";
  import type { PandoraLyricLine } from "../lib/types";

  /**
   * "Fisheye" lyric style: a lens over the current line. It is large and sharp; neighbours shrink, fade and soften
   * with distance, and only a few either side are drawn. Moving to the next line animates size and fade, so the lens
   * appears to glide down the song. Lines entering or leaving the window start or end at zero size, so nothing jumps.
   */
  interface Props {
    lines: PandoraLyricLine[];
    activeIdx: number;
    timeMs: number;
    showRomaji: boolean;
    onSeek: (ms: number) => void;
  }

  let { lines, activeIdx, timeMs, showRomaji, onSeek }: Props = $props();

  /** By distance from the current line: size factor, opacity and blur (px). Index 4 is the invisible edge. */
  const SIZE = [1, 0.6, 0.44, 0.34, 0];
  const OPACITY = [1, 0.55, 0.32, 0.18, 0];
  const BLUR = [0, 0, 0.8, 1.6, 2.4];
  const REACH = 4;

  let visible = $derived.by(() => {
    const center = Math.max(activeIdx, -1);
    const out: number[] = [];
    for (let i = Math.max(0, center - REACH); i <= Math.min(lines.length - 1, center + REACH); i++) out.push(i);
    return out;
  });

  let activeWords = $derived(
    activeIdx >= 0 && lines[activeIdx] ? timedWords(lines[activeIdx], lines[activeIdx + 1]?.timeMs ?? null) : [],
  );
  let noGaps = $derived(activeIdx >= 0 && lines[activeIdx] ? joinsWithoutSpaces(lines[activeIdx].text) : false);
</script>

<div class="lens" role="group" aria-label="Lyrics">
  {#each visible as idx (idx)}
    {@const d = Math.min(REACH, Math.abs(idx - activeIdx))}
    {@const line = lines[idx]}
    <button
      class="line"
      class:active={idx === activeIdx}
      tabindex={d < REACH ? 0 : -1}
      style:--k={SIZE[d]}
      style:opacity={OPACITY[d]}
      style:filter={BLUR[d] > 0 ? `blur(${BLUR[d]}px)` : "none"}
      onclick={() => onSeek(line.timeMs)}
    >
      {#if idx === activeIdx && activeWords.length > 0}
        {#each activeWords as w, wi (wi)}
          <span class="word" class:lit={timeMs >= w.startMs}>{w.text}</span>{noGaps ? "" : " "}
        {/each}
      {:else}
        {line.text}
      {/if}
      {#if idx === activeIdx && showRomaji && (line.romaji || line.subtitle)}
        <span class="sub">{line.romaji ?? line.subtitle}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .lens {
    --base: clamp(30px, 3.4vw, 54px);
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    padding: 0 24px 0 0;
  }

  .line {
    display: block;
    max-width: 100%;
    margin: calc(var(--k) * 0.14em) 0;
    padding: 0;
    text-align: left;
    font-size: calc(var(--base) * var(--k));
    font-weight: 800;
    line-height: 1.18;
    letter-spacing: -0.02em;
    overflow-wrap: anywhere;
    color: var(--p-text);
    transition:
      font-size 0.5s cubic-bezier(0.22, 1, 0.36, 1),
      margin 0.5s cubic-bezier(0.22, 1, 0.36, 1),
      opacity 0.5s ease,
      filter 0.5s ease;
  }

  .line:not(.active):hover {
    opacity: 0.85 !important;
    filter: none !important;
  }

  .word {
    color: color-mix(in srgb, var(--p-text) 42%, transparent);
    transition: color 0.16s ease;
  }

  .word.lit {
    color: var(--p-text);
  }

  .sub {
    display: block;
    margin-top: 0.25em;
    font-size: 0.42em;
    font-weight: 600;
    letter-spacing: 0;
    color: var(--p-text-muted);
  }
</style>
