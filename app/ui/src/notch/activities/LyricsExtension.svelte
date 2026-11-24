<script lang="ts">
  import type { LyricsState, NowPlaying } from '../../lib/ipc/engine';
  import { resolveLyricFrame } from '../../lib/notch/lyrics';
  import { toMsNumber } from '../../lib/notch/scheduler';
  import { liveMediaPositionMs } from '../../lib/notch/scrubber';

  interface Props {
    lyrics: LyricsState;
    media: NowPlaying;
    mode: 'pill' | 'card';
    wordHighlight: boolean;
    showRomaji: boolean;
    instant: boolean;
    notchVisible: boolean;
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

  const elapsedMs = $derived(liveMediaPositionMs(media, nowEpochMs, fallbackAnchorMs));
  const frame = $derived(resolveLyricFrame(doc, elapsedMs, wordHighlight));

  // Event-aligned wake: sleeps until the next word or line boundary, zero polling when paused/hidden.
  $effect(() => {
    if (!isPlaying || !notchVisible || isPlain || !doc) return;
    nowEpochMs = Date.now();
    let timerId: number | null = null;
    const schedule = () => {
      const currentMs = liveMediaPositionMs(media, Date.now(), fallbackAnchorMs);
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

  const activeLine = $derived(frame.activeLine ?? (mode === 'pill' ? frame.nextLine : null));
  // Pill second row: the next line; while the first row already previews it (intro or a gap), the one after.
  const pillNextLine = $derived(frame.activeLine ? frame.nextLine : frame.followingLine);
  const activeSubtitle = $derived(showRomaji ? (activeLine?.subtitle ?? null) : null);
  const highlightActiveWords = $derived(
    wordHighlight && !instant && frame.activeLine !== null && frame.activeLine.words.length > 0,
  );
  const useWordSpaces = $derived(
    Boolean(frame.activeLine && !/[\u3040-\u30ff\u4e00-\u9fff]/.test(frame.activeLine.text)),
  );
</script>

<div class="lyrics-ext" class:pill={mode === 'pill'} class:card={mode === 'card'} class:instant>
  {#if mode === 'pill'}
    <div class="pill-inner">
      <span class="note-dot" class:playing={isPlaying} aria-hidden="true">♪</span>
      <div class="pill-lines">
        {#key frame.activeIndex}
          <div class="line-wrap" class:animate-in={!instant}>
            <div class="primary-line" class:upcoming={frame.activeLine === null}>
              {#if highlightActiveWords && frame.activeLine}
                {#each frame.activeLine.words as word, idx (idx)}
                  <span
                    class="word"
                    class:active={idx === frame.activeWordIndex}
                    class:sung={idx < frame.activeWordIndex}
                    class:upcoming={idx > frame.activeWordIndex}
                    class:spaced={useWordSpaces && idx + 1 < frame.activeLine.words.length}
                  >{word.text}</span>
                {/each}
              {:else}
                {activeLine?.text ?? '···'}
              {/if}
            </div>
            {#if activeSubtitle}
              <div class="subtitle-line">{activeSubtitle}</div>
            {/if}
            <!-- Second row (owner, 2026-11-24): the line that comes next, dimmed. It slides up into the first row. -->
            <div class="next-line" aria-label={pillNextLine ? `Next: ${pillNextLine.text}` : undefined}>
              {pillNextLine?.text ?? ''}
            </div>
          </div>
        {/key}
      </div>
    </div>
  {:else if isPlain && doc?.plainText}
    <div class="plain-scroll" aria-label="Song lyrics">
      {doc.plainText}
    </div>
  {:else}
    <div class="card-lines" aria-label="Synced lyrics">
      <button
        class="ctx-line prev"
        type="button"
        disabled={!frame.prevLine}
        onclick={() => frame.prevLine && onSeek(frame.prevLine.startMs)}
      >
        {frame.prevLine?.text ?? ''}
      </button>

      {#key frame.activeIndex}
        <button
          class="active-block"
          class:animate-in={!instant}
          type="button"
          disabled={!frame.activeLine}
          onclick={() => frame.activeLine && onSeek(frame.activeLine.startMs)}
        >
          <span class="primary-line" class:upcoming={frame.activeLine === null}>
            {#if highlightActiveWords && frame.activeLine}
              {#each frame.activeLine.words as word, idx (idx)}
                <span
                  class="word"
                  class:active={idx === frame.activeWordIndex}
                  class:sung={idx < frame.activeWordIndex}
                  class:upcoming={idx > frame.activeWordIndex}
                  class:spaced={useWordSpaces && idx + 1 < frame.activeLine.words.length}
                >{word.text}</span>
              {/each}
            {:else}
              {frame.activeLine?.text ?? '···'}
            {/if}
          </span>
          {#if showRomaji && frame.activeLine?.subtitle}
            <span class="subtitle-line">{frame.activeLine.subtitle}</span>
          {/if}
        </button>
      {/key}

      <button
        class="ctx-line next"
        type="button"
        disabled={!frame.nextLine}
        onclick={() => frame.nextLine && onSeek(frame.nextLine.startMs)}
      >
        {frame.nextLine?.text ?? ''}
      </button>
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

  /* Aligned with the first (current) row, not centred between the two. */
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

  .pill-lines {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .line-wrap {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  /* Fixed height even when there's no next line, so the pill never jumps (PILL_LYRICS_H in geometry.ts). */
  .next-line {
    height: 15px;
    margin-top: 1px;
    font-size: 10.5px;
    font-weight: 500;
    line-height: 15px;
    color: var(--ice-lyric-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .lyrics-ext.card {
    height: 82px;
    padding: 6px 8px 8px;
    mask-image: linear-gradient(180deg, transparent 0%, #000 14%, #000 86%, transparent 100%);
  }

  .card-lines {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    min-width: 0;
    width: 100%;
  }

  .ctx-line,
  .active-block {
    appearance: none;
    border: none;
    background: transparent;
    padding: 0 4px;
    margin: 0;
    width: 100%;
    min-width: 0;
    font-family: inherit;
    text-align: center;
    cursor: pointer;
    transition: opacity 160ms ease, color 160ms ease;
  }

  .ctx-line:disabled,
  .active-block:disabled {
    cursor: default;
  }

  .ctx-line {
    height: 15px;
    font-size: 10.5px;
    font-weight: 500;
    line-height: 15px;
    color: var(--ice-lyric-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ctx-line:not(:disabled):hover {
    color: var(--ice-fg);
  }

  .active-block {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
  }

  .primary-line {
    display: block;
    max-width: 100%;
    font-size: 12px;
    font-weight: 600;
    line-height: 16px;
    letter-spacing: -0.01em;
    color: var(--ice-lyric-active);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .lyrics-ext.card .primary-line {
    font-size: 13px;
    line-height: 18px;
  }

  .primary-line.upcoming {
    color: var(--ice-lyric-muted);
    font-weight: 500;
  }

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

  .animate-in {
    animation: lyric-rise 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .lyrics-ext.instant .word,
  .lyrics-ext.instant .ctx-line,
  .lyrics-ext.instant .active-block {
    transition: none;
    animation: none;
  }

  @keyframes lyric-rise {
    from {
      opacity: 0;
      transform: translateY(5px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
