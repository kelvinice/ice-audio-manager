import type { PandoraLyricLine } from "./types";

export interface TimedWord {
  text: string;
  startMs: number;
  endMs: number;
}

/** A line with no end time or next line still gets a sensible length. */
const FALLBACK_LINE_MS = 4_000;
/** Lines with no spaces (Japanese, Chinese) are timed per character; long ones are grouped to keep a steady pace. */
const CJK_RUN = /[぀-ヿ㐀-鿿豈-﫿가-힯]/;

/**
 * Word timings for a line: the provider's own when present, otherwise the line's time shared out by length (longer
 * words take longer, as sung). `nextStartMs` bounds lines that have no end time.
 */
export function timedWords(line: PandoraLyricLine, nextStartMs: number | null): TimedWord[] {
  if (line.words.length > 0) return line.words.map((w) => ({ text: w.text, startMs: w.startMs, endMs: w.endMs }));
  const text = line.text.trim();
  if (!text) return [];
  const pieces = text.includes(" ") || !CJK_RUN.test(text) ? text.split(/\s+/) : Array.from(text);
  const end = line.endMs ?? nextStartMs ?? line.timeMs + FALLBACK_LINE_MS;
  const span = Math.max(300, end - line.timeMs);
  const weights = pieces.map((p) => Math.max(1, p.length));
  const total = weights.reduce((a, b) => a + b, 0);
  let at = line.timeMs;
  return pieces.map((piece, i) => {
    const dur = (span * weights[i]) / total;
    const word = { text: piece, startMs: Math.round(at), endMs: Math.round(at + dur) };
    at += dur;
    return word;
  });
}

/** Whether `text` is written without spaces between words (so pieces join without a gap). */
export function joinsWithoutSpaces(text: string): boolean {
  return !text.includes(" ") && CJK_RUN.test(text);
}
