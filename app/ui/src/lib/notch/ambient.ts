/**
 * Ambient lava-lamp backdrop helpers (D-25 refresh, owner 2026-11-25). Pure; no DOM, no timers.
 *
 * - `ambientTempoRate` narrows the beat tempo (~0.58–1.65×) so a fast song makes the lamp livelier, never racing.
 * - `BreathEnvelope` turns low-band kick onsets into a 0–1 "breath": instant swell on a kick, slow exhale after.
 */

/** Slowest / fastest lamp speed relative to the theme's `--ice-ambient-period`. */
export const AMBIENT_TEMPO_MIN = 0.85;
export const AMBIENT_TEMPO_MAX = 1.2;
/** Share of the beat tempo's deviation from 1× that reaches the lamp. */
const TEMPO_DAMPING = 0.35;

export function ambientTempoRate(beatTempo: number): number {
  if (!Number.isFinite(beatTempo)) return 1;
  const calm = 1 + (beatTempo - 1) * TEMPO_DAMPING;
  return Math.min(AMBIENT_TEMPO_MAX, Math.max(AMBIENT_TEMPO_MIN, calm));
}

/** Kicks weaker than this don't move the lamp (hiss, hi-hats bleeding into the low bands). */
export const BREATH_MIN_KICK = 0.12;
/** Exhale time constant: a breath falls to ~37 % after this long. */
export const BREATH_RELEASE_S = 0.42;
/** Values are reported in these steps, so a slow exhale doesn't write a style every frame. */
export const BREATH_STEP = 0.02;
/** How much the whole wash swells at a full breath (1 = +100 %). Twin: `.orb-field` in NotchBackdrop.svelte. */
export const BREATH_SCALE = 0.06;

export class BreathEnvelope {
  private level = 0;
  private lastMs: number | null = null;

  /** Feeds one meter frame's kick (0–1); returns the breath, quantised to `BREATH_STEP`. */
  update(kick: number, nowMs: number): number {
    const dt = this.lastMs === null ? 0 : Math.max(0, (nowMs - this.lastMs) / 1000);
    this.lastMs = nowMs;
    const decayed = this.level * Math.exp(-dt / BREATH_RELEASE_S);
    const k = Number.isFinite(kick) && kick >= BREATH_MIN_KICK ? Math.min(1, kick) : 0;
    this.level = Math.max(decayed, k);
    if (this.level < BREATH_STEP / 2) this.level = 0;
    return Math.round(this.level / BREATH_STEP) * BREATH_STEP;
  }

  /** Silence, pause or a stopped stream: back to rest at once. */
  reset(): void {
    this.level = 0;
    this.lastMs = null;
  }
}
