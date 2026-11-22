<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { ArtInfo } from '../../lib/ipc/engine';
  import { formatRgbCss } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    art: ArtInfo | null | undefined;
    previousArt?: ArtInfo | null;
    crossfadeKey?: number;
    size?: number;
    instant?: boolean;
  }

  let {
    art,
    previousArt = null,
    crossfadeKey = 0,
    size = 32,
    instant = false,
  }: Props = $props();

  let loadedId = $state<string | null>(null);
  let fallbackAttemptedId = $state<string | null>(null);
  let failedId = $state<string | null>(null);

  const currentId = $derived(art?.id ?? null);
  const imgSrc = $derived.by(() => {
    if (!currentId || failedId === currentId) return null;
    if (fallbackAttemptedId === currentId) {
      try {
        return convertFileSrc(currentId, 'iceart');
      } catch {
        return `iceart://${currentId}`;
      }
    }
    return `iceart://${currentId}`;
  });
  const isImgLoaded = $derived(currentId !== null && loadedId === currentId && failedId !== currentId);
  const accentCss = $derived(formatRgbCss(art?.accent) ?? 'var(--ice-accent)');
  const prevAccentCss = $derived(formatRgbCss(previousArt?.accent) ?? 'var(--ice-accent)');
  const showOutgoing = $derived(!instant && crossfadeKey > 0 && previousArt !== null && previousArt?.id !== art?.id);

  function handleLoad(id: string | null) {
    if (id) {
      loadedId = id;
    }
  }

  function handleError(id: string | null) {
    if (!id) return;
    if (fallbackAttemptedId !== id) {
      fallbackAttemptedId = id;
      return;
    }
    failedId = id;
  }
</script>

<div
  class="art-slot"
  style:width={`${size}px`}
  style:height={`${size}px`}
  style:--tile-accent={accentCss}
  style:--prev-tile-accent={prevAccentCss}
  aria-hidden="true"
>
  {#key instant ? 0 : crossfadeKey}
    {#if showOutgoing}
      <div class="art-tile outgoing">
        <span class="glyph">
          <DeviceIcon kind="music" size={Math.max(10, Math.round(size * 0.44))} />
        </span>
      </div>
    {/if}

    <div class="art-tile incoming" class:animate-in={!instant && crossfadeKey > 0}>
      <span class="glyph" class:hidden={isImgLoaded}>
        <DeviceIcon kind="music" size={Math.max(10, Math.round(size * 0.44))} />
      </span>
      {#if imgSrc}
        <img
          class="art-img"
          class:loaded={isImgLoaded}
          src={imgSrc}
          alt=""
          draggable="false"
          onload={() => handleLoad(currentId)}
          onerror={() => handleError(currentId)}
        />
      {/if}
    </div>
  {/key}
</div>

<style>
  .art-slot {
    position: relative;
    flex-shrink: 0;
    border-radius: var(--ice-radius-sm);
    overflow: hidden;
  }

  .art-tile {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--ice-radius-sm);
    background: var(--tile-accent, var(--ice-accent));
    border: 1px solid var(--ice-border);
    color: var(--ice-bg);
    overflow: hidden;
  }

  .art-tile.outgoing {
    background: var(--prev-tile-accent, var(--ice-accent));
    animation: art-out 240ms cubic-bezier(0.4, 0, 1, 1) forwards;
    pointer-events: none;
  }

  .art-tile.incoming.animate-in {
    animation: art-in 260ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .glyph {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0.92;
  }

  .glyph.hidden {
    opacity: 0;
  }

  .art-img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
  }

  .art-img.loaded {
    opacity: 1;
  }

  @keyframes art-in {
    from {
      opacity: 0;
      transform: scale(0.88);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @keyframes art-out {
    from {
      opacity: 1;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(0.92);
    }
  }
</style>
