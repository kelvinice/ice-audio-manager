<script lang="ts">
  import type { LyricLine, LyricsState, NowPlaying } from '../../lib/ipc/engine';
  import {
    hasSubtitleLines,
    ladderY,
    LADDER,
    lineSubtitle,
    lyricClockMs,
    lyricRows,
    nookLadderY,
    resolveLyricFrame,
    romajiLitCount,
    type LyricRow,
  } from '../../lib/notch/lyrics';
  import { fitLine } from '../../lib/notch/fitLine';
  import { toMsNumber } from '../../lib/notch/scheduler';
  import { liveMediaPositionMs } from '../../lib/notch/scrubber';

  interface Props {
    lyrics: LyricsState;
    media: NowPlaying;
    mode: 'pill' | 'card' | 'nook';
    wordHighlight: boolean;
    showRomaji: boolean;
    instant: boolean;
    notchVisible: boolean;
    /** The playing output's delay (`DeviceSummary.lyricsDelayMs`): lines light up when they are heard. */
    delayMs?: number;
    onSeek?: (positionMs: number) => void;
  }

  let {
    lyrics,
    media,
    mode,
    wordHighlight,
    showRomaji,
    instant,
    notchVisible,
    delayMs = 0,
    onSeek = () => {},
  }: Props = $props();

  let nowEpochMs = $state(Date.now());
  let fallbackAnchorMs = $state(Date.now());
  let lastRawPos: number | null = null;
  let lastStatus: NowPlaying['status'] | null = null;

  const isPlaying = $derived(media.status === 'playing');
  const doc = $derived(lyrics.document);
  const isPlain = $derived(doc?.syncKind === 'plain' || (doc?.lines.length ?? 0) === 0);

  $effect(() => {
    const pos = toMsNumber(media.positionMs);
    const updated = toMsNumber(media.positionUpdatedMs);
    const status = media.status;
    if (pos !== lastRawPos || status !== lastStatus) {
      lastRawPos = pos;
      lastStatus = status;
      const now = Date.now();
      if (updated === null) fallbackAnchorMs = now;
      nowEpochMs = now;
    }
  });

  const elapsedMs = $derived(lyricClockMs(liveMediaPositionMs(media, nowEpochMs, fallbackAnchorMs), delayMs));
  const frame = $derived(resolveLyricFrame(doc, elapsedMs, wordHighlight));

  // Event-aligned wake: sleeps until the next word or line boundary, zero polling when paused/hidden.
  $effect(() => {
    if (!isPlaying || !notchVisible || isPlain || !doc) return;
    nowEpochMs = Date.now();
    let timerId: number | null = null;
    const schedule = () => {
      const currentMs = lyricClockMs(liveMediaPositionMs(media, Date.now(), fallbackAnchorMs), delayMs);
      const next = resolveLyricFrame(doc, currentMs, wordHighlight);
      if (next.nextWakeDelayMs === null) return;
      timerId = window.setTimeout(() => {
        nowEpochMs = Date.now();
        schedule();
      }, next.nextWakeDelayMs);
    };
    schedule();
    return () => {
      if (timerId !== null) window.clearTimeout(timerId);
    };
  });

  // Ladder layout (2026-11-24): each visible line is one element keyed by the line, placed in a slot. When the song
  // moves on, the next line's element glides up into the main row (growing and brightening), the old main line
  // glides up and out, and only the newly revealed line fades in. `transform`/`opacity` only; instant with reduced motion.
  const rows = $derived(lyricRows(frame, mode));
  const hasSub = $derived(hasSubtitleLines(doc, showRomaji));
  const ladderHeight = $derived(
    mode === 'pill'
      ? ladderY('pill', 1, hasSub) + LADDER.pill.row
      : mode === 'nook'
        ? LADDER.nook.inner
        : LADDER.card.inner,
  );
  const highlightActiveWords = $derived(
    wordHighlight && !instant && frame.activeLine !== null && frame.activeLine.words.length > 0,
  );
  const useWordSpaces = $derived(
    Boolean(frame.activeLine && !/[぀-ヿ一-鿿]/.test(frame.activeLine.text)),
  );

  /** Hidden rows sit above the view: the pill shows slots 0–1, the card -1 to 1. */
  function hiddenSlot(slot: number): boolean {
    return mode === 'pill' ? slot < 0 : slot < -1;
  }

  // Edge Nook: the tall column wraps the current line (balanced, ≤ 3 rows) instead of shrinking or sliding it, and the
  // other rows are placed around its measured height (`nookLadderY`). Only the current row is observed.
  const wrapMain = $derived(mode === 'nook');
  let nookMainH = $state<number | null>(null);

  function rowY(slot: number): number {
    if (mode === 'pill') return ladderY('pill', slot, hasSub);
    if (mode === 'nook' && nookMainH !== null) return nookLadderY(slot, nookMainH);
    return ladderY(mode, slot, hasSub);
  }

  /** Reports the current row's height while `enabled`; event-driven (fires only when the size changes). */
  function trackMainHeight(node: HTMLElement, enabled: boolean) {
    let observer: ResizeObserver | null = null;
    const start = () => {
      observer = new ResizeObserver(() => {
        nookMainH = node.offsetHeight;
      });
      observer.observe(node);
    };
    const stop = () => {
      observer?.disconnect();
      observer = null;
    };
    if (enabled) start();
    return {
      update(next: boolean) {
        if (next && !observer) start();
        else if (!next && observer) stop();
      },
      destroy: stop,
    };
  }

  function romajiWords(text: string): string[] {
    return text.split(/\s+/).filter(Boolean);
  }

  function seekTo(row: LyricRow): void {
    if (row.line && !hiddenSlot(row.slot)) onSeek(row.line.startMs);
  }
</script>

{#snippet lineText(row: LyricRow)}
  {#if row.role === 'current' && highlightActiveWords && row.line}
    {#each row.line.words as word, idx (idx)}
      <span
        class="word"
        class:active={idx === frame.activeWordIndex}
        class:sung={idx < frame.activeWordIndex}
        class:upcoming={idx > frame.activeWordIndex}
        class:spaced={useWordSpaces && idx + 1 < row.line.words.length}
      >{word.text}</span>
    {/each}
  {:else}
    {row.line?.text ?? '···'}
  {/if}
{/snippet}

{#snippet subtitle(line: LyricLine | null, isMain: boolean, isCurrent: boolean)}
  {@const sub = lineSubtitle(line, showRomaji)}
  {#if sub}
    <span class="subtitle-line" class:shown={isMain}>
      {#if isCurrent && sub.generatedRomaji && highlightActiveWords && line}
        <!-- Romaji lights up as the line is sung, in step with the karaoke highlight above. -->
        {@const words = romajiWords(sub.text)}
        {@const lit = romajiLitCount(words.length, frame.activeWordIndex, line.words.length)}
        {#each words as w, i (i)}
          <span class="romaji-word" class:lit={i < lit}>{w}</span>{i + 1 < words.length ? ' ' : ''}
        {/each}
      {:else}
        {sub.text}
      {/if}
    </span>
  {/if}
{/snippet}

<div class="lyrics-ext" class:pill={mode === 'pill'} class:card={mode === 'card'} class:nook={mode === 'nook'} class:instant>
  {#if mode === 'pill'}
    <div class="pill-inner">
      <span class="note-dot" class:playing={isPlaying} aria-hidden="true">♪</span>
      <div class="ladder" style:height={`${ladderHeight}px`} aria-live="off">
        {#each rows as row (row.key)}
          <div
            class="rung"
            class:main={row.slot === 0}
            class:upcoming={row.role === 'upcoming'}
            class:hidden={hiddenSlot(row.slot)}
            style:transform={`translateY(${rowY(row.slot)}px)`}
          >
            <div class="rung-enter">
              <span
                class="rung-text"
                class:fitted={row.slot === 0}
                use:fitLine={{ enabled: row.slot === 0, word: row.slot === 0 ? frame.activeWordIndex : -1, key: row.key, center: false }}
              ><span class="fit">{@render lineText(row)}</span></span>
              {@render subtitle(row.line, row.slot === 0, row.role === 'current')}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {:else if isPlain && doc?.plainText}
    <div class="plain-scroll" aria-label="Song lyrics">
      {doc.plainText}
    </div>
  {:else}
    <div class="ladder card-ladder" style:height={`${ladderHeight}px`} aria-label="Synced lyrics">
      {#each rows as row (row.key)}
        <button
          class="rung"
          class:main={row.slot === 0}
          class:gap={row.role === 'gap'}
          class:hidden={hiddenSlot(row.slot)}
          type="button"
          tabindex={hiddenSlot(row.slot) ? -1 : 0}
          disabled={!row.line}
          style:transform={`translateY(${rowY(row.slot)}px)`}
          use:trackMainHeight={wrapMain && row.slot === 0}
          onclick={() => seekTo(row)}
        >
          <span class="rung-enter">
            <span
              class="rung-text"
              class:fitted={row.slot === 0 && !wrapMain}
              class:wrapped={row.slot === 0 && wrapMain}
              use:fitLine={{
                enabled: row.slot === 0 && !wrapMain,
                word: row.slot === 0 ? frame.activeWordIndex : -1,
                key: row.key,
                center: true,
              }}
            ><span class="fit">{@render lineText(row)}</span></span>
            {@render subtitle(row.line, row.slot === 0, row.role === 'current')}
          </span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .lyrics-ext {
    width: 100%;
    border-top: 1px solid var(--ice-control-border);
    display: flex;
    flex-direction: column;
    justify-content: center;
    overflow: hidden;
  }

  .lyrics-ext.pill {
    padding: 5px 4px 4px;
  }

  .pill-inner {
    display: flex;
    align-items: flex-start;
    gap: 7px;
    min-width: 0;
  }

  /* Aligned with the main row, not centred between the two. */
  .note-dot {
    font-size: 11px;
    line-height: 16px;
    color: var(--ice-active-accent, var(--ice-accent));
    opacity: 0.72;
    flex-shrink: 0;
  }

  .note-dot.playing {
    opacity: 1;
  }

  .lyrics-ext.card {
    height: 82px;
    padding: 6px 8px 8px;
    mask-image: linear-gradient(180deg, transparent 0%, #000 14%, #000 86%, transparent 100%);
  }

  .lyrics-ext.nook {
    border-top: none;
    height: 100%;
    padding: 2px 4px;
    mask-image: linear-gradient(180deg, transparent 0%, #000 16%, #000 84%, transparent 100%);
  }

  .lyrics-ext.nook .card-ladder .rung-text {
    font-size: 14px;
    line-height: 22px;
  }

  .lyrics-ext.nook .card-ladder .rung.main .rung-text {
    font-weight: 700;
  }

  .lyrics-ext.nook .card-ladder .rung:not(.main) .rung-text {
    transform: scale(0.84);
  }

  /* Edge Nook current line: wraps into balanced rows (heights twin `LADDER.nook.mainMax`) instead of being cut. */
  .rung-text.wrapped {
    white-space: normal;
    overflow: visible;
    text-overflow: clip;
    text-wrap: balance;
  }

  /* ---- Ladder: rows in absolute slots (heights twin `LADDER` in lyrics.ts). ---- */
  .ladder {
    position: relative;
    flex: 1;
    min-width: 0;
    /* `clip` + a margin as wide as the main row's bleed (`.rung-text.fitted`): the active word's lift and glow aren't
       shaved off at the ladder's edges. */
    overflow: clip;
    overflow-clip-margin: 4px;
  }

  .rung {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    min-width: 0;
    transition:
      transform 460ms cubic-bezier(0.22, 1, 0.36, 1),
      opacity 320ms ease;
  }

  .rung.hidden {
    opacity: 0;
    pointer-events: none;
  }

  /* New lines fade up into their slot; lines already on screen glide instead (they keep their element). */
  .rung-enter {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    animation: lyric-enter 360ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }

  .rung-text {
    display: block;
    max-width: 100%;
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--ice-lyric-active);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transform-origin: left center;
    transition:
      transform 460ms cubic-bezier(0.22, 1, 0.36, 1),
      color 320ms ease;
  }

  /* Main row: no ellipsis (it can't apply to the inline-block karaoke words, which were cut mid-character); the line
     shrinks to fit or slides with the singing instead (`fitLine`).
     Bleed: the clip and fade-mask box reaches past the text on every side (negative margin + equal padding, so the
     text doesn't move), so the active word's lift and glow aren't shaved off at the first or last character.
     Twin: `FIT_BLEED_X_PX` in lib/notch/fitLine.ts. */
  .rung-text.fitted {
    overflow-x: clip;
    overflow-y: visible;
    text-overflow: clip;
    margin: -3px -4px;
    padding: 3px 4px;
    max-width: calc(100% + 8px);
  }

  .fit {
    display: inline;
  }

  .rung-text.fitted > .fit {
    display: inline-block;
    position: relative; /* offsetParent for the word measurements */
    white-space: nowrap;
    transform-origin: left center;
    transition: transform 280ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  /* `fade-l`, `fade-r` and `fitting` are toggled by the `fitLine` action, so they must be `:global`: Svelte can't see
     them in the markup and would drop the rules (which cut the first characters of long centred lines and hid the fades). */

  /* Soft edges where a long line is cut while sliding. */
  .rung-text:global(.fade-r) {
    mask-image: linear-gradient(90deg, #000 calc(100% - 16px), transparent);
  }

  .rung-text:global(.fade-l) {
    mask-image: linear-gradient(90deg, transparent, #000 16px);
  }

  .rung-text:global(.fade-l.fade-r) {
    mask-image: linear-gradient(90deg, transparent, #000 16px, #000 calc(100% - 16px), transparent);
  }

  /* The card's main row spans the full width so `fitLine` measures the row, not the line's own shrink-to-fit width. */
  .card-ladder .rung-text.fitted {
    align-self: stretch;
  }

  /* A fitted card line is laid out from the left (a centred line wider than its row overflows on both sides and loses
     its first characters); `fitLine` centres it with the transform. */
  .card-ladder .rung-text.fitted:global(.fitting) {
    text-align: left;
  }

  .lyrics-ext.pill .rung-text {
    font-size: 12px;
    line-height: 16px;
  }

  /* Rows outside the main slot are smaller and dim; moving into it grows and brightens them. */
  .rung:not(.main) .rung-text {
    transform: scale(0.875);
    color: var(--ice-lyric-muted);
    font-weight: 500;
  }

  .rung.upcoming .rung-text {
    color: var(--ice-lyric-muted);
    font-weight: 500;
  }

  /* Card: centred, larger main line, clickable rows. */
  .card-ladder .rung {
    appearance: none;
    border: none;
    background: transparent;
    padding: 0 4px;
    margin: 0;
    font-family: inherit;
    text-align: center;
    cursor: pointer;
  }

  .card-ladder .rung:disabled {
    cursor: default;
  }

  .card-ladder .rung-enter {
    align-items: center;
  }

  .card-ladder .rung-text {
    font-size: 13px;
    line-height: 18px;
    transform-origin: center center;
  }

  .card-ladder .rung:not(.main) .rung-text {
    transform: scale(0.81);
  }

  .card-ladder .rung:not(.main):not(:disabled):hover .rung-text {
    color: var(--ice-fg);
  }

  .card-ladder .rung.gap .rung-text {
    color: var(--ice-lyric-muted);
  }

  /* ---- Karaoke words (current line) ---- */
  .word {
    display: inline-block;
    transform-origin: center bottom;
    transition:
      transform 180ms cubic-bezier(0.22, 1, 0.36, 1),
      color 160ms ease,
      opacity 160ms ease,
      text-shadow 180ms ease;
  }

  .word.spaced {
    margin-right: 0.28em;
  }

  .word.upcoming {
    color: var(--ice-lyric-muted);
    transform: translateY(0) scale(1);
  }

  .word.active {
    color: var(--ice-lyric-active);
    transform: translateY(-1px) scale(1.07);
    text-shadow: 0 0 10px var(--ice-lyric-glow);
  }

  .word.sung {
    color: var(--ice-fg);
    opacity: 0.88;
    transform: translateY(0) scale(1);
  }

  /* ---- Subtitle / Romaji: only under the main row; fades in as a line moves into it ---- */
  .subtitle-line {
    display: block;
    max-width: 100%;
    font-size: 9.5px;
    font-weight: 500;
    line-height: 13px;
    color: var(--ice-lyric-sub);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0;
    transition: opacity 320ms ease;
  }

  .subtitle-line.shown {
    opacity: 1;
  }

  /* Generated Romaji: unsung words dimmer, sung words lit, following the karaoke line above. */
  .romaji-word {
    color: var(--ice-lyric-muted);
    transition: color 200ms ease;
  }

  .romaji-word.lit {
    color: var(--ice-lyric-sub);
  }

  .plain-scroll {
    height: 100%;
    overflow-y: auto;
    padding: 2px 6px;
    font-size: 11px;
    line-height: 1.45;
    color: var(--ice-fg-muted);
    white-space: pre-line;
    text-align: center;
  }

  .lyrics-ext.instant .rung,
  .lyrics-ext.instant .rung-enter,
  .lyrics-ext.instant .rung-text,
  .lyrics-ext.instant .fit,
  .lyrics-ext.instant .word,
  .lyrics-ext.instant .subtitle-line,
  .lyrics-ext.instant .romaji-word {
    transition: none;
    animation: none;
  }

  @keyframes lyric-enter {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
