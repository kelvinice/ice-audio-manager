export type SpringPresetId = 'snappy' | 'smooth' | 'bouncy' | 'off';

export interface SpringParams {
  stiffness: number;
  damping: number;
  mass: number;
  instant?: boolean;
  precision?: number;
}

export const SPRING_PRESETS: Record<SpringPresetId, SpringParams> = {
  // Fast snap with minimal (~1.1%) overshoot.
  snappy: { stiffness: 420, damping: 34, mass: 1.0, precision: 0.05 },
  // Near-critically damped default (~0.1% overshoot).
  smooth: { stiffness: 280, damping: 31, mass: 1.0, precision: 0.05 },
  // Tuned to ~8.0% analytical step overshoot (D-13 / Codex analytical check).
  bouncy: { stiffness: 300, damping: 21.7, mass: 1.0, precision: 0.05 },
  // Reduced-motion / "Off" mode: instant update, zero animation frames (D-13).
  off: { stiffness: 1, damping: 1, mass: 1.0, instant: true, precision: 0.05 },
};

/**
 * Exact closed-form analytical damped harmonic oscillator (`stiffness`, `damping`, `mass`).
 *
 * Frame-rate independent: stepping by N * (1/144) or M * (1/75) over identical elapsed
 * time produces identical position & velocity (no Euler/RK4 drift across monitor refresh rates).
 */
export class Spring1D {
  value: number;
  velocity = 0;
  target: number;
  private params: SpringParams;

  constructor(initial: number, params: SpringParams = SPRING_PRESETS.smooth) {
    this.value = initial;
    this.target = initial;
    this.params = { ...params };
  }

  setParams(params: SpringParams): void {
    this.params = { ...params };
    if (this.params.instant) {
      this.value = this.target;
      this.velocity = 0;
    }
  }

  setTarget(target: number): void {
    this.target = target;
    if (this.params.instant) {
      this.value = target;
      this.velocity = 0;
    }
  }

  snap(value: number): void {
    this.value = value;
    this.target = value;
    this.velocity = 0;
  }

  isSettled(): boolean {
    if (this.params.instant) return true;
    const eps = this.params.precision ?? 0.05;
    return Math.abs(this.target - this.value) < eps && Math.abs(this.velocity) < eps;
  }

  /**
   * Advances the spring by `dtSec` seconds. Returns `true` if still animating.
   */
  step(dtSec: number): boolean {
    if (this.params.instant) {
      this.value = this.target;
      this.velocity = 0;
      return false;
    }
    if (dtSec <= 0) {
      return !this.isSettled();
    }
    // Clamp absurd dt spikes (e.g., first frame after window wake/sleep) to keep motion sane.
    const dt = Math.min(dtSec, 0.1);
    const k = Math.max(this.params.stiffness, 1e-4);
    const c = Math.max(this.params.damping, 0);
    const m = Math.max(this.params.mass, 1e-4);

    const x0 = this.value - this.target;
    const v0 = this.velocity;
    const w0 = Math.sqrt(k / m);
    const zeta = c / (2 * Math.sqrt(k * m));

    let x: number;
    let v: number;

    if (zeta < 1 - 1e-6) {
      // Underdamped
      const wd = w0 * Math.sqrt(1 - zeta * zeta);
      const decay = Math.exp(-zeta * w0 * dt);
      const a = x0;
      const b = (v0 + zeta * w0 * x0) / wd;
      const cos = Math.cos(wd * dt);
      const sin = Math.sin(wd * dt);
      x = decay * (a * cos + b * sin);
      v = decay * ((-a * zeta * w0 + b * wd) * cos - (b * zeta * w0 + a * wd) * sin);
    } else if (zeta > 1 + 1e-6) {
      // Overdamped
      const sq = Math.sqrt(zeta * zeta - 1);
      const r1 = -w0 * (zeta - sq);
      const r2 = -w0 * (zeta + sq);
      const c2 = (v0 - r1 * x0) / (r2 - r1);
      const c1 = x0 - c2;
      const e1 = Math.exp(r1 * dt);
      const e2 = Math.exp(r2 * dt);
      x = c1 * e1 + c2 * e2;
      v = c1 * r1 * e1 + c2 * r2 * e2;
    } else {
      // Critically damped
      const decay = Math.exp(-w0 * dt);
      const a = x0;
      const b = v0 + w0 * x0;
      x = decay * (a + b * dt);
      v = decay * (b - w0 * (a + b * dt));
    }

    this.value = this.target + x;
    this.velocity = v;

    if (this.isSettled()) {
      this.value = this.target;
      this.velocity = 0;
      return false;
    }
    return true;
  }
}
