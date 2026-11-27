<script lang="ts">
  import Icon from "./Icon.svelte";

  /** Square cover with title and subtitle; opens on click, plays from the hover button when `onPlay` is set. */
  interface Props {
    title: string;
    subtitle: string;
    covers: string[];
    round?: boolean;
    onOpen: () => void;
    onPlay?: () => void;
  }

  let { title, subtitle, covers, round = false, onOpen, onPlay }: Props = $props();
</script>

<div
  class="tile"
  role="button"
  tabindex="0"
  onclick={onOpen}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onOpen();
    }
  }}
>
  <div class="art" class:round class:mosaic={covers.length >= 4}>
    {#if covers.length >= 4}
      {#each covers.slice(0, 4) as src (src)}
        <img {src} alt="" loading="lazy" />
      {/each}
    {:else if covers.length > 0}
      <img src={covers[0]} alt="" loading="lazy" />
    {:else}
      <Icon name="list-music" size={34} />
    {/if}
    {#if onPlay}
      <button
        class="btn-play play"
        aria-label={`Play ${title}`}
        onclick={(e) => {
          e.stopPropagation();
          onPlay();
        }}
      >
        <Icon name="play" size={18} />
      </button>
    {/if}
  </div>
  <div class="title">{title}</div>
  {#if subtitle}<div class="sub">{subtitle}</div>{/if}
</div>

<style>
  .tile {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    padding: 10px 10px 12px;
    border-radius: 10px;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .tile:hover,
  .tile:focus-visible {
    background: var(--p-fill);
    outline: none;
  }

  .art {
    position: relative;
    aspect-ratio: 1;
    margin-bottom: 8px;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 8px;
    background: var(--p-fill);
    color: var(--p-text-dim);
    box-shadow: 0 8px 22px rgb(0 0 0 / 0.28);
  }

  .art.round {
    border-radius: 50%;
  }

  .art.mosaic {
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .play {
    position: absolute;
    right: 8px;
    bottom: 8px;
    width: 44px;
    height: 44px;
    box-shadow: 0 6px 16px rgb(0 0 0 / 0.35);
    opacity: 0;
    transform: translateY(6px);
    transition: opacity 0.16s ease, transform 0.16s ease, background 0.14s ease;
  }

  .tile:hover .play,
  .tile:focus-within .play {
    opacity: 1;
    transform: none;
  }

  .title {
    font-size: 14px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub {
    font-size: 12.5px;
    line-height: 1.35;
    color: var(--p-text-muted);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
