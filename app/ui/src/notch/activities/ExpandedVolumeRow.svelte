<script lang="ts">
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    volumePercent: number;
    muted: boolean;
    /** Spring-smoothed percent for the fill (follows wheel and keyboard changes). */
    barPercent: number;
    onToggleMute: () => void;
    onSetVolume: (percent: number) => void;
  }

  let { volumePercent, muted, barPercent, onToggleMute, onSetVolume }: Props = $props();

  const SEND_INTERVAL_MS = 40;

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragPercent = $state<number | null>(null);
  let lastSentAt = 0;
  let lastSent = -1;

  const shownPercent = $derived(dragPercent ?? barPercent);
  const fillScale = $derived(Math.max(0, Math.min(1, shownPercent / 100)));
  const label = $derived(dragPercent !== null ? `${Math.round(dragPercent)}%` : muted ? 'Muted' : `${volumePercent}%`);

  function percentAt(clientX: number): number {
    if (!trackEl) return volumePercent;
    const rect = trackEl.getBoundingClientRect();
    if (rect.width <= 0) return volumePercent;
    return Math.max(0, Math.min(100, ((clientX - rect.left) / rect.width) * 100));
  }

  function send(percent: number, force: boolean): void {
    const rounded = Math.round(percent);
    const now = performance.now();
    if (rounded === lastSent) return;
    if (!force && now - lastSentAt < SEND_INTERVAL_MS) return;
    lastSent = rounded;
    lastSentAt = now;
    onSetVolume(rounded);
  }

  function onPointerDown(e: PointerEvent): void {
    if (e.button !== 0) return;
    trackEl?.setPointerCapture(e.pointerId);
    lastSent = -1;
    dragPercent = percentAt(e.clientX);
    send(dragPercent, true);
  }

  function onPointerMove(e: PointerEvent): void {
    if (dragPercent === null) return;
    dragPercent = percentAt(e.clientX);
    send(dragPercent, false);
  }

  function onPointerUp(e: PointerEvent): void {
    if (dragPercent === null) return;
    send(percentAt(e.clientX), true);
    dragPercent = null;
  }

  function onKeyDown(e: KeyboardEvent): void {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowUp' ? 2 : e.key === 'ArrowLeft' || e.key === 'ArrowDown' ? -2 : 0;
    if (step === 0) return;
    e.preventDefault();
    onSetVolume(Math.max(0, Math.min(100, volumePercent + step)));
  }
</script>

<div class="vol-row">
  <button
    class="mute-btn"
    class:muted
    type="button"
    title={muted ? 'Unmute' : 'Mute'}
    aria-label={muted ? 'Unmute audio' : 'Mute audio'}
    onclick={onToggleMute}
  >
    <DeviceIcon kind={muted ? 'speaker-muted' : 'speaker'} size={13} />
  </button>
  <div
    class="vol-track"
    class:dragging={dragPercent !== null}
    bind:this={trackEl}
    role="slider"
    tabindex="0"
    aria-label="Volume"
    aria-valuemin={0}
    aria-valuemax={100}
    aria-valuenow={Math.round(shownPercent)}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
    onkeydown={onKeyDown}
  >
    <div class="vol-rail">
      <div class="vol-fill" class:muted style:transform={`scaleX(${fillScale.toFixed(4)})`}></div>
    </div>
    <div class="vol-thumb" style:left={`${(fillScale * 100).toFixed(2)}%`}></div>
  </div>
  <span class="pct" class:muted={muted && dragPercent === null}>{label}</span>
</div>

<style>
  .vol-row {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 22px;
  }

  .mute-btn {
    appearance: none;
    width: 22px;
    height: 22px;
    padding: 0;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--ice-radius-pill);
    background: transparent;
    color: var(--ice-fg-muted);
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .mute-btn:hover {
    background: var(--ice-control-bg-hover);
    color: var(--ice-fg);
  }

  .mute-btn.muted {
    color: var(--ice-danger);
  }

  /* The hit area is taller than the visible rail so the bar is easy to grab. */
  .vol-track {
    position: relative;
    flex: 1;
    height: 22px;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
    outline: none;
  }

  .vol-rail {
    width: 100%;
    height: 5px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
    transition: height 120ms ease;
  }

  .vol-track:hover .vol-rail,
  .vol-track.dragging .vol-rail,
  .vol-track:focus-visible .vol-rail {
    height: 7px;
  }

  .vol-fill {
    width: 100%;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    transform-origin: left center;
    background: linear-gradient(90deg, var(--ice-accent), var(--ice-accent-strong));
  }

  .vol-fill.muted {
    background: var(--ice-fg-muted);
    opacity: 0.55;
  }

  .vol-thumb {
    position: absolute;
    top: 50%;
    width: 11px;
    height: 11px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-play-bg);
    transform: translate(-50%, -50%) scale(0);
    transition: transform 120ms ease;
    pointer-events: none;
  }

  .vol-track:hover .vol-thumb,
  .vol-track.dragging .vol-thumb,
  .vol-track:focus-visible .vol-thumb {
    transform: translate(-50%, -50%) scale(1);
  }

  .pct {
    min-width: 34px;
    text-align: right;
    font-family: var(--ice-mono);
    font-size: 10.5px;
    color: var(--ice-fg-muted);
    flex-shrink: 0;
  }

  .pct.muted {
    color: var(--ice-danger);
  }
</style>
