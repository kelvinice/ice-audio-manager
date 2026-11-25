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
 * A smooth sine line from `x = 0` to at least `width + wavelength`, centred on `midY`. Each half wave is one quadratic
 * curve (`T` mirrors the control point), so the line can slide left/right by exactly one `wavelength` and loop
 * seamlessly.
 */
export function sineStripPath(width: number, wavelength: number, amp: number, midY: number): string {
  const half = Math.max(2, wavelength / 2);
  const end = Math.max(0, width) + wavelength;
  let d = `M 0 ${f(midY)} Q ${f(half / 2)} ${f(midY - 2 * amp)} ${f(half)} ${f(midY)}`;
  for (let x = half * 2; x <= end + 1e-6; x += half) d += ` T ${f(x)} ${f(midY)}`;
  return d;
}

/** Wave sizes for the progress line, per scrubber size: gentle enough to watch, never busy. */
export const PROGRESS_WAVE = Object.freeze({
  regular: { wavelength: 16, amp: 2, stroke: 3 },
  large: { wavelength: 20, amp: 2.6, stroke: 4 },
});
