<script lang="ts">
  import type { MonitorSummary } from '../../lib/ipc/types';

  interface Props {
    monitors: MonitorSummary[];
    /** Saved display path (`settings.pinnedMonitor`). */
    pinned: string | null;
    onpick: (displayPath: string) => void;
  }

  let { monitors, pinned, onpick }: Props = $props();

  const same = (a: string | null, b: string) => Boolean(a && a.toLowerCase() === b.toLowerCase());
  /** Where the notch is now: the pinned display, else the main one (placement falls back while it's unplugged). */
  const current = $derived(monitors.find((m) => same(pinned, m.displayPath)) ?? monitors.find((m) => m.isPrimary) ?? null);

  // Draw the displays as Windows arranges them (Settings → Display), scaled into the box.
  const box = $derived.by(() => {
    const left = Math.min(...monitors.map((m) => m.left));
    const top = Math.min(...monitors.map((m) => m.top));
    const right = Math.max(...monitors.map((m) => m.left + m.width));
    const bottom = Math.max(...monitors.map((m) => m.top + m.height));
    return { left, top, width: Math.max(1, right - left), height: Math.max(1, bottom - top) };
  });
</script>

<div class="map" role="radiogroup" aria-label="Display for the notch">
  <div class="stage" style:aspect-ratio={`${box.width} / ${box.height}`}>
    {#each monitors as m (m.displayPath)}
      {@const selected = current?.displayPath === m.displayPath}
      <button
        type="button"
        role="radio"
        aria-checked={selected}
        aria-label={`Display ${m.index + 1}, ${m.name}${m.isPrimary ? ', main display' : ''}`}
        class:selected
        style:left={`${((m.left - box.left) / box.width) * 100}%`}
        style:top={`${((m.top - box.top) / box.height) * 100}%`}
        style:width={`${(m.width / box.width) * 100}%`}
        style:height={`${(m.height / box.height) * 100}%`}
        onclick={() => onpick(m.displayPath)}
      >
        {#if selected}<span class="notch"></span>{/if}
        <span class="num">{m.index + 1}</span>
        <span class="name">{m.name}</span>
        <span class="spec">{m.width}×{m.height} · {m.refreshHz} Hz{m.isPrimary ? ' · Main' : ''}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .map {
    display: flex;
    justify-content: center;
    padding: 20px 16px;
  }

  .stage {
    position: relative;
    width: 100%;
    max-width: 560px;
    max-height: 190px;
  }

  button {
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: 6px;
    border: 3px solid transparent;
    background-clip: padding-box;
    border-radius: 10px;
    background-color: var(--ice-set-control);
    box-shadow: inset 0 0 0 1px var(--ice-set-control-border);
    cursor: pointer;
    overflow: hidden;
    transition: background-color 120ms var(--ice-set-ease);
  }

  button:hover {
    background-color: var(--ice-set-control-hover);
  }

  button.selected {
    background-color: var(--ice-set-accent-soft);
    box-shadow: inset 0 0 0 2px var(--ice-set-accent);
  }

  .notch {
    position: absolute;
    top: 6px;
    left: 50%;
    width: 30px;
    height: 8px;
    margin-left: -15px;
    border-radius: 4px;
    background: var(--ice-set-accent);
  }

  .num {
    font-size: 22px;
    line-height: 28px;
    font-weight: 600;
  }

  .name {
    font-size: 12px;
    line-height: 16px;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .spec {
    font-size: 11px;
    line-height: 14px;
    color: var(--ice-set-fg-muted);
    white-space: nowrap;
  }
</style>
