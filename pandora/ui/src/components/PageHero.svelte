<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  /** Header for a playlist / album page: cover (one image or a 2×2 mosaic), label, big title, meta line. */
  interface Props {
    covers: string[];
    label: string;
    title: string;
    meta: string;
    round?: boolean;
    /** Replaces the title (e.g. an inline rename form). */
    titleSlot?: Snippet;
    children?: Snippet;
  }

  let { covers, label, title, meta, round = false, titleSlot, children }: Props = $props();
</script>

<header class="hero">
  <div class="cover" class:round class:mosaic={covers.length >= 4}>
    {#if covers.length >= 4}
      {#each covers.slice(0, 4) as src (src)}
        <img {src} alt="" />
      {/each}
    {:else if covers.length > 0}
      <img src={covers[0]} alt="" />
    {:else}
      <Icon name="list-music" size={52} />
    {/if}
  </div>
  <div class="text">
    <span class="label">{label}</span>
    {#if titleSlot}
      {@render titleSlot()}
    {:else}
      <h1>{title}</h1>
    {/if}
    {#if meta}<p class="meta">{meta}</p>{/if}
  </div>
</header>

{#if children}
  <div class="actions">{@render children()}</div>
{/if}

<style>
  .hero {
    display: flex;
    align-items: flex-end;
    gap: 28px;
    padding: 36px 32px 28px;
    background: linear-gradient(180deg, var(--p-surface-tint), transparent);
  }

  .cover {
    width: clamp(150px, 18vw, 220px);
    aspect-ratio: 1;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 8px;
    background: var(--p-fill);
    color: var(--p-text-dim);
    box-shadow: 0 16px 40px rgb(0 0 0 / 0.35);
  }

  .cover.round {
    border-radius: 50%;
  }

  .cover.mosaic {
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
  }

  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .label {
    font-size: 12px;
    font-weight: 650;
    color: var(--p-text-muted);
  }

  h1 {
    margin: 0;
    font-size: clamp(30px, 4.6vw, 64px);
    font-weight: 800;
    line-height: 1.02;
    letter-spacing: -0.04em;
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .meta {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--p-text-muted);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 4px 32px 20px;
  }

  @media (max-width: 720px) {
    .hero {
      flex-direction: column;
      align-items: flex-start;
      padding: 24px 20px;
    }

    .actions {
      padding: 4px 20px 16px;
    }
  }
</style>
