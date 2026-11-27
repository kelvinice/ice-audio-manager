import { ytmRecommend } from "./ipc";
import type { Track } from "./types";

/**
 * Magic shuffle: while shuffling a list (usually a playlist), every few songs one recommended song plays in between.
 * Recommendations come from `ytm_recommend` (seeded by the list's own songs, ranked in Rust) and are fetched a batch
 * at a time; each batch asks about other songs of the list and skips everything suggested before.
 */

/** List songs between two recommended songs: 2 to 4, so the mix doesn't feel mechanical. */
const MIN_GAP = 2;
const MAX_GAP = 4;
/** Fetch the next batch when this few are left. */
const LOW_WATER = 2;

function gap(): number {
  return MIN_GAP + Math.floor(Math.random() * (MAX_GAP - MIN_GAP + 1));
}

export class MagicShuffle {
  /** Ids this mode put into the queue, so the queue panel can mark them and they never seed or count as the list. */
  readonly added = new Set<string>();
  private base: Track[] = [];
  private pool: Track[] = [];
  private suggested = new Set<string>();
  private round = 0;
  private countdown = gap();
  private generation = 0;
  private fetching = false;

  /** Starts over for a new list (its songs without the ones this mode added). */
  reset(base: Track[]): void {
    this.generation += 1;
    this.base = base;
    this.pool = [];
    this.suggested.clear();
    this.added.clear();
    this.round = 0;
    this.countdown = gap();
    this.fetching = false;
  }

  /** Fetches a batch ahead of time (when the mode is switched on or a new list starts). */
  prime(): void {
    if (this.pool.length <= LOW_WATER) void this.refill();
  }

  /** Called before each song change: a recommended song to play next, or null to play a list song. */
  takeTurn(): Track | null {
    this.countdown -= 1;
    this.prime();
    if (this.countdown > 0) return null;
    const next = this.pool.shift() ?? null;
    if (next) {
      this.countdown = gap();
      this.added.add(next.id);
    }
    return next;
  }

  private async refill(): Promise<void> {
    if (this.fetching || !this.base.some((t) => t.source === "ytmusic")) return;
    this.fetching = true;
    const generation = this.generation;
    try {
      const recs = await ytmRecommend(this.base, this.round++, [...this.suggested]);
      if (generation !== this.generation) return;
      for (const rec of recs) {
        if (this.suggested.has(rec.id)) continue;
        this.suggested.add(rec.id);
        this.pool.push(rec);
      }
    } catch {
      // Offline or YouTube Music unreachable: the list keeps shuffling on its own.
    } finally {
      if (generation === this.generation) this.fetching = false;
    }
  }
}
