import type { LyricLine, LyricsConfig, LyricsDocument, LyricsState } from '../ipc/engine.ts';

export interface LyricFrame {
  activeIndex: number;
  prevLine: LyricLine | null;
  activeLine: LyricLine | null;
  nextLine: LyricLine | null;
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

/** True when any synced line in `doc` carries a subtitle (Romaji or paired dual-timestamp LRC). */
export function hasSubtitleLines(doc: LyricsDocument | null | undefined, romajiEnabled: boolean): boolean {
  if (!doc || !romajiEnabled) return false;
  return doc.lines.some((l) => Boolean(l.subtitle?.trim()));
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
    activeLine: null,
    nextLine: null,
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
    const first = lines.find((l) => l.text.trim().length > 0) ?? null;
    const delay = first ? clampWakeDelay(first.startMs - elapsedMs) : null;
    return {
      activeIndex: -1,
      prevLine: null,
      activeLine: null,
      nextLine: first,
      activeWordIndex: -1,
      nextWakeDelayMs: delay,
    };
  }

  const rawActive = lines[activeIndex];
  const isGap = rawActive.text.trim().length === 0;
  const activeLine = isGap ? null : rawActive;
  const prevLine = findPrevTextLine(lines, activeIndex);
  const nextLine = findNextTextLine(lines, activeIndex);

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
    activeLine,
    nextLine,
    activeWordIndex,
    nextWakeDelayMs: nextBoundary !== null ? clampWakeDelay(nextBoundary - elapsedMs) : null,
  };
}

function findPrevTextLine(lines: LyricLine[], from: number): LyricLine | null {
  for (let i = from - 1; i >= 0; i -= 1) {
    if (lines[i].text.trim().length > 0) return lines[i];
  }
  return null;
}

function findNextTextLine(lines: LyricLine[], from: number): LyricLine | null {
  for (let i = from + 1; i < lines.length; i += 1) {
    if (lines[i].text.trim().length > 0) return lines[i];
  }
  return null;
}

function clampWakeDelay(deltaMs: number): number {
  if (!Number.isFinite(deltaMs)) return 250;
  return Math.max(45, Math.min(2_000, Math.ceil(deltaMs) + 15));
}
