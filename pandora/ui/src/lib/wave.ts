// Twin: app/ui/src/lib/notch/waves.ts (PROGRESS_WAVE, taperedWavePath) — the Ice notch's wavy progress line.
// Pandora draws its seek bar the same way so both apps' progress lines look and move alike; keep shapes in step.

/** Wave shape per seek-bar size, in px (`speed` is how far the crests travel per second). */
export const SEEK_WAVE = Object.freeze({
  regular: { wavelength: 18, amp: 2.2, stroke: 4, speed: 14 },
  large: { wavelength: 22, amp: 2.8, stroke: 5, speed: 14 },
});

export type SeekSize = keyof typeof SEEK_WAVE;

function smoothstep(t: number): number {
  const x = Math.max(0, Math.min(1, t));
  return x * x * (3 - 2 * x);
}

/**
 * The played part of a wavy progress line: a sine from `x0` to `x1` whose height tapers to zero near both ends, so
 * it starts and stops on `midY` and a round cap meets the flat unplayed line cleanly. `phase` (px) slides the crests
 * under the fixed ends; `ampScale` (0–1) flattens it while paused. A short played part grows its wave in gradually.
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
  const amp = opts.amp * Math.max(0, Math.min(1, opts.ampScale ?? 1)) * smoothstep(len / (opts.wavelength * 1.5));
  const n = Math.max(1, Math.ceil(len / step));
  const parts: string[] = [];
  for (let i = 0; i <= n; i += 1) {
    const x = x0 + (len * i) / n;
    const env = smoothstep((x - x0) / taper) * smoothstep((x1 - x) / taper);
    const y = midY - amp * env * Math.sin(k * (x - x0 - opts.phase));
    parts.push(`${i === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`);
  }
  return parts.join(" ");
}

/** Seek position for a pointer at `clientX` over a track starting at `left` with `width`, clamped to the song. */
export function pointerSeekMs(clientX: number, left: number, width: number, durationMs: number): number {
  if (width <= 0 || durationMs <= 0) return 0;
  return Math.round(Math.max(0, Math.min(1, (clientX - left) / width)) * durationMs);
}

/** `m:ss`, or `h:mm:ss` from an hour. */
export function formatTimecode(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}
