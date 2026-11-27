// Twin: app/ui/src/lib/notch/beat.ts (BandFollower: pink tilt, noise gate, per-band AGC blended with presence) and
// app/ui/src/lib/notch/visualizer.ts (attack/decay bar springs, frequency-graded mass, textured resting line).
// Pandora's Now Playing bars use the same shaping as Ice's notch pill so the two move alike; Ice reads WASAPI band
// levels, Pandora reads Web Audio FFT bins (dB), so the gate and presence floors below sit lower.

/**
 * Treble lift: music carries less energy up high, so without it the right half barely moves. The tilt starts below
 * zero so the loud bass bars on the left don't sit near the top all the time (Pandora only; FFT bass is hotter than
 * Ice's WASAPI bands).
 */
const TILT_DB_PER_OCTAVE = 3.2;
const BASS_TRIM_DB = -8;
const MAX_TILT_DB = 22;
/** The lowest bars reach at most this height, opening to full height over the first `BASS_REACH_SPAN` of the bars. */
const BASS_REACH = 0.72;
const BASS_REACH_SPAN = 0.3;
/** Band level (FFT bin dB) at or below which a bar stays at rest, ramping to fully open over the span. */
const GATE_FLOOR_DB = -86;
const GATE_SPAN_DB = 16;
/** Per-band automatic range: a slow floor, a ceiling that falls back slowly, and a minimum swing. */
const FLOOR_TAU_S = 1.5;
const CEIL_RELEASE_DB_PER_S = 5;
const MIN_RANGE_DB = 12;
/** Steady presence (keeps held notes and vocals alive) blended with the range swing (keeps hits punchy). */
const PRESENCE_FLOOR_DB = -72;
const PRESENCE_SPAN_DB = 46;
const PRESENCE_WEIGHT = 0.5;

/** Bars rise quickly with a touch of overshoot and fall back softly; bass bars are a little heavier. */
const ATTACK_STIFFNESS = 900;
const ATTACK_DAMPING_RATIO = 0.72;
const DECAY_STIFFNESS = 140;
const DECAY_DAMPING_RATIO = 1.02;
const SETTLE = 0.002;

/** A low, uneven resting line while paused or quiet, so the bars never read as a flat dotted rule. */
const REST_TEXTURE = [0.7, 1, 0.55, 0.85, 0.65, 0.95, 0.5, 0.9, 0.6, 1, 0.55, 0.8, 0.7, 0.85];
const REST_LEVEL = 0.09;

function clamp01(v: number): number {
  return Number.isFinite(v) ? Math.max(0, Math.min(1, v)) : 0;
}

class Band {
  private floor = Number.NaN;
  private ceil = Number.NaN;

  reset(): void {
    this.floor = Number.NaN;
    this.ceil = Number.NaN;
  }

  /** 0–1 target for one band from its loudest bin (dB), tilted by its octave position. */
  update(db: number, tiltDb: number, dt: number): number {
    if (!Number.isFinite(db)) return 0;
    const gate = clamp01((db - GATE_FLOOR_DB) / GATE_SPAN_DB);
    if (gate <= 0) {
      if (!Number.isNaN(this.ceil)) this.ceil -= CEIL_RELEASE_DB_PER_S * dt;
      return 0;
    }
    const tilted = db + tiltDb;
    if (Number.isNaN(this.floor)) {
      this.floor = tilted - 8;
      this.ceil = tilted;
    } else {
      this.floor += (tilted - this.floor) * (1 - Math.exp(-dt / FLOOR_TAU_S)) * (0.25 + 0.75 * gate);
      this.ceil = Math.max(tilted, this.ceil - CEIL_RELEASE_DB_PER_S * dt);
    }
    this.ceil = Math.max(this.ceil, this.floor + MIN_RANGE_DB);
    const swing = clamp01((tilted - this.floor) / (this.ceil - this.floor));
    const presence = clamp01((tilted - PRESENCE_FLOOR_DB) / PRESENCE_SPAN_DB);
    return clamp01((PRESENCE_WEIGHT * presence + (1 - PRESENCE_WEIGHT) * swing) * gate);
  }
}

class Spring {
  value = 0;
  private velocity = 0;

  step(target: number, mass: number, dt: number): boolean {
    const rising = target > this.value;
    const k = rising ? ATTACK_STIFFNESS : DECAY_STIFFNESS;
    const zeta = rising ? ATTACK_DAMPING_RATIO : DECAY_DAMPING_RATIO;
    const c = 2 * Math.sqrt(k * mass) * zeta;
    // Two half steps keep the stiff attack stable at 30 Hz.
    for (let i = 0; i < 2; i++) {
      const h = dt / 2;
      const accel = (k * (target - this.value) - c * this.velocity) / mass;
      this.velocity += accel * h;
      this.value += this.velocity * h;
    }
    if (Math.abs(target - this.value) < SETTLE && Math.abs(this.velocity) < SETTLE * 10) {
      this.value = target;
      this.velocity = 0;
      return false;
    }
    return true;
  }
}

/**
 * Turns per-band dB levels into smoothly moving bar heights (0–1) with a resting texture. Allocates only when the
 * bar count changes.
 */
export class SpectrumShaper {
  private bands: Band[] = [];
  private springs: Spring[] = [];
  private tilts: number[] = [];
  private masses: number[] = [];
  private reach: number[] = [];
  private out: number[] = [];

  private resize(count: number): void {
    if (this.springs.length === count) return;
    const octaves = Math.log2(15_000 / 55);
    this.bands = Array.from({ length: count }, () => new Band());
    this.springs = Array.from({ length: count }, () => new Spring());
    this.tilts = Array.from({ length: count }, (_, i) =>
      Math.min(MAX_TILT_DB, BASS_TRIM_DB + (i / Math.max(1, count - 1)) * octaves * TILT_DB_PER_OCTAVE),
    );
    this.masses = Array.from({ length: count }, (_, i) => 1.25 - 0.5 * (i / Math.max(1, count - 1)));
    this.reach = Array.from({ length: count }, (_, i) => {
      const x = Math.min(1, i / Math.max(1, count - 1) / BASS_REACH_SPAN);
      return BASS_REACH + (1 - BASS_REACH) * x;
    });
    this.out = new Array(count).fill(0);
  }

  /**
   * Advances by `dt` seconds toward the levels in `db` (one dB value per band, or `null` while paused / silent).
   * Returns the heights and whether anything is still moving.
   */
  step(db: ArrayLike<number> | null, count: number, dt: number): { levels: number[]; moving: boolean } {
    this.resize(count);
    let moving = false;
    for (let i = 0; i < count; i++) {
      const target = db ? this.bands[i].update(db[i], this.tilts[i], dt) * this.reach[i] : 0;
      if (!db) this.bands[i].reset();
      if (this.springs[i].step(target, this.masses[i], dt)) moving = true;
      const rest = REST_LEVEL * REST_TEXTURE[i % REST_TEXTURE.length];
      this.out[i] = Math.max(rest, clamp01(this.springs[i].value));
    }
    return { levels: this.out, moving };
  }
}
