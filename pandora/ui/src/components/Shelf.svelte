<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  /** A titled home row whose content scrolls sideways, with previous / next buttons. */
  interface Props {
    title: string;
    strapline?: string | null;
    /** Width of one column (one tile, or one stack of song rows). */
    colWidth?: string;
    /** Items stacked per column (song rows use 4, cover tiles 1). */
    rows?: number;
    children: Snippet;
  }

  let { title, strapline = null, colWidth = "clamp(150px, 15vw, 190px)", rows = 1, children }: Props = $props();
  let track: HTMLDivElement | undefined = $state();
  let atStart = $state(true);
  let atEnd = $state(false);

  function update() {
    if (!track) return;
    atStart = track.scrollLeft <= 4;
    atEnd = track.scrollLeft + track.clientWidth >= track.scrollWidth - 4;
  }

  function page(direction: 1 | -1) {
    track?.scrollBy({ left: direction * track.clientWidth * 0.9, behavior: "smooth" });
  }

  $effect(() => {
    if (!track) return;
    update();
    const observer = new ResizeObserver(update);
    observer.observe(track);
    return () => observer.disconnect();
  });
</script>

<section class="shelf">
  <div class="head">
    <div class="titles">
      {#if strapline}<span class="strap">{strapline}</span>{/if}
      <h2>{title}</h2>
    </div>
    {#if !(atStart && atEnd)}
      <div class="nav">
        <button class="btn-icon sm" aria-label="Scroll back" disabled={atStart} onclick={() => page(-1)}>
          <Icon name="chevron-left" size={16} />
        </button>
        <button class="btn-icon sm" aria-label="Scroll forward" disabled={atEnd} onclick={() => page(1)}>
          <Icon name="chevron-right" size={16} />
        </button>
      </div>
    {/if}
  </div>
  <div class="track" style:--col={colWidth} style:--rows={rows} bind:this={track} onscroll={update}>
    {@render children()}
  </div>
</section>

<style>
  .shelf {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }

  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px;
    padding: 0 10px;
  }

  .titles {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .strap {
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--p-text-dim);
  }

  h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 750;
    letter-spacing: -0.02em;
  }

  .nav {
    display: flex;
    gap: 6px;
  }

  .track {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: var(--col);
    grid-template-rows: repeat(var(--rows), auto);
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    scrollbar-width: none;
  }

  .track::-webkit-scrollbar {
    display: none;
  }

  .track > :global(*) {
    scroll-snap-align: start;
  }
</style>
