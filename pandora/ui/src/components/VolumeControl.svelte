<script lang="ts">
  import { audioEngine, playerStore } from "../lib/audio";
  import Icon from "./Icon.svelte";

  /**
   * Player volume: a mute button whose icon follows the level, and a slider built like the seek bar (thin rail that
   * thickens on hover, thumb on hover, the level as a tooltip). The wheel anywhere over it steps 5 %, arrows 2 %.
   */
  const WHEEL_STEP = 0.05;
  const KEY_STEP = 0.02;

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragging = $state(false);

  let level = $derived($playerStore.muted ? 0 : $playerStore.volume);
  let pct = $derived(Math.round(level * 100));
  let icon = $derived<"volume-x" | "volume-1" | "volume-2">(level === 0 ? "volume-x" : level < 0.5 ? "volume-1" : "volume-2");

  function set(vol: number) {
    audioEngine.setVolume(Math.round(Math.max(0, Math.min(1, vol)) * 100) / 100);
  }

  function at(clientX: number): number {
    if (!trackEl) return level;
    const rect = trackEl.getBoundingClientRect();
    return (clientX - rect.left) / rect.width;
  }

  function onKey(e: KeyboardEvent) {
    const delta: Record<string, number> = { ArrowRight: KEY_STEP, ArrowUp: KEY_STEP, ArrowLeft: -KEY_STEP, ArrowDown: -KEY_STEP };
    if (e.key in delta) set(level + delta[e.key]);
    else if (e.key === "Home") set(0);
    else if (e.key === "End") set(1);
    else return;
    e.preventDefault();
  }
</script>

<div
  class="volume"
  role="group"
  aria-label="Volume"
  onwheel={(e) => {
    e.preventDefault();
    set(level + (e.deltaY < 0 ? WHEEL_STEP : -WHEEL_STEP));
  }}
>
  <button
    class="btn-icon sm"
    title={$playerStore.muted ? "Unmute" : "Mute"}
    aria-label={$playerStore.muted ? "Unmute" : "Mute"}
    onclick={() => audioEngine.toggleMute()}
  >
    <Icon name={icon} size={18} />
  </button>
  <div
    class="track"
    class:dragging
    bind:this={trackEl}
    role="slider"
    tabindex="0"
    aria-label="Volume level"
    aria-valuemin={0}
    aria-valuemax={100}
    aria-valuenow={pct}
    aria-valuetext={`${pct}%`}
    onpointerdown={(e) => {
      if (e.button !== 0 || !trackEl) return;
      trackEl.setPointerCapture(e.pointerId);
      dragging = true;
      set(at(e.clientX));
    }}
    onpointermove={(e) => dragging && set(at(e.clientX))}
    onpointerup={() => (dragging = false)}
    onpointercancel={() => (dragging = false)}
    onkeydown={onKey}
  >
    <div class="rail"><div class="fill" style:transform={`scaleX(${level})`}></div></div>
    <span class="thumb" style:left={`${pct}%`}></span>
    <span class="tip" style:left={`${pct}%`}>{pct}%</span>
  </div>
</div>

<style>
  .volume {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .track {
    position: relative;
    width: 96px;
    height: 20px;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
    outline: none;
  }

  .track:focus-visible {
    outline: 2px solid var(--p-accent);
    outline-offset: 3px;
    border-radius: 4px;
  }

  .rail {
    width: 100%;
    height: 4px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--p-border-strong);
    transition: height 0.12s ease;
  }

  .track:hover .rail,
  .track.dragging .rail,
  .track:focus-visible .rail {
    height: 6px;
  }

  .fill {
    width: 100%;
    height: 100%;
    border-radius: 999px;
    transform-origin: left center;
    background: var(--p-text);
  }

  .track:hover .fill,
  .track.dragging .fill {
    background: var(--p-accent);
  }

  .thumb {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--p-text);
    box-shadow: 0 1px 6px rgb(0 0 0 / 0.4);
    transform: translate(-50%, -50%) scale(0);
    transition: transform 0.12s ease;
    pointer-events: none;
  }

  .track:hover .thumb,
  .track.dragging .thumb,
  .track:focus-visible .thumb {
    transform: translate(-50%, -50%) scale(1);
  }

  /* The level above the thumb while adjusting. */
  .tip {
    position: absolute;
    bottom: calc(100% + 6px);
    padding: 3px 7px;
    border-radius: 6px;
    background: var(--p-surface-elevated);
    box-shadow: 0 4px 14px rgb(0 0 0 / 0.35);
    font-size: 11px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    transform: translateX(-50%);
    opacity: 0;
    transition: opacity 0.12s ease;
    pointer-events: none;
  }

  .track.dragging .tip,
  .track:focus-visible .tip {
    opacity: 1;
  }
</style>
