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
    <!-- Unkeyed lava-lamp field: 9 full-bodied circular orbs overlap, merge, and sweep freely past edges. -->
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
      <div class="orb orb-8"></div>
      <div class="orb orb-9"></div>
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

  .backdrop-layer.animate-in { animation: backdrop-fade 360ms cubic-bezier(0.22, 1, 0.36, 1); }

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
    isolation: isolate;
    opacity: var(--ice-art-wash, 1);
    --orb-scale: 1;
  }

  .art-wash.compact-pill { --orb-scale: 0.56; }
  .wash-base { background: var(--ice-active-wash-base, none); }

  /* Full-bodied soft-halo wax orbs (wide 0-34% plateau + smooth feather; compositor-only transform, zero blur filter). */
  .orb {
    position: absolute;
    left: 50%;
    top: 50%;
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    pointer-events: none;
    mix-blend-mode: screen;
    background: radial-gradient(
      closest-side,
      var(--orb-c) 0%,
      var(--orb-c) 34%,
      color-mix(in srgb, var(--orb-c) 56%, transparent) 62%,
      transparent 100%
    );
    animation: none;
  }

  :global([data-theme='ice_light']) .orb { mix-blend-mode: normal; }

  .orb-1 { width: calc(228px * var(--orb-scale)); --orb-c: var(--ice-active-wash-primary, var(--ice-accent-glow)); transform: translate(-50%, -50%) rotate(28deg) translate3d(-118px, -28px, 0); }
  .orb-2 { width: calc(168px * var(--orb-scale)); --orb-c: var(--ice-active-wash-secondary, transparent); transform: translate(-50%, -50%) rotate(118deg) translate3d(58px, -68px, 0); }
  .orb-3 { width: calc(206px * var(--orb-scale)); --orb-c: var(--ice-active-wash-tertiary, transparent); transform: translate(-50%, -50%) rotate(-142deg) translate3d(-38px, 76px, 0); }
  .orb-4 { width: calc(182px * var(--orb-scale)); --orb-c: var(--ice-active-wash-quaternary, transparent); transform: translate(-50%, -50%) rotate(18deg) translate3d(76px, 44px, 0); }
  .orb-5 { width: calc(214px * var(--orb-scale)); --orb-c: var(--ice-active-wash-highlight, transparent); transform: translate(-50%, -50%) rotate(-58deg) translate3d(108px, -18px, 0); }
  .orb-6 { width: calc(148px * var(--orb-scale)); --orb-c: var(--ice-active-wash-secondary, transparent); transform: translate(-50%, -50%) rotate(148deg) translate3d(-84px, 24px, 0); }
  .orb-7 { width: calc(192px * var(--orb-scale)); --orb-c: var(--ice-active-wash-primary, var(--ice-accent-glow)); transform: translate(-50%, -50%) rotate(-74deg) translate3d(-22px, -84px, 0); }
  .orb-8 { width: calc(162px * var(--orb-scale)); --orb-c: var(--ice-active-wash-tertiary, transparent); transform: translate(-50%, -50%) rotate(64deg) translate3d(134px, -42px, 0); }
  .orb-9 { width: calc(176px * var(--orb-scale)); --orb-c: var(--ice-active-wash-quaternary, transparent); transform: translate(-50%, -50%) rotate(-112deg) translate3d(-128px, -46px, 0); }

  .art-wash.ambient-active .orb { will-change: transform; }
  .art-wash.ambient-active .orb-1 { animation: orb-1 calc(var(--ice-ambient-period, 9s) * 0.79) ease-in-out -2.1s infinite alternate; }
  .art-wash.ambient-active .orb-2 { animation: orb-2 calc(var(--ice-ambient-period, 9s) * 0.48) ease-in-out -1.1s infinite alternate; }
  .art-wash.ambient-active .orb-3 { animation: orb-3 calc(var(--ice-ambient-period, 9s) * 0.58) ease-in-out -3.6s infinite alternate; }
  .art-wash.ambient-active .orb-4 { animation: orb-4 calc(var(--ice-ambient-period, 9s) * 0.67) ease-in-out -1.8s infinite alternate; }
  .art-wash.ambient-active .orb-5 { animation: orb-5 calc(var(--ice-ambient-period, 9s) * 0.89) ease-in-out -0.6s infinite alternate; }
  .art-wash.ambient-active .orb-6 { animation: orb-6 calc(var(--ice-ambient-period, 9s) * 1.04) ease-in-out -6.4s infinite alternate; }
  .art-wash.ambient-active .orb-7 { animation: orb-7 calc(var(--ice-ambient-period, 9s) * 1.54) ease-in-out -5.7s infinite alternate; }
  .art-wash.ambient-active .orb-8 { animation: orb-8 calc(var(--ice-ambient-period, 9s) * 1.19) ease-in-out -4.2s infinite alternate; }
  .art-wash.ambient-active .orb-9 { animation: orb-9 calc(var(--ice-ambient-period, 9s) * 1.37) ease-in-out -7.8s infinite alternate; }
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

  /* Sweeping lava-lamp orbital arcs crossing diagonally, vertically, and horizontally past every edge. */
  @keyframes orb-1 {
    from { transform: translate(-50%, -50%) rotate(84deg) translate3d(118px, -16px, 0) scale(0.94); }
    to { transform: translate(-50%, -50%) rotate(124deg) translate3d(-96px, 24px, 0) scale(1.12); }
  }
  @keyframes orb-2 {
    from { transform: translate(-50%, -50%) rotate(196deg) translate3d(-112px, 42px, 0) scale(1.08); }
    to { transform: translate(-50%, -50%) rotate(244deg) translate3d(-94px, 56px, 0) scale(0.92); }
  }
  @keyframes orb-3 {
    from { transform: translate(-50%, -50%) rotate(-165deg) translate3d(88px, -124px, 0) scale(0.95); }
    to { transform: translate(-50%, -50%) rotate(-98deg) translate3d(96px, -118px, 0) scale(1.14); }
  }
  @keyframes orb-4 {
    from { transform: translate(-50%, -50%) rotate(28deg) translate3d(-72px, -84px, 0) scale(1.06); }
    to { transform: translate(-50%, -50%) rotate(-24deg) translate3d(-66px, -78px, 0) scale(0.94); }
  }
  @keyframes orb-5 {
    from { transform: translate(-50%, -50%) rotate(-54deg) translate3d(116px, 98px, 0) scale(0.96); }
    to { transform: translate(-50%, -50%) rotate(-78deg) translate3d(104px, 108px, 0) scale(1.1); }
  }
  @keyframes orb-6 {
    from { transform: translate(-50%, -50%) rotate(122deg) translate3d(-134px, 152px, 0) scale(1.05); }
    to { transform: translate(-50%, -50%) rotate(92deg) translate3d(156px, 146px, 0) scale(0.94); }
  }
  @keyframes orb-7 {
    from { transform: translate(-50%, -50%) rotate(-54deg) translate3d(-86px, 168px, 0) scale(0.94); }
    to { transform: translate(-50%, -50%) rotate(-76deg) translate3d(-72px, -152px, 0) scale(1.12); }
  }
  @keyframes orb-8 {
    from { transform: translate(-50%, -50%) rotate(42deg) translate3d(164px, -96px, 0) scale(1.08); }
    to { transform: translate(-50%, -50%) rotate(-18deg) translate3d(-154px, 86px, 0) scale(0.92); }
  }
  @keyframes orb-9 {
    from { transform: translate(-50%, -50%) rotate(-118deg) translate3d(-148px, -104px, 0) scale(0.96); }
    to { transform: translate(-50%, -50%) rotate(-62deg) translate3d(146px, 98px, 0) scale(1.1); }
  }
</style>
