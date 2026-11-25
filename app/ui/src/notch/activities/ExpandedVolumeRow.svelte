<script lang="ts">
  // D-28 (owner, 2026-11-25): the card's volume control. It belongs to the output, so it looks unlike the song's seek
  // bar (B1: neutral fill, speaker icon, %, thinner) and never jumps on a click (B2: drag is relative, a click steps).
  import { VOLUME_CLICK_STEP, VOLUME_DRAG_THRESHOLD_PX, volumeClickPercent, volumeDragPercent } from '../../lib/notch/volume';
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    volumePercent: number;
    muted: boolean;
    /** Spring-smoothed percent for the fill (follows wheel and keyboard changes). */
    barPercent: number;
    /** `inline`: a short rail beside the device chips; `wide`: the whole strip (opened from the volume chip). */
    variant?: 'inline' | 'wide';
    onToggleMute: () => void;
    onSetVolume: (percent: number) => void;
  }

  let { volumePercent, muted, barPercent, variant = 'inline', onToggleMute, onSetVolume }: Props = $props();

  const SEND_INTERVAL_MS = 40;

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragPercent = $state<number | null>(null);
  let press: { x: number; start: number; dragging: boolean } | null = null;
  let lastSentAt = 0;
  let lastSent = -1;

  const shownPercent = $derived(dragPercent ?? barPercent);
  const fillScale = $derived(Math.max(0, Math.min(1, shownPercent / 100)));
  const label = $derived(dragPercent !== null ? `${Math.round(dragPercent)}%` : muted ? 'Muted' : `${volumePercent}%`);

  function railWidth(): number {
    return trackEl?.getBoundingClientRect().width ?? 0;
  }

  function percentAt(clientX: number): number {
    const rect = trackEl?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return volumePercent;
    return Math.max(0, Math.min(100, ((clientX - rect.left) / rect.width) * 100));
  }

  function send(percent: number, force: boolean): void {
    const now = performance.now();
    if (percent === lastSent) return;
    if (!force && now - lastSentAt < SEND_INTERVAL_MS) return;
    lastSent = percent;
    lastSentAt = now;
    onSetVolume(percent);
  }

  function onPointerDown(e: PointerEvent): void {
    if (e.button !== 0) return;
    trackEl?.setPointerCapture(e.pointerId);
    press = { x: e.clientX, start: volumePercent, dragging: false };
    lastSent = -1;
  }

  function onPointerMove(e: PointerEvent): void {
    if (!press) return;
    const dx = e.clientX - press.x;
    if (!press.dragging && Math.abs(dx) < VOLUME_DRAG_THRESHOLD_PX) return;
    press.dragging = true;
    dragPercent = volumeDragPercent(press.start, dx, railWidth());
    send(dragPercent, false);
  }

  function onPointerUp(e: PointerEvent): void {
    if (!press) return;
    if (press.dragging) {
      send(volumeDragPercent(press.start, e.clientX - press.x, railWidth()), true);
    } else {
      onSetVolume(volumeClickPercent(volumePercent, percentAt(e.clientX)));
    }
    press = null;
    dragPercent = null;
  }

  function onKeyDown(e: KeyboardEvent): void {
    const up = e.key === 'ArrowRight' || e.key === 'ArrowUp';
    const down = e.key === 'ArrowLeft' || e.key === 'ArrowDown';
    if (!up && !down) return;
    e.preventDefault();
    onSetVolume(Math.max(0, Math.min(100, volumePercent + (up ? VOLUME_CLICK_STEP : -VOLUME_CLICK_STEP))));
  }
</script>

<div class="vol" class:wide={variant === 'wide'}>
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
    title="Volume: drag to change, click to step"
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
  .vol {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    flex-shrink: 0;
  }

  .vol.wide {
    flex: 1;
    min-width: 0;
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

  /* The hit area is taller than the visible rail so it's easy to grab. */
  .vol-track {
    position: relative;
    width: 84px;
    height: 22px;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
    outline: none;
  }

  .vol.wide .vol-track {
    flex: 1;
    width: auto;
  }

  /* B1: thinner than the seek bar and never the accent colour. */
  .vol-rail {
    width: 100%;
    height: 3px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
    transition: height 120ms ease;
  }

  .vol-track:hover .vol-rail,
  .vol-track.dragging .vol-rail,
  .vol-track:focus-visible .vol-rail {
    height: 5px;
  }

  .vol-fill {
    width: 100%;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    transform-origin: left center;
    background: var(--ice-fg-muted);
  }

  .vol-fill.muted {
    opacity: 0.45;
  }

  .vol-thumb {
    position: absolute;
    top: 50%;
    width: 11px;
    height: 11px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-fg);
    transform: translate(-50%, -50%) scale(0);
    transition: transform 120ms ease;
    pointer-events: none;
  }

  .vol-track:hover .vol-thumb,
  .vol-track.dragging .vol-thumb,
  .vol-track:focus-visible .vol-thumb {
    transform: translate(-50%, -50%) scale(1);
  }

  .vol-track:focus-visible {
    outline: 2px solid var(--ice-accent);
    outline-offset: 1px;
    border-radius: var(--ice-radius-sm);
  }

  .pct {
    min-width: 30px;
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
