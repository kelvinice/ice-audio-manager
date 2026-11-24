export function clampSeekPositionMs(ms: number, durationMs: number | null): number {
  if (durationMs === null || !Number.isFinite(durationMs) || durationMs <= 0) return 0;
  if (!Number.isFinite(ms)) return 0;
  return Math.round(Math.max(0, Math.min(durationMs, ms)));
}

export function pointerSeekMs(
  clientX: number,
  rectLeft: number,
  rectWidth: number,
  durationMs: number | null,
): number | null {
  if (durationMs === null || !Number.isFinite(durationMs) || durationMs <= 0 || rectWidth <= 0) {
    return null;
  }
  const ratio = Math.max(0, Math.min(1, (clientX - rectLeft) / rectWidth));
  return Math.round(ratio * durationMs);
}

export function stepSeekMs(
  currentMs: number,
  deltaMs: number,
  durationMs: number | null,
): number | null {
  if (durationMs === null || !Number.isFinite(durationMs) || durationMs <= 0) {
    return null;
  }
  return clampSeekPositionMs(currentMs + deltaMs, durationMs);
}

export function nextSecondTickDelayMs(elapsedMs: number): number {
  const safe = Number.isFinite(elapsedMs) && elapsedMs >= 0 ? elapsedMs : 0;
  const rem = 1000 - (safe % 1000);
  return Math.max(200, Math.min(1025, rem + 25));
}
