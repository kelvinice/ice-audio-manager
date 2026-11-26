<script lang="ts">
  // Experimental "Cover tilt" (owner, 2026-11-26, `appearance.experiments.coverTilt`): the open card's cover leans toward
  // the pointer like a held card, with a soft glare that follows it, and eases back flat when the pointer leaves.
  // Pointer events only (no loop); `transform` + `opacity` only; off with `instant` (reduce motion / preset off).
  import type { Snippet } from 'svelte';

  interface Props {
    enabled: boolean;
    instant?: boolean;
    children: Snippet;
  }

  let { enabled, instant = false, children }: Props = $props();

  /** Largest lean at the cover's edge, in degrees. */
  const MAX_DEG = 9;

  let rx = $state(0);
  let ry = $state(0);
  let gx = $state(0);
  let gy = $state(0);
  let active = $state(false);
  const on = $derived(enabled && !instant);

  function onMove(e: PointerEvent): void {
    // The outer box is never transformed, so its rect is the cover's resting place.
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return;
    const px = Math.max(-0.5, Math.min(0.5, (e.clientX - r.left) / r.width - 0.5));
    const py = Math.max(-0.5, Math.min(0.5, (e.clientY - r.top) / r.height - 0.5));
    ry = px * 2 * MAX_DEG;
    rx = -py * 2 * MAX_DEG;
    gx = px * r.width;
    gy = py * r.height;
    active = true;
  }

  function onLeave(): void {
    active = false;
    rx = 0;
    ry = 0;
  }
</script>

{#if on}
  <div class="tilt" onpointermove={onMove} onpointerleave={onLeave} role="presentation">
    <div
      class="tilt-body"
      class:active
      style:transform={`perspective(260px) rotateX(${rx.toFixed(2)}deg) rotateY(${ry.toFixed(2)}deg)`}
    >
      {@render children()}
      <div class="glare-clip" aria-hidden="true">
        <div class="glare" style:transform={`translate(${gx.toFixed(1)}px, ${gy.toFixed(1)}px)`} class:shown={active}></div>
      </div>
    </div>
  </div>
{:else}
  {@render children()}
{/if}

<style>
  .tilt {
    display: inline-flex;
    flex-shrink: 0;
  }

  .tilt-body {
    position: relative;
    display: inline-flex;
    transition: transform 380ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .tilt-body.active {
    transition-duration: 90ms;
  }

  .glare-clip {
    position: absolute;
    inset: 0;
    border-radius: var(--ice-radius-md);
    overflow: hidden;
    pointer-events: none;
  }

  /* A soft light spot centred on the pointer: twice the cover's size so its edge never shows. */
  .glare {
    position: absolute;
    inset: -50%;
    background: radial-gradient(
      circle at center,
      color-mix(in srgb, var(--ice-fg) 26%, transparent) 0%,
      transparent 45%
    );
    opacity: 0;
    transition: opacity 260ms ease;
  }

  .glare.shown {
    opacity: 1;
  }
</style>
