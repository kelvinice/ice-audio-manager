<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { ArtInfo } from '../../lib/ipc/engine';

  interface Props {
    art: ArtInfo | null | undefined;
    hasArt: boolean;
    instant: boolean;
  }

  let { art, hasArt, instant }: Props = $props();

  let loadedId = $state<string | null>(null);
  let fallbackAttemptedId = $state<string | null>(null);
  let failedId = $state<string | null>(null);

  const artId = $derived(hasArt ? (art?.id ?? null) : null);
  const imgSrc = $derived.by(() => {
    if (!artId || failedId === artId) return null;
    if (fallbackAttemptedId === artId) {
      try {
        return convertFileSrc(artId, 'iceart');
      } catch {
        return `iceart://${artId}`;
      }
    }
    return `iceart://${artId}`;
  });
  const isImgReady = $derived(artId !== null && loadedId === artId && failedId !== artId);

  function handleLoad(id: string | null) {
    if (id) loadedId = id;
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

<div class="notch-backdrop" class:instant aria-hidden="true">
  {#if hasArt && artId}
    {#key instant ? 'static' : artId}
      <div class="backdrop-layer" class:animate-in={!instant}>
        {#if imgSrc}
          <img
            class="art-blur-img"
            class:ready={isImgReady}
            src={imgSrc}
            alt=""
            draggable="false"
            onload={() => handleLoad(artId)}
            onerror={() => handleError(artId)}
          />
        {/if}
        <div class="art-wash"></div>
        <div class="art-scrim"></div>
      </div>
    {/key}
  {:else}
    <div class="accent-glow" class:instant></div>
  {/if}
</div>

<style>
  .notch-backdrop {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }

  .backdrop-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .backdrop-layer.animate-in {
    animation: backdrop-fade 360ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  /* Enlarged blurred 128px cover opt-in (`V1`: enabled when `--ice-art-backdrop` > 0). */
  .art-blur-img {
    position: absolute;
    inset: -24px;
    width: calc(100% + 48px);
    height: calc(100% + 48px);
    object-fit: cover;
    filter: blur(var(--ice-art-blur, 28px)) saturate(1.25);
    transform: scale(1.06);
    opacity: 0;
    transition: opacity 280ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .art-blur-img.ready {
    opacity: var(--ice-art-backdrop, 0);
  }

  .notch-backdrop.instant .art-blur-img {
    transition: none;
  }

  /* Two-stop art-derived gradient wash (`V2` default; `0` in `Minimal`). */
  .art-wash {
    position: absolute;
    inset: 0;
    opacity: var(--ice-art-wash, 1);
    background:
      radial-gradient(
        76% 92% at 18% 18%,
        var(--ice-active-wash-primary, var(--ice-accent-glow)),
        transparent 68%
      ),
      radial-gradient(
        68% 84% at 88% 84%,
        var(--ice-active-wash-secondary, transparent),
        transparent 72%
      );
  }

  /* Luminance-clamped dark scrim protecting `--ice-fg` and `--ice-fg-muted` contrast. */
  .art-scrim {
    position: absolute;
    inset: 0;
    background: var(--ice-scrim, none);
  }

  .accent-glow {
    position: absolute;
    inset: 0;
    opacity: 0.28;
    background: radial-gradient(
      62% 44% at 50% 0%,
      var(--ice-active-glow, var(--ice-accent-glow)),
      transparent 72%
    );
    transition: opacity 400ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .accent-glow.instant {
    transition: none;
  }

  @keyframes backdrop-fade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
