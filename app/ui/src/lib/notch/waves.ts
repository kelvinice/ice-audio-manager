// Wave shapes for the experimental notch effects (owner, 2026-11-26): the wavy landing ripple and the wavy progress
// line. Pure SVG path maths; the components only move them with `transform` / `opacity` (compositor-only).

const f = (n: number): string => n.toFixed(2);

export interface RippleRing {
  /** Closed (full ellipse) or open (half ellipse against the docked edge) SVG path. */
  d: string;
  /** Centre the ring grows from, in canvas px (used as `transform-origin`). */
  cx: number;
  cy: number;
}

/**
 * A wavy ellipse around `(cx, cy)`: the radius wobbles `amp` px, `waves` times around a full turn, starting at `phase`
 * (radians). `from`/`to` (radians, y pointing down) pick the arc: `0 → π` is the lower half (top-docked Edge Nook),
 * `π → 2π` the upper half (bottom-docked), `0 → 2π` a closed ring (Floating).
 */
export function wavyEllipse(
  cx: number,
  cy: number,
  rx: number,
  ry: number,
  opts: { waves: number; amp: number; phase?: number; from?: number; to?: number; steps?: number },
): RippleRing {
  const from = opts.from ?? 0;
  const to = opts.to ?? Math.PI * 2;
  const steps = Math.max(8, opts.steps ?? 96);
  const closed = Math.abs(to - from - Math.PI * 2) < 1e-6;
  const parts: string[] = [];
  for (let i = 0; i <= steps; i += 1) {
    const t = from + ((to - from) * i) / steps;
    const wobble = opts.amp * Math.sin(opts.waves * t + (opts.phase ?? 0));
    const x = cx + (rx + wobble) * Math.cos(t);
    const y = cy + (ry + wobble) * Math.sin(t);
    parts.push(`${i === 0 ? 'M' : 'L'} ${f(x)} ${f(y)}`);
  }
  return { d: parts.join(' ') + (closed ? ' Z' : ''), cx, cy };
}

/**
 * Wave sizes for the progress line, per scrubber size: gentle enough to watch, never busy. `stroke` equals the flat
 * rail's thickness (`MediaScrubber`) so the wave runs straight into the unplayed line. `speed` is px/s.
 */
export const PROGRESS_WAVE = Object.freeze({
  regular: { wavelength: 16, amp: 2, stroke: 4, speed: 12 },
  large: { wavelength: 20, amp: 2.6, stroke: 5, speed: 12 },
});

const smoothstep = (t: number): number => {
  const x = Math.max(0, Math.min(1, t));
  return x * x * (3 - 2 * x);
};

/**
 * The played part of the wavy progress line (owner, 2026-11-26: round ends, nothing clipped): a sine from `x0` to
 * `x1` whose height tapers to zero over about one wavelength at both ends, so the line starts and stops exactly on
 * `midY` and a round line cap finishes it (and meets the flat unplayed line seamlessly). `phase` (px) slides the
 * crests under the fixed ends; `ampScale` (0–1) flattens it (paused).
 */
export function taperedWavePath(
  x0: number,
  x1: number,
  midY: number,
  opts: { wavelength: number; amp: number; phase: number; ampScale?: number; step?: number },
): string {
  const len = Math.max(0, x1 - x0);
  const step = Math.max(0.5, opts.step ?? 1.5);
  const taper = Math.max(1, Math.min(opts.wavelength * 0.85, len / 2));
  const k = (Math.PI * 2) / Math.max(2, opts.wavelength);
  // A short played part (the song just started) grows its wave in over ~1.5 wavelengths instead of one sharp bump.
  const amp = opts.amp * Math.max(0, Math.min(1, opts.ampScale ?? 1)) * smoothstep(len / (opts.wavelength * 1.5));
  const n = Math.max(1, Math.ceil(len / step));
  const parts: string[] = [];
  for (let i = 0; i <= n; i += 1) {
    const x = x0 + (len * i) / n;
    const env = smoothstep((x - x0) / taper) * smoothstep((x1 - x) / taper);
    const y = midY - amp * env * Math.sin(k * (x - x0 - opts.phase));
    parts.push(`${i === 0 ? 'M' : 'L'} ${f(x)} ${f(y)}`);
  }
  return parts.join(' ');
}
