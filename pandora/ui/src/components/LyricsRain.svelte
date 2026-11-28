<script lang="ts">
  import { onDestroy } from "svelte";
  import { joinsWithoutSpaces, quickenWords, timedWords, type TimedWord } from "../lib/lyricWords";
  import type { PandoraLyricLine } from "../lib/types";

  /**
   * "Rain" lyric style: one line at a time, centred. Each word drops into place letter by letter as it is sung; when
   * the song moves on, the finished line's letters fall away like rain (each with its own drift, spin and delay)
   * while the next line drops in. Transform/opacity only. With reduced motion, lines simply cross-fade.
   */
  interface Props {
    lines: PandoraLyricLine[];
    activeIdx: number;
    timeMs: number;
    showRomaji: boolean;
  }

  let { lines, activeIdx, timeMs, showRomaji }: Props = $props();

  /** How long a finished line keeps falling before it is removed (longest fall + largest delay). */
  const FALL_MS = 2_600;
  /** Words drop in after 72 % of their (mostly estimated) delay, at most 340 ms apart: ahead of the estimate so they
   *  keep up with the voice, but not so far ahead that they outrun it (55 % / 260 ms did). */
  const WORD_PACE = 0.72;
  const MAX_WORD_STEP_MS = 340;

  interface Glyph {
    ch: string;
    /** Stable pseudo-random values in -1..1 for this letter, so re-renders never reshuffle it. */
    a: number;
    b: number;
    c: number;
  }
  interface RainWord extends TimedWord {
    glyphs: Glyph[];
  }
  interface RainLine {
    idx: number;
    words: RainWord[];
    noGaps: boolean;
    /** Romaji / subtitle shown under the line, kept with it while it falls so its layout never changes. */
    reading: string | null;
  }

  const reduceMotion = typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function rand(seed: number): number {
    const x = Math.sin(seed * 12.9898 + 78.233) * 43758.5453;
    return (x - Math.floor(x)) * 2 - 1;
  }

  function build(idx: number): RainLine | null {
    const line = lines[idx];
    if (!line) return null;
    let n = 0;
    const timed = timedWords(line, lines[idx + 1]?.timeMs ?? null);
    const words = quickenWords(timed, line.timeMs, WORD_PACE, MAX_WORD_STEP_MS).map((w) => ({
      ...w,
      glyphs: Array.from(w.text).map((ch) => {
        n += 1;
        const seed = idx * 997 + n * 31;
        return { ch, a: rand(seed), b: rand(seed + 1), c: rand(seed + 2) };
      }),
    }));
    const reading = showRomaji ? (line.romaji ?? line.subtitle ?? null) : null;
    return { idx, words, noGaps: joinsWithoutSpaces(line.text), reading };
  }

  let current = $derived(activeIdx >= 0 ? build(activeIdx) : null);
  let upcoming = $derived(lines[activeIdx + 1]?.text ?? "");

  /** Lines that already ended, still raining down. */
  let falling = $state<RainLine[]>([]);
  let lastIdx = -1;
  const timers = new Set<number>();

  $effect(() => {
    const idx = activeIdx;
    if (idx === lastIdx) return;
    const previous = lastIdx >= 0 ? build(lastIdx) : null;
    lastIdx = idx;
    if (!previous || reduceMotion) return;
    falling = [...falling.filter((l) => l.idx !== previous.idx), previous];
    const timer = window.setTimeout(() => {
      falling = falling.filter((l) => l !== previous);
      timers.delete(timer);
    }, FALL_MS);
    timers.add(timer);
  });

  onDestroy(() => timers.forEach((t) => window.clearTimeout(t)));
</script>

<div class="sky" aria-live="off">
  <!-- A finished line falls from exactly the layout it was shown in (same stack, romaji included), so nothing
       jumps when it changes layer: its letters rain down and its romaji fades out where it was. -->
  {#each falling as line (line.idx)}
    <div class="stack falling" aria-hidden="true">
    <p class="line">
      {#each line.words as word, wi (wi)}<span class="word"
          >{#each word.glyphs as g, gi (gi)}<span
              class="glyph"
              style:--dx={`${g.a * 9}vw`}
              style:--rot={`${g.b * 70}deg`}
              style:--dur={`${1.5 + (g.c + 1) * 0.35}s`}
              style:--delay={`${(g.a + 1) * 0.18}s`}>{g.ch}</span
            >{/each}</span
        >{line.noGaps ? "" : " "}{/each}
    </p>
    {#if line.reading}<p class="reading fading">{line.reading}</p>{/if}
    </div>
  {/each}

  {#if current}
    {#key current.idx}
      <div class="stack">
      <p class="line current" class:calm={reduceMotion}>
        {#each current.words as word, wi (wi)}<span class="word" class:sung={timeMs >= word.startMs}
            >{#each word.glyphs as g, gi (gi)}<span
                class="glyph"
                style:--rot={`${g.b * 24}deg`}
                style:--lift={`${0.6 + (g.c + 1) * 0.5}em`}
                style:--stagger={`${gi * 22}ms`}>{g.ch}</span
              >{/each}</span
          >{current.noGaps ? "" : " "}{/each}
      </p>
      {#if current.reading}<p class="reading">{current.reading}</p>{/if}
      </div>
    {/key}
  {/if}

  {#if upcoming}
    <p class="next" aria-hidden="true">{upcoming}</p>
  {/if}
</div>

<style>
  .sky {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: grid;
    place-items: center;
    padding: 0 24px;
  }

  .line {
    grid-area: 1 / 1;
    margin: 0;
    max-width: 100%;
    text-align: center;
    font-size: clamp(34px, 4vw, 64px);
    font-weight: 800;
    line-height: 1.16;
    letter-spacing: -0.02em;
    overflow-wrap: anywhere;
    color: var(--p-text);
  }

  /* The current line and its romaji sit together in the middle of the sky. */
  .stack {
    grid-area: 1 / 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    max-width: 100%;
  }

  .stack .line {
    grid-area: auto;
  }

  .reading {
    margin: 0;
    text-align: center;
    font-size: clamp(15px, 1.5vw, 22px);
    font-weight: 600;
    color: var(--p-text-muted);
    animation: reading-in 0.6s ease 0.15s both;
  }

  .reading.fading {
    animation: reading-out 0.5s ease both;
  }

  @keyframes reading-out {
    to {
      opacity: 0;
      translate: 0 10px;
    }
  }

  @keyframes reading-in {
    from {
      opacity: 0;
      translate: 0 6px;
    }
  }

  .word {
    display: inline-block;
    white-space: pre;
  }

  .glyph {
    display: inline-block;
  }

  /* Not sung yet: invisible. Once sung, each letter drops in from above, tipped, with a small overshoot; the letters
     of a word land a few milliseconds apart. An animation (not a transition) so a word already sung when its line
     appears drops in too. */
  .current .word:not(.sung) .glyph {
    opacity: 0;
  }

  .current .word.sung .glyph {
    animation: land 0.38s cubic-bezier(0.34, 1.56, 0.64, 1) var(--stagger) backwards;
  }

  .current.calm .word:not(.sung) .glyph {
    opacity: 0.35;
  }

  @keyframes land {
    from {
      opacity: 0;
      translate: 0 calc(-1 * var(--lift));
      rotate: var(--rot);
    }
  }

  /* Finished line: every letter falls away with its own drift, spin, speed and delay. */
  .falling .glyph {
    animation: fall var(--dur) cubic-bezier(0.45, 0, 0.9, 0.6) var(--delay) both;
  }

  @keyframes fall {
    to {
      translate: var(--dx) 70vh;
      rotate: var(--rot);
      opacity: 0;
    }
  }

  .next {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 8%;
    margin: 0;
    padding: 0 24px;
    text-align: center;
    font-size: 15px;
    font-weight: 650;
    color: var(--p-text);
    opacity: 0.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
