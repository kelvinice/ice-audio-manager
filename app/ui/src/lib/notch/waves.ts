// Wave shapes for the notch (owner, 2026-11-26): the landing ripple (Experimental) and the wavy progress line. Pure SVG
// path maths; the components only move them with `transform` / `opacity` (compositor-only), except the progress line's
// bounded loop (`WavyProgress.svelte`).

const f = (n: number): string => n.toFixed(2);

const smoothstep = (t: number): number => {
  const x = Math.max(0, Math.min(1, t));
  return x * x * (3 - 2 * x);
};

/** The pill a landing ripple leaves from: its body box in canvas px (Edge Nook ears excluded) and how it docks. */
export interface RipplePill {
  left: number;
  top: number;
  w: number;
  h: number;
  r: number;
  /** `none`: a floating capsule (closed ring). `top` / `bottom`: docked flush to that canvas edge (open ring). */
  dock: 'none' | 'top' | 'bottom';
}

export interface RippleRing {
  /** The pill's outline grown outward by the ring's offset, with a soft wobble along it (SVG path). */
  d: string;
  /** Point the ring grows from, in canvas px (`transform-origin`): the pill's centre, or the middle of its docked edge. */
  cx: number;
  cy: number;
  /** Start scale: shrinks `d` back onto the pill (`startOffset` px off its edge), so the ring rises off the outline. */
  sx0: number;
  sy0: number;
}

interface Seg {
  /** Point and outward normal at `t` (0–1) along the segment. */
  at(t: number): [number, number, number, number];
  len: number;
}

const line = (x0: number, y0: number, x1: number, y1: number, nx: number, ny: number): Seg => ({
  at: (t) => [x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, nx, ny],
  len: Math.hypot(x1 - x0, y1 - y0),
});

/** A convex corner: the outward normal is the radius direction. Angles in radians, y down. */
const arc = (cx: number, cy: number, r: number, a0: number, a1: number): Seg => ({
  at: (t) => {
    const a = a0 + (a1 - a0) * t;
    return [cx + r * Math.cos(a), cy + r * Math.sin(a), Math.cos(a), Math.sin(a)];
  },
  len: r * Math.abs(a1 - a0),
});

/**
 * The pill's outline pushed `d` px outward. Floating: a closed capsule, clockwise from the top edge. Docked: an open U
 * from the docked edge round the free side and back, built top-docked (the caller mirrors it for a bottom dock).
 */
function outlineSegments(p: RipplePill, d: number): Seg[] {
  const closed = p.dock === 'none';
  const W = p.w + 2 * d;
  const H = closed ? p.h + 2 * d : p.h + d;
  const T = closed ? p.top - d : 0;
  const R = Math.max(0, Math.min(p.r + d, W / 2, closed ? H / 2 : H));
  const [x0, x1, y1] = [p.left - d, p.left - d + W, T + H];
  const PI = Math.PI;
  if (!closed) {
    return [
      line(x0, T, x0, y1 - R, -1, 0),
      arc(x0 + R, y1 - R, R, PI, PI / 2),
      line(x0 + R, y1, x1 - R, y1, 0, 1),
      arc(x1 - R, y1 - R, R, PI / 2, 0),
      line(x1, y1 - R, x1, T, 1, 0),
    ];
  }
  return [
    line(x0 + R, T, x1 - R, T, 0, -1),
    arc(x1 - R, T + R, R, -PI / 2, 0),
    line(x1, T + R, x1, y1 - R, 1, 0),
    arc(x1 - R, y1 - R, R, 0, PI / 2),
    line(x1 - R, y1, x0 + R, y1, 0, 1),
    arc(x0 + R, y1 - R, R, PI / 2, PI),
    line(x0, y1 - R, x0, T + R, -1, 0),
    arc(x0 + R, T + R, R, PI, (PI * 3) / 2),
  ];
}

/**
 * One landing-ripple ring (owner, 2026-11-26: it should "come out of the outer side of the current pill shape"): the
 * pill's own outline grown `offset` px outward on every side, with a gentle wobble of `amp` px (about one crest per
 * `wavelength` px; a closed ring gets a whole number of crests so it meets itself). A docked ring is open and tapers its
 * wobble to nothing where it meets the edge. Scaling it by `sx0`/`sy0` around `(cx, cy)` puts it `startOffset` px off
 * the pill, so animating the scale to 1 moves it out from the pill's edge by the same distance on every side.
 */
export function pillRippleRing(
  pill: RipplePill,
  offset: number,
  opts: { amp: number; wavelength: number; canvasH: number; phase?: number; startOffset?: number; stepPx?: number },
): RippleRing {
  const segs = outlineSegments(pill, offset);
  const total = segs.reduce((sum, s) => sum + s.len, 0);
  const closed = pill.dock === 'none';
  const crests = closed ? Math.max(3, Math.round(total / opts.wavelength)) : total / opts.wavelength;
  const taper = Math.max(1, opts.wavelength * 0.75);
  const steps = Math.max(24, Math.ceil(total / Math.max(1, opts.stepPx ?? 3)));
  const flipY = (y: number): number => (pill.dock === 'bottom' ? opts.canvasH - y : y);
  const parts: string[] = [];
  let segIdx = 0;
  let segStart = 0;
  // A closed ring stops one step short and lets `Z` join it; an open one runs to the far edge.
  const last = closed ? steps - 1 : steps;
  for (let i = 0; i <= last; i += 1) {
    const s = (total * i) / steps;
    while (segIdx < segs.length - 1 && s > segStart + segs[segIdx].len) {
      segStart += segs[segIdx].len;
      segIdx += 1;
    }
    const seg = segs[segIdx];
    const [x, y, nx, ny] = seg.at(seg.len > 0 ? Math.min(1, (s - segStart) / seg.len) : 0);
    const env = closed ? 1 : smoothstep(s / taper) * smoothstep((total - s) / taper);
    const wob = opts.amp * env * Math.sin((Math.PI * 2 * crests * s) / total + (opts.phase ?? 0));
    parts.push(`${i === 0 ? 'M' : 'L'} ${f(x + nx * wob)} ${f(flipY(y + ny * wob))}`);
  }
  const start = Math.max(0, Math.min(offset, opts.startOffset ?? 1.5));
  const sx0 = (pill.w + 2 * start) / (pill.w + 2 * offset);
  const cx = pill.left + pill.w / 2;
  if (closed) {
    const sy0 = (pill.h + 2 * start) / (pill.h + 2 * offset);
    return { d: `${parts.join(' ')} Z`, cx, cy: pill.top + pill.h / 2, sx0, sy0 };
  }
  return { d: parts.join(' '), cx, cy: flipY(0), sx0, sy0: (pill.h + start) / (pill.h + offset) };
}

/**
 * Wave sizes for the progress line, per scrubber size: gentle enough to watch, never busy. `stroke` equals the flat
 * rail's thickness (`MediaScrubber`) so the wave runs straight into the unplayed line. `speed` is px/s.
 */
export const PROGRESS_WAVE = Object.freeze({
  regular: { wavelength: 16, amp: 2, stroke: 4, speed: 12 },
  large: { wavelength: 20, amp: 2.6, stroke: 5, speed: 12 },
});

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
