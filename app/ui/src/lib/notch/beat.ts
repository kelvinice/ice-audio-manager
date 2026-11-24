import type { MeterFrame } from '../ipc/engine.ts';

/**
 * Spectrum- and beat-driven bar targets for the notch visualizer (D-26).
 *
 * Two automatic modes, zero allocations per frame:
 *
 * 1. **Spectrum mode (`A2 + B1`, when `frame.bands` carries 14 bands from local WASAPI loopback):**
 *    - **Per-band AGC + pink-noise tilt (`B1`):** lifts upper-mid/treble energy (`+3.6 dB/octave`, capped) so
 *      vocals and hi-hats swing across the full bar height alongside bass kicks.
 *    - **Audible-energy noise gate (Codex caveat):** bands below `-62 dB` ramp cleanly to `0`, so quiet hiss
 *      in empty bands is never boosted to melody height and silence settles strictly to `0`.
 *    - **Low-band beat onset accent (Claude amendment):** tracks kicks on bands `0–2` to keep the crisp beat
 *      pop and drive `tempoRate` for the ambient backdrop.
 *    - **No artificial neighbor coupling (`B3` rejected on `A2`):** the 4th-order IIR bank already gives smooth
 *      natural separation (`38%` adjacent overlap).
 *
 * 2. **Peak-only fallback (`A1`, when `frame.bands` is absent — Privacy toggle off or loopback unavailable):**
 *    - Multi-envelope travelling-wave shaper over `[L, R]` peaks (fast onset + medium punch + short wave delay
 *      line + per-beat deterministic pattern), returning strictly to `0` on silence.
 *
 * Layout (`B4a`): the 14-bar bank is ordered low→high (`bass → treble`) for the wide scrubber, while
 * `pickBadgeBars` maps it symmetrically (`[high-mid, low-mid, bass, upper-mid, treble]`) for the 5-bar badge.
 */

/** Bars the shared bank animates. The 5-bar variants map from these (`pickBadgeBars` / `pickBars`). */
export const BEAT_BAR_COUNT = 14;

const SILENT = 0.001;
/** Running body level of the track (peak-only follower & kick detector). */
const FLOOR_TAU_S = 1.2;
/** How fast the remembered recent peak falls back (dB/s). */
const CEIL_RELEASE_DB_PER_S = 4;
/** Never treat less than this much swing as full height (a steady peak-only tone stays low). */
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
const DEFAULT_BPM = 110;
const MIN_IBI_MS = 240;
const MAX_IBI_MS = 1300;
const TEMPO_EMA_ALPHA = 0.24;
const MIN_TEMPO_RATE = 0.58;
const MAX_TEMPO_RATE = 1.65;

// D-26 B1 spectrum constants (14 bands spaced ~0.637 octaves across 45 Hz – 14 kHz).
const OCTAVES_PER_BAND = Math.log2(14_000 / 45) / (BEAT_BAR_COUNT - 1);
const PINK_TILT_DB_PER_OCTAVE = 3.6;
const MAX_PINK_TILT_DB = 24;
/** Audible noise gate (raw band dB): <= -62 dB outputs 0; ramps smoothly to full gate at -48 dB. */
const BAND_GATE_FLOOR_DB = -62;
const BAND_GATE_SPAN_DB = 14;
const BAND_FLOOR_TAU_S = 1.5;
const BAND_CEIL_RELEASE_DB_PER_S = 5.0;
const BAND_MIN_RANGE_DB = 11;
const BAND_PRESENCE_FLOOR_DB = -46;
const BAND_PRESENCE_SPAN_DB = 40;
/** Travelling-wave delay history length for the A1 peak-only fallback. */
const WAVE_HISTORY_LEN = 6;

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

/** Per-band adaptive range tracker for D-26 B1 (gated on raw audible energy). */
class BandFollower {
  private floor = Number.NaN;
  private ceil = Number.NaN;

  reset(): void {
    this.floor = Number.NaN;
    this.ceil = Number.NaN;
  }

  update(rawLevel: number, tiltDb: number, dt: number): number {
    if (rawLevel < SILENT * 0.5) {
      if (!Number.isNaN(this.ceil)) {
        this.ceil = Math.max(FLOOR_MIN_DB, this.ceil - BAND_CEIL_RELEASE_DB_PER_S * dt);
      }
      return 0;
    }
    const rawDb = 20 * Math.log10(Math.max(rawLevel, 1e-6));
    const gate = clamp01((rawDb - BAND_GATE_FLOOR_DB) / BAND_GATE_SPAN_DB);
    if (gate <= 0) {
      return 0;
    }

    const tiltedDb = Math.min(6, rawDb + tiltDb);
    const tracked = Math.max(tiltedDb, FLOOR_MIN_DB);
    if (Number.isNaN(this.floor)) {
      this.floor = tracked - 6;
      this.ceil = tracked;
    } else {
      const alpha = (1 - Math.exp(-dt / BAND_FLOOR_TAU_S)) * (0.25 + 0.75 * gate);
      this.floor += (tracked - this.floor) * alpha;
      this.ceil = Math.max(tracked, this.ceil - BAND_CEIL_RELEASE_DB_PER_S * dt);
    }
    this.ceil = Math.max(this.ceil, this.floor + BAND_MIN_RANGE_DB);

    const agc = clamp01((tiltedDb - this.floor) / (this.ceil - this.floor));
    const presence = clamp01((tiltedDb - BAND_PRESENCE_FLOOR_DB) / BAND_PRESENCE_SPAN_DB);
    // Blend melodic presence (sustained vocals/notes stay alive) with local AGC swing (transients punch).
    const shaped = 0.52 * presence + 0.48 * agc;
    return clamp01(shaped * gate);
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
  private readonly lowBeat = new Follower();
  private readonly bandFollowers: BandFollower[];
  private readonly bandTilts: number[];
  private readonly wavePunch = new Array<number>(WAVE_HISTORY_LEN).fill(0);
  private waveHead = 0;
  private readonly count: number;
  private beat = 0;
  private armed = true;
  private lastMs: number | null = null;
  private lastBeatMs: number | null = null;
  private smoothedBpm = DEFAULT_BPM;
  private lastMode: 'spectrum' | 'peak' | null = null;

  constructor(count = BEAT_BAR_COUNT) {
    this.count = Math.max(1, Math.round(count));
    this.bandFollowers = Array.from({ length: this.count }, () => new BandFollower());
    this.bandTilts = Array.from({ length: this.count }, (_, i) =>
      Math.min(MAX_PINK_TILT_DB, i * OCTAVES_PER_BAND * PINK_TILT_DB_PER_OCTAVE),
    );
  }

  /** Smoothed animation speed multiplier (~0.58× slow ballads .. ~1.65× fast beats, 1.0 at ~110 BPM). */
  get tempoRate(): number {
    return Math.max(MIN_TEMPO_RATE, Math.min(MAX_TEMPO_RATE, this.smoothedBpm / DEFAULT_BPM));
  }

  reset(): void {
    this.left.reset();
    this.right.reset();
    this.lowBeat.reset();
    for (const f of this.bandFollowers) f.reset();
    this.wavePunch.fill(0);
    this.waveHead = 0;
    this.armed = true;
    this.lastMs = null;
    this.lastBeatMs = null;
    this.smoothedBpm = DEFAULT_BPM;
    this.lastMode = null;
  }

  private trackBeatOnset(onset: number, nowMs: number): void {
    if (onset >= BEAT_ONSET && this.armed) {
      this.beat += 1;
      this.armed = false;
      if (this.lastBeatMs !== null) {
        const ibi = nowMs - this.lastBeatMs;
        if (ibi >= MIN_IBI_MS && ibi <= MAX_IBI_MS) {
          let bpm = 60_000 / ibi;
          while (bpm > 165) bpm /= 2;
          while (bpm < 65) bpm *= 2;
          this.smoothedBpm += (bpm - this.smoothedBpm) * TEMPO_EMA_ALPHA;
        }
      }
      this.lastBeatMs = nowMs;
    } else if (onset < BEAT_ONSET / 2) {
      this.armed = true;
    }
  }

  shape(frame: MeterFrame, nowMs: number, out: number[] = []): number[] {
    out.length = this.count;
    const dt = this.lastMs === null ? 1 / 30 : Math.max(0.001, Math.min(MAX_DT_S, (nowMs - this.lastMs) / 1000));
    this.lastMs = nowMs;

    const bands = frame.bands;
    if (Array.isArray(bands) && bands.length === this.count) {
      return this.shapeSpectrum(frame, bands, dt, nowMs, out);
    }
    return this.shapePeakFallback(frame, dt, nowMs, out);
  }

  /** D-26 A2 + B1: real 14-band spectrum with audible-energy-gated AGC, pink tilt, and low-band beat accent. */
  private shapeSpectrum(
    frame: MeterFrame,
    bands: ReadonlyArray<number>,
    dt: number,
    nowMs: number,
    out: number[],
  ): number[] {
    if (this.lastMode === 'peak') {
      // Switching into spectrum mode: start from clean per-band state.
      for (const f of this.bandFollowers) f.reset();
      this.lowBeat.reset();
    }
    this.lastMode = 'spectrum';

    let maxBand = 0;
    for (let i = 0; i < this.count; i += 1) {
      const v = clamp01(bands[i] ?? 0);
      if (v > maxBand) maxBand = v;
    }
    if (maxBand < SILENT && clamp01(frame.peak) < SILENT) {
      out.fill(0);
      this.armed = true;
      return out;
    }

    // Claude's low-band beat onset detector (bands 0–2: 45–110 Hz kicks).
    const lowEnergy = Math.max(
      clamp01(bands[0] ?? 0),
      clamp01(bands[1] ?? 0),
      clamp01(bands[2] ?? 0),
    );
    this.lowBeat.update(lowEnergy, dt);
    this.trackBeatOnset(this.lowBeat.onset, nowMs);
    const kickAccent = this.lowBeat.onset;

    for (let i = 0; i < this.count; i += 1) {
      const raw = clamp01(bands[i] ?? 0);
      let level = this.bandFollowers[i].update(raw, this.bandTilts[i], dt);
      // Light low-band kick pop on bands 0–2 while audio is actually present in that band.
      if (i <= 2 && level > 0 && kickAccent > 0.1) {
        const kickWeight = 0.22 * (1 - i * 0.28);
        level = clamp01(level + kickAccent * kickWeight);
      }
      out[i] = level;
    }
    return out;
  }

  /** D-26 A1 fallback: multi-envelope travelling-wave shaper over `[L, R]` scalar peaks. */
  private shapePeakFallback(frame: MeterFrame, dt: number, nowMs: number, out: number[]): number[] {
    if (this.lastMode === 'spectrum') {
      // Discard previous spectrum state on fallback (Codex amendment).
      for (const f of this.bandFollowers) f.reset();
      this.left.reset();
      this.right.reset();
      this.wavePunch.fill(0);
    }
    this.lastMode = 'peak';

    const [l, r] = stereo(frame);
    if (Math.max(l, r, clamp01(frame.peak)) < SILENT) {
      out.fill(0);
      this.wavePunch.fill(0);
      this.armed = true;
      return out;
    }
    this.left.update(l, dt);
    this.right.update(r, dt);

    const onset = Math.max(this.left.onset, this.right.onset);
    this.trackBeatOnset(onset, nowMs);

    // Record centre beat punch into the short travelling-wave delay ring (A1 outward ripple).
    this.waveHead = (this.waveHead + 1) % WAVE_HISTORY_LEN;
    this.wavePunch[this.waveHead] = Math.max(this.left.punch, this.right.punch);

    for (let i = 0; i < this.count; i += 1) {
      const t = this.count > 1 ? i / (this.count - 1) : 0.5;
      const edge = Math.abs(t - 0.5) * 2; // 0 at the centre, 1 at the ends
      const mix = (a: number, b: number) => a * (1 - t) + b * t; // left bars follow the left channel
      const presence = mix(this.left.presence, this.right.presence);
      const localPunch = mix(this.left.punch, this.right.punch);
      // Delay outward bars by 0..3 ticks so hits ripple outward instead of freezing into one flat block.
      const lag = Math.min(WAVE_HISTORY_LEN - 1, Math.round(edge * 2.5));
      const delayedPunch = this.wavePunch[(this.waveHead - lag + WAVE_HISTORY_LEN) % WAVE_HISTORY_LEN];
      const wave = 0.72 * localPunch + 0.28 * delayedPunch;
      const rise = mix(this.left.onset, this.right.onset);
      const spark = 0.35 + 0.65 * beatHash(this.beat, i);

      const body = presence * (0.1 + 0.1 * (1 - edge)); // quiet music sits low, silence at rest
      const swing = wave * spark * (0.6 + 0.3 * (1 - edge)); // the beat lifts the centre most
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

function bandMax(levels: ReadonlyArray<number>, start: number, endInclusive: number): number {
  let m = 0;
  const hi = Math.min(levels.length - 1, endInclusive);
  for (let i = Math.max(0, start); i <= hi; i += 1) {
    const v = clamp01(levels[i] ?? 0);
    if (v > m) m = v;
  }
  return m;
}

/**
 * D-26 B4a symmetric centre-bass mapping for compact/expanded 5-bar badges:
 * `[presence/high-mid, low-mid, bass/kick, upper-mid/vocals, treble/air]`.
 * Keeps kicks anchored at the centre while vocals fill the shoulders and hi-hats flick the edges.
 */
export function pickBadgeBars(levels: ReadonlyArray<number>, count: number, out: number[] = []): number[] {
  out.length = count;
  if (levels.length === 0) {
    out.fill(0);
    return out;
  }
  if (count === 5 && levels.length >= BEAT_BAR_COUNT) {
    out[0] = bandMax(levels, 9, 11); // outer-left: presence / high-mid
    out[1] = bandMax(levels, 3, 5); // inner-left: low-mid
    out[2] = bandMax(levels, 0, 2); // centre: sub-bass & kick
    out[3] = bandMax(levels, 6, 8); // inner-right: upper-mid / vocals
    out[4] = bandMax(levels, 11, 13); // outer-right: treble / cymbals
    return out;
  }
  // General symmetric fold from centre (low) to edges (high).
  const center = (count - 1) / 2;
  const maxDist = Math.max(center, 1);
  for (let i = 0; i < count; i += 1) {
    const dist = Math.abs(i - center) / maxDist;
    const idx = Math.round(dist * (levels.length - 1));
    out[i] = clamp01(levels[idx] ?? 0);
  }
  return out;
}
