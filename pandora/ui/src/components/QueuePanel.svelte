<script lang="ts">
  import { audioEngine, playerStore } from "../lib/audio";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { Track } from "../lib/types";
  import Icon from "./Icon.svelte";

  /** Right-hand panel (toggled from the player bar): what plays now and next, and what played before. */
  interface Props {
    history: Track[];
    proxyPort: number;
    onClose: () => void;
  }

  let { history, proxyPort, onClose }: Props = $props();

  let tab = $state<"queue" | "recent">("queue");

  let current = $derived($playerStore.currentIndex >= 0 ? ($playerStore.queue[$playerStore.currentIndex] ?? null) : null);
  let upNext = $derived(
    $playerStore.queue
      .map((track, index) => ({ track, index }))
      .filter(({ index }) => index > $playerStore.currentIndex),
  );
</script>

{#snippet row(track: Track, isCurrent: boolean, onPlay: () => void, onRemove?: () => void)}
  {@const art = resolveArtworkUrl(track.artworkUrl, proxyPort)}
  <div class="row" class:current={isCurrent}>
    <button class="main" onclick={onPlay} aria-label={`Play ${track.title}`}>
      <span class="art">
        {#if art}<img src={art} alt="" loading="lazy" />{:else}<Icon name="music" size={16} />{/if}
        {#if isCurrent && $playerStore.playing}
          <span class="bars" aria-hidden="true"><i></i><i></i><i></i></span>
        {/if}
      </span>
      <span class="text">
        <span class="title">{track.title}</span>
        <span class="sub">
          {#if $playerStore.magicIds.includes(track.id)}<span class="magic" title="Added by Magic shuffle"
              ><Icon name="sparkles" size={11} /></span
            >{/if}{track.artist}
        </span>
      </span>
    </button>
    {#if onRemove}
      <button class="btn-icon sm remove" aria-label={`Remove ${track.title} from the queue`} title="Remove from queue" onclick={onRemove}>
        <Icon name="x" size={14} />
      </button>
    {/if}
  </div>
{/snippet}

<aside class="panel" aria-label="Queue">
  <header class="head">
    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === "queue"} class:active={tab === "queue"} onclick={() => (tab = "queue")}>Queue</button>
      <button role="tab" aria-selected={tab === "recent"} class:active={tab === "recent"} onclick={() => (tab = "recent")}>
        Recently played
      </button>
    </div>
    <button class="btn-icon sm" aria-label="Close queue" title="Close" onclick={onClose}>
      <Icon name="x" size={16} />
    </button>
  </header>

  <div class="scroll">
    {#if tab === "queue"}
      {#if current}
        <h3>Now playing</h3>
        {@render row(current, true, () => void audioEngine.togglePlayPause())}
      {/if}

      {#if upNext.length > 0}
        <h3>Next up</h3>
        {#each upNext as item (item.track.id + item.index)}
          {@render row(
            item.track,
            false,
            () => void audioEngine.playTrack(item.track, $playerStore.queue),
            () => audioEngine.removeFromQueue(item.index),
          )}
        {/each}
      {:else}
        <p class="empty">
          {current ? "Nothing queued after this song." : "Play something and the songs that follow show up here."}
        </p>
      {/if}
    {:else if history.length > 0}
      {#each history as track (track.id)}
        {@render row(track, track.id === current?.id, () => void audioEngine.playTrack(track, history))}
      {/each}
    {:else}
      <p class="empty">Songs you play show up here.</p>
    {/if}
  </div>
</aside>

<style>
  .panel {
    width: clamp(280px, 24vw, 360px);
    flex-shrink: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    margin: 0 8px 8px 0;
    border-radius: 10px;
    background: var(--p-bg);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 14px 12px 6px 18px;
  }

  .tabs {
    display: flex;
    gap: 18px;
  }

  .tabs button {
    padding: 6px 0;
    border-bottom: 2px solid transparent;
    font-size: 14px;
    font-weight: 700;
    color: var(--p-text-muted);
    transition: color 0.14s ease, border-color 0.14s ease;
  }

  .tabs button:hover {
    color: var(--p-text);
  }

  .tabs button.active {
    color: var(--p-text);
    border-bottom-color: var(--p-accent);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 6px 8px 16px;
  }

  h3 {
    margin: 16px 10px 8px;
    font-size: 15px;
    font-weight: 750;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 6px 6px 8px;
    border-radius: 8px;
  }

  .row:hover,
  .row:focus-within {
    background: var(--p-fill);
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
  }

  .art {
    position: relative;
    width: 48px;
    height: 48px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 6px;
    background: var(--p-fill);
    color: var(--p-text-dim);
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .title,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-size: 14px;
    font-weight: 600;
  }

  .row.current .title {
    color: var(--p-accent-bright);
  }

  .sub {
    font-size: 12.5px;
    color: var(--p-text-dim);
  }

  .magic {
    margin-right: 4px;
    color: var(--p-accent-bright);
  }

  .remove {
    opacity: 0;
  }

  .row:hover .remove,
  .row:focus-within .remove {
    opacity: 1;
  }

  .empty {
    margin: 16px 10px;
    font-size: 13px;
    color: var(--p-text-dim);
  }

  /* Playing indicator over the cover of the current song. */
  .bars {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 2px;
    padding-bottom: 14px;
    background: rgb(0 0 0 / 0.45);
  }

  .bars i {
    width: 3px;
    border-radius: 1px;
    background: #fff;
    animation: bar 0.8s ease-in-out infinite alternate;
  }

  .bars i:nth-child(2) {
    animation-delay: 0.2s;
  }

  .bars i:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes bar {
    from {
      height: 3px;
    }
    to {
      height: 16px;
    }
  }
</style>
