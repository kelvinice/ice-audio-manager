// Twin: app/ui/src/lib/notch/ambient.ts (BreathEnvelope: instant swell on a kick, slow exhale). Ice gets its kicks
// from the meter's beat follower; Pandora finds them here from the FFT's bass level rising above its recent average.

/** Kicks weaker than this don't move the lamp. */
const MIN_KICK = 0.12;
/** Exhale time constant: a breath falls to ~37 % after this long. */
const RELEASE_S = 0.42;
/** Values are reported in these steps, so a slow exhale doesn't write a style every frame. */
const STEP = 0.02;
/** How fast the bass "normal level" follows the music; a kick is a rise above it. */
const AVERAGE_TAU_S = 0.8;
/** dB above the average where a kick starts, and the rise that counts as a full kick. */
const KICK_THRESHOLD_DB = 2.5;
const KICK_RANGE_DB = 9;
/** How much the whole field swells at a full breath (1 = +100 %). */
export const BREATH_SCALE = 0.07;

export class BassBreath {
  private level = 0;
  private average = Number.NaN;

  /** Feeds the loudest bass band (dB) over `dt` seconds; returns the breath 0–1, quantised to `STEP`. */
  update(bassDb: number, dt: number): number {
    const decayed = this.level * Math.exp(-dt / RELEASE_S);
    let kick = 0;
    if (Number.isFinite(bassDb)) {
      if (Number.isNaN(this.average)) this.average = bassDb;
      kick = Math.min(1, Math.max(0, (bassDb - this.average - KICK_THRESHOLD_DB) / KICK_RANGE_DB));
      this.average += (bassDb - this.average) * (1 - Math.exp(-dt / AVERAGE_TAU_S));
    }
    this.level = Math.max(decayed, kick >= MIN_KICK ? kick : 0);
    if (this.level < STEP / 2) this.level = 0;
    return Math.round(this.level / STEP) * STEP;
  }

  /** Pause or a new song: back to rest at once. */
  reset(): void {
    this.level = 0;
    this.average = Number.NaN;
  }
}
