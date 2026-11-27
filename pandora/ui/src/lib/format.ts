import type { Track } from "./types";

/** `3:07`, or an em dash when the length is unknown. */
export function formatTrackDuration(ms: number | null): string {
  if (!ms || ms <= 0) return "—";
  const totalSec = Math.floor(ms / 1000);
  return `${Math.floor(totalSec / 60)}:${(totalSec % 60).toString().padStart(2, "0")}`;
}

/** `1 hr 30 min` / `42 min` for a list, or null when no track has a length. */
export function formatTotalDuration(tracks: Track[]): string | null {
  const totalMs = tracks.reduce((sum, t) => sum + (t.durationMs ?? 0), 0);
  if (totalMs <= 0) return null;
  const totalMin = Math.round(totalMs / 60_000);
  if (totalMin < 1) return "< 1 min";
  const hrs = Math.floor(totalMin / 60);
  const mins = totalMin % 60;
  return hrs > 0 ? `${hrs} hr ${mins} min` : `${mins} min`;
}

export function formatCount(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

export function formatMb(bytes: number): string {
  const mb = bytes / (1024 * 1024);
  return mb >= 100 ? `${Math.round(mb)} MB` : `${mb.toFixed(1)} MB`;
}

/** Shuffled copy (Fisher–Yates; `sort(random)` is biased). */
export function shuffled<T>(list: T[]): T[] {
  const out = [...list];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}
