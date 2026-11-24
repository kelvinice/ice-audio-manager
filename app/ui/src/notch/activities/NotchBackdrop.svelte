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
    <!-- Unkeyed orbital lava-lamp field: 7 soft circular orbs sweep freely across and past the pill edges. -->
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
      <div class="orb orb-1"></div>
      <div class="orb orb-2"></div>
      <div class="orb orb-3"></div>
      <div class="orb orb-4"></div>
      <div class="orb orb-5"></div>
      <div class="orb orb-6"></div>
      <div class="orb orb-7"></div>
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

  .art-blur-img.ready { opacity: var(--ice-art-backdrop, 0); }
  .notch-backdrop.instant .art-blur-img { transition: none; }

  .art-wash {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    pointer-events: none;
    overflow: hidden;
    opacity: var(--ice-art-wash, 1);
    --orb-scale: 1;
  }

  .art-wash.compact-pill {
    --orb-scale: 0.62;
  }

  .wash-base {
    background: var(--ice-active-wash-base, none);
  }

  /* Centered circular soft radial orbs; `rotate + translate3d` orbits sweep diagonally/vertically past edges. */
  .orb {
    position: absolute;
    left: 50%;
    top: 50%;
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    pointer-events: none;
    animation: none;
  }

  .orb-1 {
    width: calc(168px * var(--orb-scale));
    background: radial-gradient(closest-side, var(--ice-active-wash-primary, var(--ice-accent-glow)) 0%, transparent 74%);
    transform: translate(-50%, -50%) rotate(32deg) translate3d(-115px, -38px, 0);
  }

  .orb-2 {
    width: calc(114px * var(--orb-scale));
    background: radial-gradient(closest-side, var(--ice-active-wash-secondary, transparent) 0%, transparent 74%);
    transform: translate(-50%, -50%) rotate(118deg) translate3d(68px, -74px, 0);
  }

  .orb-3 {
    width: calc(146px * var(--orb-scale));
    background: radial-gradient(closest-side, var(--ice-active-wash-tertiary, transparent) 0%, transparent 74%);
    transform: translate(-50%, -50%) rotate(-142deg) translate3d(-44px, 86px, 0);
  }

  .orb-4 {
    width: calc(122px * var(--orb-scale));
    background: radial-gradient(closest-side, var(--ice-active-wash-quaternary, transparent) 0%, transparent 74%);
    transform: translate(-50%, -50%) rotate(18deg) translate3d(78px, 52px, 0);
  }

  .orb-5 {
    width: calc(154px * var(--orb-scale));
    background: radial-gradient(closest-side, var(--ice-active-wash-highlight, transparent) 0%, transparent 74%);
    transform: translate(-50%, -50%) rotate(-58deg) translate3d(118px, -24px, 0);
  }

  .orb-6 {
    width: calc(96px * var(--orb-scale));
    background: radial-gradient(closest-side, var(--ice-active-wash-secondary, transparent) 0%, transparent 74%);
    transform: translate(-50%, -50%) rotate(148deg) translate3d(-88px, 28px, 0);
  }

  .orb-7 {
    width: calc(136px * var(--orb-scale));
    background: radial-gradient(closest-side, var(--ice-active-wash-primary, var(--ice-accent-glow)) 0%, transparent 74%);
    transform: translate(-50%, -50%) rotate(-74deg) translate3d(-26px, -96px, 0);
  }

  .art-wash.ambient-active .orb { will-change: transform; }
  .art-wash.ambient-active .orb-1 { animation: orb-1 calc(var(--ice-ambient-period, 12s) * 0.92) ease-in-out -2.1s infinite alternate; }
  .art-wash.ambient-active .orb-2 { animation: orb-2 calc(var(--ice-ambient-period, 12s) * 0.63) ease-in-out -1.3s infinite alternate; }
  .art-wash.ambient-active .orb-3 { animation: orb-3 calc(var(--ice-ambient-period, 12s) * 0.77) ease-in-out -4.4s infinite alternate; }
  .art-wash.ambient-active .orb-4 { animation: orb-4 calc(var(--ice-ambient-period, 12s) * 1.12) ease-in-out -2.7s infinite alternate; }
  .art-wash.ambient-active .orb-5 { animation: orb-5 calc(var(--ice-ambient-period, 12s) * 0.86) ease-in-out -6.1s infinite alternate; }
  .art-wash.ambient-active .orb-6 { animation: orb-6 calc(var(--ice-ambient-period, 12s) * 0.57) ease-in-out -3.9s infinite alternate; }
  .art-wash.ambient-active .orb-7 { animation: orb-7 calc(var(--ice-ambient-period, 12s) * 1.31) ease-in-out -8.2s infinite alternate; }
  .art-wash.ambient-paused .orb { animation-play-state: paused; }
  :global([data-theme='minimal']) .art-wash.ambient-active .orb { animation: none; will-change: auto; }

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

  .accent-glow.instant { transition: none; }

  @keyframes backdrop-fade { from { opacity: 0; } to { opacity: 1; } }

  /* Sweeping orbital paths: diagonal, vertical, and cross-card arcs drifting in and out past edges. */
  @keyframes orb-1 {
    from { transform: translate(-50%, -50%) rotate(18deg) translate3d(-175px, -54px, 0) scale(0.92); }
    to { transform: translate(-50%, -50%) rotate(58deg) translate3d(165px, 46px, 0) scale(1.1); }
  }

  @keyframes orb-2 {
    from { transform: translate(-50%, -50%) rotate(84deg) translate3d(-120px, -96px, 0) scale(1.06); }
    to { transform: translate(-50%, -50%) rotate(148deg) translate3d(110px, 92px, 0) scale(0.9); }
  }

  @keyframes orb-3 {
    from { transform: translate(-50%, -50%) rotate(-168deg) translate3d(-135px, 98px, 0) scale(0.94); }
    to { transform: translate(-50%, -50%) rotate(-98deg) translate3d(145px, -84px, 0) scale(1.12); }
  }

  @keyframes orb-4 {
    from { transform: translate(-50%, -50%) rotate(42deg) translate3d(160px, -78px, 0) scale(1.08); }
    to { transform: translate(-50%, -50%) rotate(-28deg) translate3d(-150px, 76px, 0) scale(0.94); }
  }

  @keyframes orb-5 {
    from { transform: translate(-50%, -50%) rotate(-46deg) translate3d(185px, 68px, 0) scale(0.96); }
    to { transform: translate(-50%, -50%) rotate(-82deg) translate3d(-170px, -62px, 0) scale(1.08); }
  }

  @keyframes orb-6 {
    from { transform: translate(-50%, -50%) rotate(116deg) translate3d(-64px, -118px, 0) scale(1.04); }
    to { transform: translate(-50%, -50%) rotate(76deg) translate3d(72px, 118px, 0) scale(0.92); }
  }

  @keyframes orb-7 {
    from { transform: translate(-50%, -50%) rotate(-52deg) translate3d(-96px, 126px, 0) scale(0.95); }
    to { transform: translate(-50%, -50%) rotate(-96deg) translate3d(112px, -122px, 0) scale(1.12); }
  }
</style>
