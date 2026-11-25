<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { ArtInfo } from '../../lib/ipc/engine';
  import { ambientTempoRate, BREATH_SCALE } from '../../lib/notch/ambient';
  import type { AmbientMotionState } from '../../lib/notch/backdrop';
  import { subscribeBreath, subscribeTempo } from '../../lib/notch/visualizer';

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
  let fieldEl = $state<HTMLDivElement | null>(null);

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

  function applyTempoRate(beatTempo: number): void {
    if (!washEl || typeof washEl.getAnimations !== 'function') return;
    // A calm range (0.85–1.2×): faster songs make the lamp livelier, never racing.
    const rate = ambientTempoRate(beatTempo);
    for (const anim of washEl.getAnimations({ subtree: true })) {
      // Orb loops only (not the breath's CSS transition). `updatePlaybackRate` keeps the phase and stays in sync with
      // the compositor instead of forcing a main-thread sync like assigning `playbackRate`.
      if (!(anim instanceof CSSAnimation) || Math.abs(anim.playbackRate - rate) < 0.04) continue;
      if (typeof anim.updatePlaybackRate === 'function') anim.updatePlaybackRate(rate);
      else anim.playbackRate = rate;
    }
  }

  // Scale the running loops with the music's tempo (no restart, no phase snap).
  $effect(() => {
    if (ambientState !== 'running' || !washEl) return;
    return subscribeTempo((rate) => applyTempoRate(rate));
  });

  // Bass breath: the whole wash swells a little on each kick and exhales slowly (`ambient.ts`). One compositor-only
  // transform on `.orb-field`, written only when the value changes, from meter frames that already arrive. Off when
  // paused, hidden, reduced motion / preset Off, or the moving background is off (`ambientState`).
  $effect(() => {
    const field = fieldEl;
    if (ambientState !== 'running' || instant || !field) return;
    const stop = subscribeBreath((breath) => {
      field.style.transform = breath > 0 ? `scale(${(1 + BREATH_SCALE * breath).toFixed(4)})` : '';
    });
    return () => {
      stop();
      field.style.transform = '';
    };
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
    <!-- Unkeyed lava-lamp field: 9 wax orbs, each looping around its own spot (never reversing). Colours fade between
         songs (registered `--orb-c*`), the field breathes with the bass. -->
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
      <div class="orb-field" bind:this={fieldEl}>
        <div class="wash-base"></div>
        <div class="orb-layer">
          {#each { length: 9 }, i (i)}
            <div class="orb orb-{i + 1}"></div>
          {/each}
        </div>
      </div>
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
    opacity: var(--ice-art-wash, 1);
    --orb-scale: 1;
    /* Registered `<color>`s (`@property` in themes/base.css), so a new song's palette fades in instead of snapping. */
    --orb-c1: var(--ice-active-wash-primary, var(--ice-accent-glow));
    --orb-c2: var(--ice-active-wash-secondary, transparent);
    --orb-c3: var(--ice-active-wash-tertiary, transparent);
    --orb-c4: var(--ice-active-wash-quaternary, transparent);
    --orb-c5: var(--ice-active-wash-highlight, transparent);
    transition:
      --orb-c1 1400ms ease,
      --orb-c2 1400ms ease,
      --orb-c3 1400ms ease,
      --orb-c4 1400ms ease,
      --orb-c5 1400ms ease;
  }

  .notch-backdrop.instant .art-wash {
    transition: none;
  }

  .art-wash.compact-pill { --orb-scale: 0.56; }

  /* The breath target (`transform` written from script) and the blend group for the orbs. */
  .orb-field {
    position: absolute;
    inset: 0;
    isolation: isolate;
    transform-origin: 50% 50%;
    transition: transform 90ms linear;
  }

  .notch-backdrop.instant .orb-field {
    transition: none;
  }

  .wash-base { background: var(--ice-active-wash-base, none); }

  /* A zero-size point at the centre; the compact pill shrinks every orb path at once (static, so the loops below stay
     plain-pixel, compositor-only keyframes). */
  .orb-layer {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 0;
    height: 0;
    transform: scale(var(--orb-scale));
  }

  /* Full-bodied soft-halo wax orbs (wide 0-34% plateau + smooth feather; compositor-only transform, zero blur filter).
     `lighten`, not `screen`: overlaps never get brighter than the brightest orb, whose lightness is capped for text
     contrast (`tuneWashColor`); `screen` added overlaps up towards white behind muted text. */
  .orb {
    position: absolute;
    left: 0;
    top: 0;
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    pointer-events: none;
    mix-blend-mode: lighten;
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

  /* Generated by a script from (centre, start angle, direction, 4 radii, 4 swells, period, phase) per orb. */
  .orb-1 { width: 228px; --orb-c: var(--orb-c1); transform: translate(-50%, -50%) translate3d(-170px, -10px, 0) rotate(20deg) translate3d(70px, 0, 0) scale(0.96); }
  .orb-2 { width: 168px; --orb-c: var(--orb-c2); transform: translate(-50%, -50%) translate3d(150px, 20px, 0) rotate(140deg) translate3d(62px, 0, 0) scale(1.06); }
  .orb-3 { width: 206px; --orb-c: var(--orb-c3); transform: translate(-50%, -50%) translate3d(-40px, 30px, 0) rotate(250deg) translate3d(54px, 0, 0) scale(0.95); }
  .orb-4 { width: 182px; --orb-c: var(--orb-c4); transform: translate(-50%, -50%) translate3d(60px, -30px, 0) rotate(80deg) translate3d(66px, 0, 0) scale(1.04); }
  .orb-5 { width: 214px; --orb-c: var(--orb-c5); transform: translate(-50%, -50%) translate3d(210px, -10px, 0) rotate(300deg) translate3d(58px, 0, 0) scale(0.97); }
  .orb-6 { width: 148px; --orb-c: var(--orb-c2); transform: translate(-50%, -50%) translate3d(-120px, 40px, 0) rotate(190deg) translate3d(72px, 0, 0) scale(1.05); }
  .orb-7 { width: 192px; --orb-c: var(--orb-c1); transform: translate(-50%, -50%) translate3d(0px, -20px, 0) rotate(40deg) translate3d(84px, 0, 0) scale(0.94); }
  .orb-8 { width: 162px; --orb-c: var(--orb-c3); transform: translate(-50%, -50%) translate3d(110px, 30px, 0) rotate(220deg) translate3d(60px, 0, 0) scale(1.08); }
  .orb-9 { width: 176px; --orb-c: var(--orb-c4); transform: translate(-50%, -50%) translate3d(-220px, 0px, 0) rotate(330deg) translate3d(64px, 0, 0) scale(0.96); }

  .art-wash.ambient-active .orb { will-change: transform; }
  /* One full loop per `--ice-ambient-period × multiplier` (coprime-ish multipliers, so the pattern never repeats
     visibly); `linear` + closed loops = steady drift with no turn-arounds. */
  .art-wash.ambient-active .orb-1 { animation: orb-1 calc(var(--ice-ambient-period, 30s) * 0.83) linear calc(var(--ice-ambient-period, 30s) * -0.1) infinite; }
  .art-wash.ambient-active .orb-2 { animation: orb-2 calc(var(--ice-ambient-period, 30s) * 1.0) linear calc(var(--ice-ambient-period, 30s) * -0.47) infinite; }
  .art-wash.ambient-active .orb-3 { animation: orb-3 calc(var(--ice-ambient-period, 30s) * 0.91) linear calc(var(--ice-ambient-period, 30s) * -0.664) infinite; }
  .art-wash.ambient-active .orb-4 { animation: orb-4 calc(var(--ice-ambient-period, 30s) * 1.17) linear calc(var(--ice-ambient-period, 30s) * -0.339) infinite; }
  .art-wash.ambient-active .orb-5 { animation: orb-5 calc(var(--ice-ambient-period, 30s) * 0.77) linear calc(var(--ice-ambient-period, 30s) * -0.447) infinite; }
  .art-wash.ambient-active .orb-6 { animation: orb-6 calc(var(--ice-ambient-period, 30s) * 1.29) linear calc(var(--ice-ambient-period, 30s) * -1.109) infinite; }
  .art-wash.ambient-active .orb-7 { animation: orb-7 calc(var(--ice-ambient-period, 30s) * 1.09) linear calc(var(--ice-ambient-period, 30s) * -0.382) infinite; }
  .art-wash.ambient-active .orb-8 { animation: orb-8 calc(var(--ice-ambient-period, 30s) * 0.95) linear calc(var(--ice-ambient-period, 30s) * -0.608) infinite; }
  .art-wash.ambient-active .orb-9 { animation: orb-9 calc(var(--ice-ambient-period, 30s) * 1.37) linear calc(var(--ice-ambient-period, 30s) * -0.26) infinite; }
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

  /* Closed orbital loops: each ends where it starts (angle + 360°), so `infinite` wraps without a jump. */
  @keyframes orb-1 {
    0% { transform: translate(-50%, -50%) translate3d(-170px, -10px, 0) rotate(20deg) translate3d(70px, 0, 0) scale(0.96); }
    25% { transform: translate(-50%, -50%) translate3d(-170px, -10px, 0) rotate(110deg) translate3d(95px, 0, 0) scale(1.08); }
    50% { transform: translate(-50%, -50%) translate3d(-170px, -10px, 0) rotate(200deg) translate3d(78px, 0, 0) scale(1.0); }
    75% { transform: translate(-50%, -50%) translate3d(-170px, -10px, 0) rotate(290deg) translate3d(104px, 0, 0) scale(1.12); }
    100% { transform: translate(-50%, -50%) translate3d(-170px, -10px, 0) rotate(380deg) translate3d(70px, 0, 0) scale(0.96); }
  }
  @keyframes orb-2 {
    0% { transform: translate(-50%, -50%) translate3d(150px, 20px, 0) rotate(140deg) translate3d(62px, 0, 0) scale(1.06); }
    25% { transform: translate(-50%, -50%) translate3d(150px, 20px, 0) rotate(50deg) translate3d(88px, 0, 0) scale(0.94); }
    50% { transform: translate(-50%, -50%) translate3d(150px, 20px, 0) rotate(-40deg) translate3d(70px, 0, 0) scale(1.1); }
    75% { transform: translate(-50%, -50%) translate3d(150px, 20px, 0) rotate(-130deg) translate3d(96px, 0, 0) scale(0.98); }
    100% { transform: translate(-50%, -50%) translate3d(150px, 20px, 0) rotate(-220deg) translate3d(62px, 0, 0) scale(1.06); }
  }
  @keyframes orb-3 {
    0% { transform: translate(-50%, -50%) translate3d(-40px, 30px, 0) rotate(250deg) translate3d(54px, 0, 0) scale(0.95); }
    25% { transform: translate(-50%, -50%) translate3d(-40px, 30px, 0) rotate(340deg) translate3d(80px, 0, 0) scale(1.1); }
    50% { transform: translate(-50%, -50%) translate3d(-40px, 30px, 0) rotate(430deg) translate3d(64px, 0, 0) scale(0.98); }
    75% { transform: translate(-50%, -50%) translate3d(-40px, 30px, 0) rotate(520deg) translate3d(90px, 0, 0) scale(1.06); }
    100% { transform: translate(-50%, -50%) translate3d(-40px, 30px, 0) rotate(610deg) translate3d(54px, 0, 0) scale(0.95); }
  }
  @keyframes orb-4 {
    0% { transform: translate(-50%, -50%) translate3d(60px, -30px, 0) rotate(80deg) translate3d(66px, 0, 0) scale(1.04); }
    25% { transform: translate(-50%, -50%) translate3d(60px, -30px, 0) rotate(-10deg) translate3d(92px, 0, 0) scale(0.95); }
    50% { transform: translate(-50%, -50%) translate3d(60px, -30px, 0) rotate(-100deg) translate3d(74px, 0, 0) scale(1.12); }
    75% { transform: translate(-50%, -50%) translate3d(60px, -30px, 0) rotate(-190deg) translate3d(100px, 0, 0) scale(1.0); }
    100% { transform: translate(-50%, -50%) translate3d(60px, -30px, 0) rotate(-280deg) translate3d(66px, 0, 0) scale(1.04); }
  }
  @keyframes orb-5 {
    0% { transform: translate(-50%, -50%) translate3d(210px, -10px, 0) rotate(300deg) translate3d(58px, 0, 0) scale(0.97); }
    25% { transform: translate(-50%, -50%) translate3d(210px, -10px, 0) rotate(390deg) translate3d(84px, 0, 0) scale(1.09); }
    50% { transform: translate(-50%, -50%) translate3d(210px, -10px, 0) rotate(480deg) translate3d(66px, 0, 0) scale(1.02); }
    75% { transform: translate(-50%, -50%) translate3d(210px, -10px, 0) rotate(570deg) translate3d(92px, 0, 0) scale(1.11); }
    100% { transform: translate(-50%, -50%) translate3d(210px, -10px, 0) rotate(660deg) translate3d(58px, 0, 0) scale(0.97); }
  }
  @keyframes orb-6 {
    0% { transform: translate(-50%, -50%) translate3d(-120px, 40px, 0) rotate(190deg) translate3d(72px, 0, 0) scale(1.05); }
    25% { transform: translate(-50%, -50%) translate3d(-120px, 40px, 0) rotate(100deg) translate3d(98px, 0, 0) scale(0.96); }
    50% { transform: translate(-50%, -50%) translate3d(-120px, 40px, 0) rotate(10deg) translate3d(80px, 0, 0) scale(1.1); }
    75% { transform: translate(-50%, -50%) translate3d(-120px, 40px, 0) rotate(-80deg) translate3d(106px, 0, 0) scale(0.94); }
    100% { transform: translate(-50%, -50%) translate3d(-120px, 40px, 0) rotate(-170deg) translate3d(72px, 0, 0) scale(1.05); }
  }
  @keyframes orb-7 {
    0% { transform: translate(-50%, -50%) translate3d(0px, -20px, 0) rotate(40deg) translate3d(84px, 0, 0) scale(0.94); }
    25% { transform: translate(-50%, -50%) translate3d(0px, -20px, 0) rotate(130deg) translate3d(110px, 0, 0) scale(1.1); }
    50% { transform: translate(-50%, -50%) translate3d(0px, -20px, 0) rotate(220deg) translate3d(92px, 0, 0) scale(1.0); }
    75% { transform: translate(-50%, -50%) translate3d(0px, -20px, 0) rotate(310deg) translate3d(118px, 0, 0) scale(1.08); }
    100% { transform: translate(-50%, -50%) translate3d(0px, -20px, 0) rotate(400deg) translate3d(84px, 0, 0) scale(0.94); }
  }
  @keyframes orb-8 {
    0% { transform: translate(-50%, -50%) translate3d(110px, 30px, 0) rotate(220deg) translate3d(60px, 0, 0) scale(1.08); }
    25% { transform: translate(-50%, -50%) translate3d(110px, 30px, 0) rotate(130deg) translate3d(86px, 0, 0) scale(0.96); }
    50% { transform: translate(-50%, -50%) translate3d(110px, 30px, 0) rotate(40deg) translate3d(68px, 0, 0) scale(1.04); }
    75% { transform: translate(-50%, -50%) translate3d(110px, 30px, 0) rotate(-50deg) translate3d(94px, 0, 0) scale(0.92); }
    100% { transform: translate(-50%, -50%) translate3d(110px, 30px, 0) rotate(-140deg) translate3d(60px, 0, 0) scale(1.08); }
  }
  @keyframes orb-9 {
    0% { transform: translate(-50%, -50%) translate3d(-220px, 0px, 0) rotate(330deg) translate3d(64px, 0, 0) scale(0.96); }
    25% { transform: translate(-50%, -50%) translate3d(-220px, 0px, 0) rotate(420deg) translate3d(90px, 0, 0) scale(1.06); }
    50% { transform: translate(-50%, -50%) translate3d(-220px, 0px, 0) rotate(510deg) translate3d(72px, 0, 0) scale(0.94); }
    75% { transform: translate(-50%, -50%) translate3d(-220px, 0px, 0) rotate(600deg) translate3d(98px, 0, 0) scale(1.1); }
    100% { transform: translate(-50%, -50%) translate3d(-220px, 0px, 0) rotate(690deg) translate3d(64px, 0, 0) scale(0.96); }
  }
</style>
