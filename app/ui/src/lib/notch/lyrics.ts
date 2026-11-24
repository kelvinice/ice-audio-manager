import type { LyricLine, LyricsConfig, LyricsDocument, LyricsState } from '../ipc/engine.ts';

export interface LyricFrame {
  activeIndex: number;
  prevLine: LyricLine | null;
  /** The text line before `prevLine`: the card keeps it just above view so the top line scrolls out, not pops. */
  beforePrevLine: LyricLine | null;
  activeLine: LyricLine | null;
  nextLine: LyricLine | null;
  /** The text line after `nextLine`: the music pill's second row while `activeLine` is null (before the first line or
   *  in a gap), when the first row is already previewing `nextLine`. */
  followingLine: LyricLine | null;
  activeWordIndex: number;
  nextWakeDelayMs: number | null;
}

/** True when a track has non-empty synced lines eligible for the compact music-pill bottom extension. */
export function hasSyncedLyrics(lyrics: LyricsState | null | undefined): boolean {
  const doc = lyrics?.document;
  return Boolean(
    lyrics?.status === 'ready' &&
      doc &&
      !doc.instrumental &&
      doc.syncKind !== 'plain' &&
      doc.lines.some((l) => l.text.trim().length > 0),
  );
}

/** True when a track has either synced lines or plain text eligible for the expanded card bottom extension. */
export function hasAnyLyrics(lyrics: LyricsState | null | undefined): boolean {
  const doc = lyrics?.document;
  if (lyrics?.status !== 'ready' || !doc || doc.instrumental) return false;
  return doc.lines.some((l) => l.text.trim().length > 0) || Boolean(doc.plainText?.trim());
}

/** True when any synced line shows a subtitle row: a paired line from the source, or generated Romaji while on. */
export function hasSubtitleLines(doc: LyricsDocument | null | undefined, romajiEnabled: boolean): boolean {
  if (!doc) return false;
  return doc.lines.some((l) => lineSubtitle(l, romajiEnabled) !== null);
}

/**
 * The subtitle row for a line. The source's own paired line (translation or Romaji in the `.lrc`) always shows;
 * Romaji generated on this PC only while the Romaji setting is on (it used to hide paired translations too).
 */
export function lineSubtitle(
  line: LyricLine | null | undefined,
  romajiEnabled: boolean,
): { text: string; generatedRomaji: boolean } | null {
  const paired = line?.subtitle?.trim();
  if (paired) return { text: paired, generatedRomaji: false };
  const romaji = romajiEnabled ? line?.romaji?.trim() : undefined;
  return romaji ? { text: romaji, generatedRomaji: true } : null;
}

/**
 * Romaji words to light up while the line is sung: the sung share of the line's words mapped onto the Romaji's
 * words (the two splits differ, so it's proportional, not word-for-word). All lit when there's no word progress.
 */
export function romajiLitCount(romajiWords: number, activeWordIndex: number, wordCount: number): number {
  if (romajiWords <= 0) return 0;
  if (wordCount <= 0 || activeWordIndex < 0) return activeWordIndex < 0 && wordCount > 0 ? 0 : romajiWords;
  return Math.min(romajiWords, Math.round(((activeWordIndex + 1) / wordCount) * romajiWords));
}

export type LyricRole = 'past' | 'current' | 'upcoming' | 'next' | 'gap';

/** One rendered lyric row. Rows are keyed by line, so a line keeps its element while it moves between slots. */
export interface LyricRow {
  key: string;
  line: LyricLine | null;
  /** -2/-1 above (leaving or the card's dim previous line), 0 the main row, 1 the next row. */
  slot: number;
  role: LyricRole;
}

/**
 * The rows for a frame. When the song moves on, the next line's row slides up into the main slot, the old main
 * line slides up and out, and only the newly visible line fades in; that slide is the line-change animation.
 * - pill: past (hidden above), current (or the upcoming line in an intro/gap), next.
 * - card: before-previous (hidden above), previous (dim), current (or "···" in a gap), next.
 */
export function lyricRows(frame: LyricFrame, mode: 'pill' | 'card' | 'nook'): LyricRow[] {
  const rows: LyricRow[] = [];
  const add = (line: LyricLine | null, slot: number, role: LyricRole) => {
    if (line && !rows.some((r) => r.key === String(line.startMs))) {
      rows.push({ key: String(line.startMs), line, slot, role });
    }
  };
  if (mode === 'pill') {
    add(frame.prevLine, -1, 'past');
    if (frame.activeLine) {
      add(frame.activeLine, 0, 'current');
      add(frame.nextLine, 1, 'next');
    } else {
      add(frame.nextLine, 0, 'upcoming');
      add(frame.followingLine, 1, 'next');
    }
    return rows;
  }
  add(frame.beforePrevLine, -2, 'past');
  add(frame.prevLine, -1, 'past');
  if (frame.activeLine) add(frame.activeLine, 0, 'current');
  else rows.push({ key: 'gap', line: null, slot: 0, role: 'gap' });
  add(frame.nextLine, 1, 'next');
  return rows;
}

/** Row heights (px) for the ladder layout. Twin: `.rung` sizes in LyricsExtension.svelte. */
export const LADDER = {
  /** Pill: current row 16, subtitle 14 (13 + 1 gap). Rows above slot 0 sit one row up, out of view. */
  pill: { row: 16, sub: 14 },
  /** Card: 68 px inside the 82 px drawer; context rows 15, current 18, subtitle 14, 3 px gaps. */
  card: { inner: 68, ctx: 15, cur: 18, sub: 14, gap: 3 },
  /** Nook: 152 px side-by-side column stage; generous vertical rhythm inspired by NotchNook. The current line wraps
   * (up to 3 rows of 22 px + a 16 px subtitle = `mainMax`) instead of shrinking or sliding, see `nookLadderY`. */
  nook: { inner: 152, ctx: 22, cur: 24, sub: 16, gap: 8, mainMax: 82 },
} as const;

/**
 * Edge Nook: vertical position (px) of a slot around the current line's measured height `mainH` (text + subtitle).
 * The current line is centred in the column; context rows sit one `ctx + gap` step above and below it. Heights are
 * clamped to one row … `mainMax`, so a 3-line current line still leaves the next line inside the column.
 */
export function nookLadderY(slot: number, mainH: number): number {
  const { inner, ctx, cur, gap, mainMax } = LADDER.nook;
  const h = Math.min(Math.max(mainH, cur), mainMax);
  const y0 = Math.round((inner - h) / 2);
  if (slot === 0) return y0;
  if (slot < 0) return y0 + slot * (ctx + gap);
  return y0 + h + gap + (slot - 1) * (ctx + gap);
}

/** Vertical position (px) of a slot. `hasSub` reserves the subtitle row so positions never jump between lines. */
export function ladderY(mode: 'pill' | 'card' | 'nook', slot: number, hasSub: boolean): number {
  if (mode === 'pill') {
    const { row, sub } = LADDER.pill;
    return slot <= 0 ? slot * row : row + (hasSub ? sub : 0) + (slot - 1) * row;
  }
  if (mode === 'nook') return nookLadderY(slot, LADDER.nook.cur + (hasSub ? LADDER.nook.sub : 0));
  const { inner, ctx, cur, sub, gap } = LADDER.card;
  const curH = cur + (hasSub ? sub : 0);
  const top = (inner - (ctx + gap + curH + gap + ctx)) / 2;
  if (slot <= -1) return top + (slot + 1) * (ctx + gap);
  if (slot === 0) return top + ctx + gap;
  return top + ctx + gap + curH + gap + (slot - 1) * (ctx + gap);
}

export function shouldShowPillLyrics(
  config: LyricsConfig | null | undefined,
  lyrics: LyricsState | null | undefined,
  hasMedia: boolean,
): boolean {
  return Boolean(hasMedia && config?.enabled && config.showOnPill && hasSyncedLyrics(lyrics));
}

export function shouldShowCardLyrics(
  config: LyricsConfig | null | undefined,
  lyrics: LyricsState | null | undefined,
  hasMedia: boolean,
  drawerOpen: boolean,
): boolean {
  return Boolean(hasMedia && config?.enabled && config.showOnCard && drawerOpen && hasAnyLyrics(lyrics));
}

/** Resolves the active lyric line, neighbour context lines, active word index, and next boundary delay. */
export function resolveLyricFrame(
  doc: LyricsDocument | null | undefined,
  elapsedMs: number,
  wordHighlight: boolean,
): LyricFrame {
  const empty: LyricFrame = {
    activeIndex: -1,
    prevLine: null,
    beforePrevLine: null,
    activeLine: null,
    nextLine: null,
    followingLine: null,
    activeWordIndex: -1,
    nextWakeDelayMs: null,
  };
  if (!doc || doc.instrumental || doc.syncKind === 'plain' || doc.lines.length === 0) {
    return empty;
  }

  const lines = doc.lines;
  let activeIndex = -1;
  for (let i = 0; i < lines.length; i += 1) {
    if (lines[i].startMs <= elapsedMs) {
      activeIndex = i;
    } else {
      break;
    }
  }

  // Before the first lyric starts: preview first line as upcoming and schedule wake right at its start.
  if (activeIndex < 0) {
    const firstIndex = nextTextIndex(lines, -1);
    const first = firstIndex >= 0 ? lines[firstIndex] : null;
    const delay = first ? clampWakeDelay(first.startMs - elapsedMs) : null;
    return {
      activeIndex: -1,
      prevLine: null,
      beforePrevLine: null,
      activeLine: null,
      nextLine: first,
      followingLine: firstIndex >= 0 ? textLineAt(lines, nextTextIndex(lines, firstIndex)) : null,
      activeWordIndex: -1,
      nextWakeDelayMs: delay,
    };
  }

  const rawActive = lines[activeIndex];
  const isGap = rawActive.text.trim().length === 0;
  const activeLine = isGap ? null : rawActive;
  const prevIndex = prevTextIndex(lines, activeIndex);
  const prevLine = textLineAt(lines, prevIndex);
  const beforePrevLine = prevIndex >= 0 ? textLineAt(lines, prevTextIndex(lines, prevIndex)) : null;
  const nextIndex = nextTextIndex(lines, activeIndex);
  const nextLine = textLineAt(lines, nextIndex);
  const followingLine = nextIndex >= 0 ? textLineAt(lines, nextTextIndex(lines, nextIndex)) : null;

  let activeWordIndex = -1;
  let nextWordAt: number | null = null;
  if (activeLine && wordHighlight && activeLine.words.length > 0) {
    for (let i = 0; i < activeLine.words.length; i += 1) {
      const w = activeLine.words[i];
      if (w.startMs <= elapsedMs) {
        activeWordIndex = i;
      } else {
        nextWordAt = w.startMs;
        break;
      }
    }
    if (nextWordAt === null && activeWordIndex >= 0) {
      const cur = activeLine.words[activeWordIndex];
      if (cur.endMs > elapsedMs) {
        nextWordAt = cur.endMs;
      }
    }
  }

  const nextLineAt = nextLine ? nextLine.startMs : rawActive.endMs && rawActive.endMs > elapsedMs ? rawActive.endMs : null;
  const nextBoundary =
    nextWordAt !== null && nextLineAt !== null
      ? Math.min(nextWordAt, nextLineAt)
      : (nextWordAt ?? nextLineAt);

  return {
    activeIndex: isGap ? -1 : activeIndex,
    prevLine,
    beforePrevLine,
    activeLine,
    nextLine,
    followingLine,
    activeWordIndex,
    nextWakeDelayMs: nextBoundary !== null ? clampWakeDelay(nextBoundary - elapsedMs) : null,
  };
}

/** Index of the last non-empty line before `from`, or -1. */
function prevTextIndex(lines: LyricLine[], from: number): number {
  for (let i = from - 1; i >= 0; i -= 1) {
    if (lines[i].text.trim().length > 0) return i;
  }
  return -1;
}

/** Index of the first non-empty line after `from`, or -1. */
function nextTextIndex(lines: LyricLine[], from: number): number {
  for (let i = from + 1; i < lines.length; i += 1) {
    if (lines[i].text.trim().length > 0) return i;
  }
  return -1;
}

function textLineAt(lines: LyricLine[], index: number): LyricLine | null {
  return index >= 0 ? lines[index] : null;
}

function clampWakeDelay(deltaMs: number): number {
  if (!Number.isFinite(deltaMs)) return 250;
  return Math.max(45, Math.min(2_000, Math.ceil(deltaMs) + 15));
}
