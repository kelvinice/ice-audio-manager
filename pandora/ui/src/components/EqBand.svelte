<script lang="ts">
  /**
   * One equalizer band: a vertical slider drawn in CSS (the native vertical range input forced a minimum width that
   * overflowed the drawer). Drag, click, wheel or arrow keys set the gain; double-click resets it to 0 dB.
   */
  interface Props {
    value: number;
    label: string;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    onChange: (db: number) => void;
  }

  let { value, label, min = -12, max = 12, step = 0.5, disabled = false, onChange }: Props = $props();

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragging = $state(false);

  /** Position on the rail, 0 at the bottom, 1 at the top. */
  let pos = $derived((value - min) / (max - min));
  let zero = $derived((0 - min) / (max - min));
  let shown = $derived(value > 0 ? `+${value}` : `${value}`);

  function clampSnap(db: number): number {
    return Math.max(min, Math.min(max, Math.round(db / step) * step));
  }

  function valueAt(clientY: number): number {
    if (!trackEl) return value;
    const rect = trackEl.getBoundingClientRect();
    return clampSnap(max - ((clientY - rect.top) / rect.height) * (max - min));
  }

  function onDown(e: PointerEvent) {
    if (disabled || e.button !== 0 || !trackEl) return;
    trackEl.setPointerCapture(e.pointerId);
    dragging = true;
    onChange(valueAt(e.clientY));
  }

  function onMove(e: PointerEvent) {
    if (dragging) onChange(valueAt(e.clientY));
  }

  function onKey(e: KeyboardEvent) {
    const steps: Record<string, number> = { ArrowUp: step, ArrowRight: step, ArrowDown: -step, ArrowLeft: -step, PageUp: 3, PageDown: -3 };
    let next: number | null = null;
    if (e.key in steps) next = value + steps[e.key];
    else if (e.key === "Home") next = max;
    else if (e.key === "End") next = min;
    else if (e.key === "0" || e.key === "Delete") next = 0;
    if (next === null) return;
    e.preventDefault();
    onChange(clampSnap(next));
  }
</script>

<div class="band" class:disabled>
  <span class="db" class:boost={value > 0} class:cut={value < 0}>{shown}</span>
  <div
    class="track"
    class:dragging
    bind:this={trackEl}
    role="slider"
    tabindex={disabled ? -1 : 0}
    aria-label={`${label} Hz`}
    aria-orientation="vertical"
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuenow={value}
    aria-valuetext={`${shown} dB`}
    aria-disabled={disabled}
    title="Double-click to reset"
    onpointerdown={onDown}
    onpointermove={onMove}
    onpointerup={() => (dragging = false)}
    onpointercancel={() => (dragging = false)}
    ondblclick={() => !disabled && onChange(0)}
    onwheel={(e) => {
      if (disabled) return;
      e.preventDefault();
      onChange(clampSnap(value + (e.deltaY < 0 ? step : -step)));
    }}
    onkeydown={onKey}
  >
    <span class="rail"></span>
    <span class="zero" style:bottom={`${zero * 100}%`}></span>
    <span
      class="fill"
      style:bottom={`${Math.min(pos, zero) * 100}%`}
      style:height={`${Math.abs(pos - zero) * 100}%`}
    ></span>
    <span class="thumb" style:bottom={`${pos * 100}%`}></span>
  </div>
  <span class="hz">{label}</span>
</div>

<style>
  .band {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .band.disabled {
    opacity: 0.45;
  }

  .db,
  .hz {
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    color: var(--p-text-dim);
  }

  .db.boost {
    color: var(--p-accent-bright);
  }

  .db.cut {
    color: var(--p-text-muted);
  }

  .track {
    position: relative;
    width: 24px;
    height: 150px;
    cursor: ns-resize;
    touch-action: none;
    outline: none;
  }

  .disabled .track {
    cursor: default;
  }

  .track:focus-visible {
    outline: 2px solid var(--p-accent);
    outline-offset: 2px;
    border-radius: 6px;
  }

  .rail,
  .fill,
  .zero {
    position: absolute;
    left: 50%;
    translate: -50% 0;
  }

  .rail {
    top: 0;
    bottom: 0;
    width: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--p-text) 12%, transparent);
  }

  .zero {
    width: 12px;
    height: 1px;
    background: var(--p-border-strong);
  }

  .fill {
    width: 4px;
    border-radius: 999px;
    background: var(--p-accent);
  }

  .thumb {
    position: absolute;
    left: 50%;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--p-text);
    box-shadow: 0 1px 6px rgb(0 0 0 / 0.4);
    translate: -50% 50%;
    transition: scale 0.12s ease;
  }

  .track:hover .thumb,
  .track.dragging .thumb {
    scale: 1.15;
  }
</style>
