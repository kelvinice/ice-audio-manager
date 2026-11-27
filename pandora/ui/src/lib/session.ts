import type { Track } from "./types";

/**
 * The listening session that Pandora reopens at launch: the queue, where in it you were, and how far into the song.
 * A per-machine view convenience in localStorage (like Spotify showing the last song paused); library data such as
 * history stays in `library.json`. Every access is guarded: storage can be unavailable, and then nothing is restored.
 */
export interface SavedSession {
  queue: Track[];
  index: number;
  positionMs: number;
}

const KEY = "pandora.session.v1";
/** Keeps the saved queue small; the songs around the current one are what matter. */
const MAX_QUEUE = 200;
/** Within this much of the end, the song counts as finished and starts over next time. */
const END_SLACK_MS = 5_000;

export function saveSession(queue: Track[], index: number, positionMs: number): void {
  if (index < 0 || index >= queue.length) return;
  const start = Math.max(0, Math.min(index - MAX_QUEUE / 2, queue.length - MAX_QUEUE));
  const kept = queue.slice(start, start + MAX_QUEUE);
  const value: SavedSession = { queue: kept, index: index - start, positionMs: Math.max(0, Math.round(positionMs)) };
  try {
    localStorage.setItem(KEY, JSON.stringify(value));
  } catch {
    // Unavailable or full storage: the next launch simply starts without a restored song.
  }
}

/**
 * What to show at launch. Uses the saved session when its song is still the most recent one played (`lastPlayed`,
 * from history); otherwise that last-played song alone, from the start. `null` when nothing was ever played.
 */
export function loadSession(lastPlayed: Track | null): SavedSession | null {
  let saved: SavedSession | null = null;
  try {
    const raw = localStorage.getItem(KEY);
    saved = raw ? (JSON.parse(raw) as SavedSession) : null;
  } catch {
    saved = null;
  }
  const valid =
    saved !== null &&
    Array.isArray(saved.queue) &&
    Number.isInteger(saved.index) &&
    saved.index >= 0 &&
    saved.index < saved.queue.length &&
    typeof saved.queue[saved.index]?.id === "string";
  if (valid && saved && (!lastPlayed || saved.queue[saved.index].id === lastPlayed.id)) {
    const track = saved.queue[saved.index];
    const duration = track.durationMs ?? 0;
    const position = Number.isFinite(saved.positionMs) ? Math.max(0, saved.positionMs) : 0;
    return { ...saved, positionMs: duration > 0 && position > duration - END_SLACK_MS ? 0 : position };
  }
  return lastPlayed ? { queue: [lastPlayed], index: 0, positionMs: 0 } : null;
}
