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
    <!-- V1 scrim stays anchored behind the wash and only activates when V1 blur (`--ice-art-backdrop > 0`) is enabled. -->
    <div class="art-scrim"></div>
    <!-- Unkeyed wash keeps drift phase smooth across track changes and pin/pause transitions. -->
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
    left: -12%;
    top: -24%;
    width: 66%;
    height: 112%;
    background: radial-gradient(
      closest-side,
      var(--ice-active-wash-primary, var(--ice-accent-glow)) 0%,
      transparent 76%
    );
  }

  .wash-blob-b {
    right: -12%;
    bottom: -26%;
    width: 64%;
    height: 108%;
    background: radial-gradient(
      closest-side,
      var(--ice-active-wash-secondary, transparent) 0%,
      transparent 78%
    );
  }

  .wash-blob-c {
    left: 20%;
    top: -10%;
    width: 58%;
    height: 96%;
    background: radial-gradient(
      closest-side,
      var(--ice-active-wash-tertiary, transparent) 0%,
      transparent 78%
    );
  }

  .art-wash.ambient-active .wash-blob {
    will-change: transform;
  }

  .art-wash.ambient-active .wash-blob-a {
    animation: wash-drift-a var(--ice-ambient-period, 12s) ease-in-out infinite alternate;
  }

  .art-wash.ambient-active .wash-blob-b {
    animation: wash-drift-b calc(var(--ice-ambient-period, 12s) * 1.43) ease-in-out infinite alternate;
  }

  .art-wash.ambient-active .wash-blob-c {
    animation: wash-drift-c calc(var(--ice-ambient-period, 12s) * 1.79) ease-in-out infinite alternate;
  }

  .art-wash.ambient-paused .wash-blob {
    animation-play-state: paused;
  }

  :global([data-theme='minimal']) .art-wash.ambient-active .wash-blob {
    animation: none;
    will-change: auto;
  }

  /* Luminance-clamped V1 scrim protecting text contrast when `--ice-art-backdrop > 0`. */
  .art-scrim {
    background: var(--ice-scrim, none);
    opacity: clamp(0, calc(var(--ice-art-backdrop, 0) * 100), 1);
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
      transform: translate3d(-18px, -12px, 0) scale(0.94);
    }
    50% {
      transform: translate3d(52px, 22px, 0) scale(1.08);
    }
    100% {
      transform: translate3d(114px, -6px, 0) scale(0.98);
    }
  }

  @keyframes wash-drift-b {
    0% {
      transform: translate3d(18px, 14px, 0) scale(1.06);
    }
    50% {
      transform: translate3d(-56px, -22px, 0) scale(0.95);
    }
    100% {
      transform: translate3d(-112px, 8px, 0) scale(1.08);
    }
  }

  @keyframes wash-drift-c {
    0% {
      transform: translate3d(-34px, 10px, 0) scale(0.95);
    }
    100% {
      transform: translate3d(46px, -16px, 0) scale(1.08);
    }
  }
</style>
