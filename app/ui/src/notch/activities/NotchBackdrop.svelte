<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { ArtInfo } from '../../lib/ipc/engine';
  import type { AmbientMotionState } from '../../lib/notch/backdrop';
  import { subscribeTempo } from '../../lib/notch/visualizer';

  interface Props {
    art: ArtInfo | null | undefined;
    hasArt: boolean;
    instant: boolean;
    ambientState?: AmbientMotionState;
    compactPill?: boolean;
    boxW?: number;
    boxH?: number;
    topPad?: number;
  }

  let {
    art,
    hasArt,
    instant,
    ambientState = 'stopped',
    compactPill = false,
    boxW = 392,
    boxH = 164,
    topPad = 6,
  }: Props = $props();

  let loadedId = $state<string | null>(null);
  let fallbackAttemptedId = $state<string | null>(null);
  let failedId = $state<string | null>(null);
  let washEl = $state<HTMLDivElement | null>(null);

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

  function applyTempoRate(rate: number): void {
    if (!washEl || typeof washEl.getAnimations !== 'function') return;
    for (const anim of washEl.getAnimations({ subtree: true })) {
      if (Math.abs(anim.playbackRate - rate) >= 0.04) {
        anim.playbackRate = rate;
      }
    }
  }

  // Smoothly scale running compositor keyframes with music tempo (no animation restart or phase snap).
  $effect(() => {
    if (ambientState !== 'running' || !washEl) return;
    return subscribeTempo((rate) => applyTempoRate(rate));
  });

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
    <!-- Unkeyed multi-stop aurora wash keeps drift phase smooth across track changes and pin/pause transitions. -->
    <div
      bind:this={washEl}
      class="art-wash"
      class:compact-pill={compactPill}
      class:ambient-active={ambientState !== 'stopped'}
      class:ambient-paused={ambientState === 'paused'}
      style:top={`${topPad}px`}
      style:width={`${boxW}px`}
      style:height={`${boxH}px`}
    >
      <div class="wash-base"></div>
      <div class="wash-blob wash-blob-a"></div>
      <div class="wash-blob wash-blob-b"></div>
      <div class="wash-blob wash-blob-c"></div>
      <div class="wash-blob wash-blob-d"></div>
    </div>
  {:else}
    <div class="accent-glow" class:instant></div>
  {/if}
</div>

<style>
  .notch-backdrop,
  .backdrop-layer,
  .art-scrim,
  .wash-base {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }

  .backdrop-layer.animate-in {
    animation: backdrop-fade 360ms cubic-bezier(0.22, 1, 0.36, 1);
  }

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

  .art-wash {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    pointer-events: none;
    overflow: hidden;
    opacity: var(--ice-art-wash, 1);
  }

  .wash-base {
    background: var(--ice-active-wash-base, none);
  }

  .wash-blob {
    position: absolute;
    pointer-events: none;
    animation: none;
  }

  /* Expanded card: 4 smaller colour pools weaving on coprime periods. */
  .wash-blob-a {
    left: -4%;
    top: -14%;
    width: 44%;
    height: 84%;
    background: radial-gradient(closest-side, var(--ice-active-wash-primary, var(--ice-accent-glow)) 0%, transparent 76%);
  }

  .wash-blob-b {
    right: -4%;
    bottom: -16%;
    width: 42%;
    height: 82%;
    background: radial-gradient(closest-side, var(--ice-active-wash-secondary, transparent) 0%, transparent 78%);
  }

  .wash-blob-c {
    left: 32%;
    top: -18%;
    width: 38%;
    height: 76%;
    background: radial-gradient(closest-side, var(--ice-active-wash-tertiary, transparent) 0%, transparent 78%);
  }

  .wash-blob-d {
    left: 16%;
    bottom: -18%;
    width: 36%;
    height: 72%;
    background: radial-gradient(closest-side, var(--ice-active-wash-quaternary, transparent) 0%, transparent 78%);
  }

  /* Compact music pill (292 × 46): balanced horizontal aurora centered vertically across the capsule. */
  .art-wash.compact-pill .wash-blob { top: -42%; bottom: auto; height: 184%; }
  .art-wash.compact-pill .wash-blob-a { left: -6%; width: 36%; }
  .art-wash.compact-pill .wash-blob-b { left: 22%; right: auto; width: 34%; }
  .art-wash.compact-pill .wash-blob-c { left: 48%; width: 32%; }
  .art-wash.compact-pill .wash-blob-d { left: auto; right: -6%; width: 34%; }

  .art-wash.ambient-active .wash-blob { will-change: transform; }
  .art-wash.ambient-active .wash-blob-a { animation: wash-drift-a var(--ice-ambient-period, 12s) ease-in-out infinite alternate; }
  .art-wash.ambient-active .wash-blob-b { animation: wash-drift-b calc(var(--ice-ambient-period, 12s) * 1.37) ease-in-out infinite alternate; }
  .art-wash.ambient-active .wash-blob-c { animation: wash-drift-c calc(var(--ice-ambient-period, 12s) * 1.71) ease-in-out infinite alternate; }
  .art-wash.ambient-active .wash-blob-d { animation: wash-drift-d calc(var(--ice-ambient-period, 12s) * 2.09) ease-in-out infinite alternate; }
  .art-wash.ambient-paused .wash-blob { animation-play-state: paused; }
  :global([data-theme='minimal']) .art-wash.ambient-active .wash-blob { animation: none; will-change: auto; }

  .art-scrim {
    background: var(--ice-scrim, none);
    opacity: clamp(0, calc(var(--ice-art-backdrop, 0) * 100), 1);
  }

  .accent-glow {
    position: absolute;
    inset: 0;
    opacity: 0.28;
    background: radial-gradient(62% 44% at 50% 0%, var(--ice-active-glow, var(--ice-accent-glow)), transparent 72%);
    transition: opacity 400ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .accent-glow.instant {
    transition: none;
  }

  @keyframes backdrop-fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes wash-drift-a {
    0% { transform: translate3d(-14px, -8px, 0) scale(0.94); }
    50% { transform: translate3d(72px, 28px, 0) scale(1.1); }
    100% { transform: translate3d(148px, 6px, 0) scale(0.96); }
  }

  @keyframes wash-drift-b {
    0% { transform: translate3d(14px, 10px, 0) scale(1.06); }
    50% { transform: translate3d(-76px, -28px, 0) scale(0.94); }
    100% { transform: translate3d(-150px, -4px, 0) scale(1.08); }
  }

  @keyframes wash-drift-c {
    0% { transform: translate3d(-58px, 6px, 0) scale(0.92); }
    50% { transform: translate3d(18px, 38px, 0) scale(1.12); }
    100% { transform: translate3d(78px, -4px, 0) scale(0.96); }
  }

  @keyframes wash-drift-d {
    0% { transform: translate3d(68px, -4px, 0) scale(1.06); }
    50% { transform: translate3d(-8px, -36px, 0) scale(0.94); }
    100% { transform: translate3d(-66px, 8px, 0) scale(1.1); }
  }
</style>
