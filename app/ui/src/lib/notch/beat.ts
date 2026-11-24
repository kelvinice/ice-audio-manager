import type { MeterFrame } from '../ipc/engine.ts';

/**
 * Beat-driven bar targets for the notch visualizer (owner, 2026-11-24: "make the user feel the beat go up and down").
 *
 * The engine's meter gives only a ~30 Hz peak per channel, no spectrum, and loud masters sit at 0.7–1.0 almost every
 * frame, so mapping loudness straight to height made the bars twitch near the top. This works on *change* instead:
 *
 * - **Adaptive range (dB):** each channel tracks its running body level (`FLOOR_TAU_S`) and recent peak. A bar's
 *   swing is how far the current frame sits above the body, so any track, loud or quiet, uses the full height,
 *   and between beats the bars fall back down.
 * - **Beats:** a sharp rise (`BEAT_ONSET`) starts a new beat, and each beat throws the bars into a new pattern
 *   (`spark`), so hits don't all look the same.
 * - **Two characters:** centre bars swing hardest with the beat's body; outer bars flick on fast rises. This is a
 *   *feel*, not real pitch: real bass/treble bands would need the engine to analyse the audio itself.
 * - **Silence** → every bar at rest (lowest).
 *
 * Pure and deterministic (no randomness), so it's unit-tested; `out` is reused to avoid per-frame allocation.
 */

/** Bars the shared bank animates. The 5-bar variants pick from these (`pickBars`). */
export const BEAT_BAR_COUNT = 14;

const SILENT = 0.001;
/** Running body level of the track. */
const FLOOR_TAU_S = 1.2;
/** How fast the remembered recent peak falls back. */
const CEIL_RELEASE_DB_PER_S = 4;
/** Never treat less than this much swing as full height (a steady tone stays low). */
const MIN_RANGE_DB = 6;
/** At and below this a channel counts as quiet (no body). */
const QUIET_DB = -48;
/** The floor never follows below this, so music returning after silence doesn't spike for seconds. */
const FLOOR_MIN_DB = -60;
/** A rise this large within one frame is a full-strength onset. */
const ONSET_FULL_DB = 6;
/** Onset strength that starts a new beat pattern; the next needs the onset to drop below half of it first. */
const BEAT_ONSET = 0.3;
const MAX_DT_S = 0.25;

function clamp01(v: number): number {
  return Number.isFinite(v) ? Math.max(0, Math.min(1, v)) : 0;
}

/** Deterministic 0–1 hash of (beat, bar): a new pattern per beat without `Math.random`. */
export function beatHash(beat: number, bar: number): number {
  const x = Math.sin(beat * 12.9898 + bar * 78.233) * 43758.5453;
  return x - Math.floor(x);
}

class Follower {
  private floor = Number.NaN;
  private ceil = Number.NaN;
  private prevDb = Number.NaN;
  punch = 0;
  onset = 0;
  presence = 0;

  reset(): void {
    this.floor = Number.NaN;
    this.ceil = Number.NaN;
    this.prevDb = Number.NaN;
    this.punch = 0;
    this.onset = 0;
    this.presence = 0;
  }

  update(peak: number, dt: number): void {
    const db = 20 * Math.log10(Math.max(peak, 1e-5));
    const tracked = Math.max(db, FLOOR_MIN_DB);
    if (Number.isNaN(this.floor)) {
      this.floor = tracked - 3;
      this.ceil = tracked;
      this.prevDb = db;
    } else {
      this.floor += (tracked - this.floor) * (1 - Math.exp(-dt / FLOOR_TAU_S));
      this.ceil = Math.max(tracked, this.ceil - CEIL_RELEASE_DB_PER_S * dt);
    }
    this.ceil = Math.max(this.ceil, this.floor + MIN_RANGE_DB);
    this.punch = clamp01((db - this.floor) / (this.ceil - this.floor));
    this.onset = clamp01((db - this.prevDb) / ONSET_FULL_DB);
    this.presence = clamp01((db - QUIET_DB) / -QUIET_DB);
    this.prevDb = db;
  }
}

/** Left/right peaks; surround endpoints playing stereo use their first pair; mono or no channels use the peak. */
function stereo(frame: MeterFrame): [number, number] {
  const ch = frame.channels;
  if (ch.length >= 2) return [clamp01(ch[0]), clamp01(ch[1])];
  if (ch.length === 1) return [clamp01(ch[0]), clamp01(ch[0])];
  const p = clamp01(frame.peak);
  return [p, p];
}

export class BeatShaper {
  private readonly left = new Follower();
  private readonly right = new Follower();
  private readonly count: number;
  private beat = 0;
  private armed = true;
  private lastMs: number | null = null;

  constructor(count = BEAT_BAR_COUNT) {
    this.count = Math.max(1, Math.round(count));
  }

  reset(): void {
    this.left.reset();
    this.right.reset();
    this.armed = true;
    this.lastMs = null;
  }

  shape(frame: MeterFrame, nowMs: number, out: number[] = []): number[] {
    out.length = this.count;
    const dt = this.lastMs === null ? 1 / 30 : Math.max(0.001, Math.min(MAX_DT_S, (nowMs - this.lastMs) / 1000));
    this.lastMs = nowMs;
    const [l, r] = stereo(frame);
    if (Math.max(l, r, clamp01(frame.peak)) < SILENT) {
      out.fill(0);
      this.armed = true;
      return out;
    }
    this.left.update(l, dt);
    this.right.update(r, dt);

    const onset = Math.max(this.left.onset, this.right.onset);
    if (onset >= BEAT_ONSET && this.armed) {
      this.beat += 1;
      this.armed = false;
    } else if (onset < BEAT_ONSET / 2) {
      this.armed = true;
    }

    for (let i = 0; i < this.count; i += 1) {
      const t = this.count > 1 ? i / (this.count - 1) : 0.5;
      const edge = Math.abs(t - 0.5) * 2; // 0 at the centre, 1 at the ends
      const mix = (a: number, b: number) => a * (1 - t) + b * t; // left bars follow the left channel
      const presence = mix(this.left.presence, this.right.presence);
      const punch = mix(this.left.punch, this.right.punch);
      const rise = mix(this.left.onset, this.right.onset);
      const spark = 0.35 + 0.65 * beatHash(this.beat, i);

      const body = presence * (0.1 + 0.1 * (1 - edge)); // quiet music sits low, silence at rest
      const swing = punch * spark * (0.6 + 0.3 * (1 - edge)); // the beat lifts the centre most
      const flick = rise * beatHash(this.beat * 7 + 3, i) * (0.25 + 0.4 * edge); // fast rises flick the edges
      out[i] = clamp01(body + swing + flick);
    }
    return out;
  }
}

/** `count` bars taken evenly from the bank (each stays an independent bar, no interpolation). */
export function pickBars(levels: ReadonlyArray<number>, count: number, out: number[] = []): number[] {
  out.length = count;
  const n = levels.length;
  for (let i = 0; i < count; i += 1) {
    const src = n > 1 && count > 1 ? Math.round((i * (n - 1)) / (count - 1)) : 0;
    out[i] = clamp01(levels[src] ?? 0);
  }
  return out;
}
