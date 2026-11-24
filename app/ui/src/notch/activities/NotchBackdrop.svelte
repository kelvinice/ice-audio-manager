<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { ArtInfo } from '../../lib/ipc/engine';
  import type { AmbientMotionState } from '../../lib/notch/backdrop';

  interface Props {
    art: ArtInfo | null | undefined;
    hasArt: boolean;
    instant: boolean;
    ambientState?: AmbientMotionState;
    hasTertiary?: boolean;
    boxW?: number;
    boxH?: number;
    topPad?: number;
  }

  let {
    art,
    hasArt,
    instant,
    ambientState = 'stopped',
    hasTertiary = false,
    boxW = 392,
    boxH = 164,
    topPad = 6,
  }: Props = $props();

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
      </div>
    {/key}
    <!-- Unkeyed wash & scrim keep drift phase smooth across track changes and pin/pause transitions. -->
    <div
      class="art-wash"
      class:ambient-active={ambientState !== 'stopped'}
      class:ambient-paused={ambientState === 'paused'}
      style:top={`${topPad}px`}
      style:width={`${boxW}px`}
      style:height={`${boxH}px`}
    >
      <div class="wash-blob wash-blob-a"></div>
      <div class="wash-blob wash-blob-b"></div>
      {#if hasTertiary}
        <div class="wash-blob wash-blob-c"></div>
      {/if}
    </div>
    <div class="art-scrim"></div>
  {:else}
    <div class="accent-glow" class:instant></div>
  {/if}
</div>

<style>
  .notch-backdrop,
  .backdrop-layer,
  .art-scrim {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
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

  /* Two/three-stop art-derived gradient wash (`V2` default; `0` in `Minimal`), sized to the active card. */
  .art-wash {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    pointer-events: none;
    overflow: hidden;
    opacity: var(--ice-art-wash, 1);
  }

  .wash-blob {
    position: absolute;
    pointer-events: none;
    animation: none;
  }

  .wash-blob-a {
    left: -18%;
    top: -32%;
    width: 94%;
    height: 124%;
    background: radial-gradient(
      closest-side,
      var(--ice-active-wash-primary, var(--ice-accent-glow)) 0%,
      transparent 78%
    );
  }

  .wash-blob-b {
    right: -16%;
    bottom: -30%;
    width: 88%;
    height: 118%;
    background: radial-gradient(
      closest-side,
      var(--ice-active-wash-secondary, transparent) 0%,
      transparent 80%
    );
  }

  .wash-blob-c {
    left: 16%;
    bottom: -24%;
    width: 78%;
    height: 106%;
    background: radial-gradient(
      closest-side,
      var(--ice-active-wash-tertiary, transparent) 0%,
      transparent 80%
    );
  }

  .art-wash.ambient-active .wash-blob {
    will-change: transform;
  }

  .art-wash.ambient-active .wash-blob-a {
    animation: wash-drift-a var(--ice-ambient-period, 22s) ease-in-out infinite alternate;
  }

  .art-wash.ambient-active .wash-blob-b {
    animation: wash-drift-b calc(var(--ice-ambient-period, 22s) * 1.41) ease-in-out infinite alternate;
  }

  .art-wash.ambient-active .wash-blob-c {
    animation: wash-drift-c calc(var(--ice-ambient-period, 22s) * 1.73) ease-in-out infinite alternate;
  }

  .art-wash.ambient-paused .wash-blob {
    animation-play-state: paused;
  }

  :global([data-theme='minimal']) .art-wash.ambient-active .wash-blob {
    animation: none;
    will-change: auto;
  }

  /* Luminance-clamped dark scrim protecting `--ice-fg` and `--ice-fg-muted` contrast. */
  .art-scrim {
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

  @keyframes wash-drift-a {
    0% {
      transform: translate3d(-12px, -8px, 0) scale(0.96);
    }
    100% {
      transform: translate3d(24px, 14px, 0) scale(1.06);
    }
  }

  @keyframes wash-drift-b {
    0% {
      transform: translate3d(14px, 10px, 0) scale(1.05);
    }
    100% {
      transform: translate3d(-22px, -14px, 0) scale(0.95);
    }
  }

  @keyframes wash-drift-c {
    0% {
      transform: translate3d(-10px, 8px, 0) scale(0.97);
    }
    100% {
      transform: translate3d(16px, -12px, 0) scale(1.07);
    }
  }
</style>
