<script lang="ts">
  import { onMount } from 'svelte';
  import { createNotchController } from '../lib/notch/controller.svelte';
  import { CANVAS_H, CANVAS_W } from '../lib/notch/geometry';
  import BeatRim from './activities/BeatRim.svelte';
  import ExpandedNotch from './activities/ExpandedNotch.svelte';
  import IdlePill from './activities/IdlePill.svelte';
  import LandingRipple from './activities/LandingRipple.svelte';
  import LyricsExtension from './activities/LyricsExtension.svelte';
  import MicActivity from './activities/MicActivity.svelte';
  import NookExpandedNotch from './activities/NookExpandedNotch.svelte';
  import NotchBackdrop from './activities/NotchBackdrop.svelte';
  import PlainNotch from './activities/PlainNotch.svelte';
  import PlugActivity from './activities/PlugActivity.svelte';
  import SwitchActivity from './activities/SwitchActivity.svelte';
  import TrackActivity from './activities/TrackActivity.svelte';
  import VolumeActivity from './activities/VolumeActivity.svelte';

  const ctrl = createNotchController();

  onMount(() => ctrl.mount());
</script>

<div
  class="stage"
  class:hidden={(ctrl.notchSnapshot?.hiddenForBench ?? false) || ctrl.isSuspended}
  class:idle-hidden={ctrl.isIdleHidden}
  class:instant={ctrl.isInstant}
  class:plain={ctrl.isHibernating}
  style:width={`${CANVAS_W}px`}
  style:height={`${CANVAS_H}px`}
>
  <div
    class="notch-motion"
    class:bottom-anchor={ctrl.isBottomAnchor}
    class:nook={ctrl.notchStyle === 'nook'}
    class:place-settle={ctrl.placedFlash && !ctrl.isInstant}
    class:jelly={ctrl.dragEffects.jellyLanding}
    style:--lift={ctrl.lift.toFixed(3)}
  >
  <div
    class="pill-shadow"
    style:top={ctrl.isBottomAnchor ? undefined : `${ctrl.surfaceShape.topPad}px`}
    style:bottom={ctrl.isBottomAnchor ? `${ctrl.surfaceShape.bottomPad}px` : undefined}
    style:width={`${ctrl.pillSize.w}px`}
    style:height={`${ctrl.pillSize.h}px`}
    style:border-radius={ctrl.surfaceShape.shadowRadiusCss}
    aria-hidden="true"
  ></div>
  <!-- Held / carried: a deeper shadow fades in with the lift; absent at rest. -->
  {#if ctrl.lift > 0.001}
    <div
      class="lift-shadow"
      style:top={ctrl.isBottomAnchor ? undefined : `${ctrl.surfaceShape.topPad}px`}
      style:bottom={ctrl.isBottomAnchor ? `${ctrl.surfaceShape.bottomPad}px` : undefined}
      style:width={`${ctrl.pillSize.w}px`}
      style:height={`${ctrl.pillSize.h}px`}
      style:border-radius={ctrl.surfaceShape.shadowRadiusCss}
      style:opacity={Math.min(1, ctrl.lift).toFixed(3)}
      aria-hidden="true"
    ></div>
    {#if ctrl.dragEffects.carryGlow}
      <!-- Experimental "Glow while carrying": a crisp ring of the song's colour hugging the pill plus a halo that
           breathes slowly (opacity only) while it's held. The outer box follows the lift; the inner one breathes.
           The window ends a few px past the docked side, so the halo leans away from it and fades out by the window
           edge (`--glow-room`) instead of being cut there. -->
      <div
        class="carry-glow"
        class:still={ctrl.isInstant}
        class:bottom-anchor={ctrl.isBottomAnchor}
        style:top={ctrl.isBottomAnchor ? undefined : `${ctrl.surfaceShape.topPad}px`}
        style:bottom={ctrl.isBottomAnchor ? `${ctrl.surfaceShape.bottomPad}px` : undefined}
        style:width={`${ctrl.pillSize.w}px`}
        style:height={`${ctrl.pillSize.h}px`}
        style:border-radius={ctrl.surfaceShape.shadowRadiusCss}
        style:opacity={Math.min(1, ctrl.lift).toFixed(3)}
        style:--glow-c={ctrl.activeAccentCss}
        style:--glow-r={ctrl.surfaceShape.shadowRadiusCss}
        style:--glow-room={`${Math.max(0, ctrl.isBottomAnchor ? CANVAS_H - ctrl.surfaceShape.insetTop - ctrl.pillSize.h : ctrl.surfaceShape.insetTop).toFixed(1)}px`}
        aria-hidden="true"
      >
        <div class="carry-halo-fade"><div class="carry-halo"></div></div>
      </div>
    {/if}
  {/if}
  <div
    class="pill-clip"
    style:clip-path={ctrl.surfaceShape.clipPath}
    onpointerdown={ctrl.onPointerDown}
    onpointerenter={ctrl.onPointerEnter}
    onpointermove={ctrl.onPointerMove}
    onpointerleave={ctrl.onPointerLeave}
    onwheel={ctrl.onWheel}
    onauxclick={ctrl.onAuxClick}
    role="region"
    aria-label="Ice Audio Notch"
  >
    <div
      class="pill-surface"
      class:plain={ctrl.isHibernating}
      style:--ice-active-accent={ctrl.activeAccentCss}
      style:--ice-active-glow={ctrl.activeGlowCss}
      style:--ice-active-wash-base={ctrl.artBackdrop.washBaseCss}
      style:--ice-active-wash-primary={ctrl.artBackdrop.washPrimaryCss}
      style:--ice-active-wash-secondary={ctrl.artBackdrop.washSecondaryCss}
      style:--ice-active-wash-tertiary={ctrl.artBackdrop.washTertiaryCss}
      style:--ice-active-wash-quaternary={ctrl.artBackdrop.washQuaternaryCss}
      style:--ice-active-wash-highlight={ctrl.artBackdrop.washHighlightCss}
      style:--ice-active-scrim-top={ctrl.artBackdrop.scrimTopCss}
      style:--ice-active-scrim-bottom={ctrl.artBackdrop.scrimBottomCss}
    >
      {#if !ctrl.isHibernating}
      <NotchBackdrop
        art={ctrl.displayedMedia?.art}
        hasArt={ctrl.hasBackdropArt}
        instant={ctrl.isInstant}
        ambientState={ctrl.ambientMotionState}
        compactPill={!ctrl.isExpanded}
        boxW={ctrl.targetBox.w + ctrl.surfaceShape.earRadius * 2}
        boxH={ctrl.targetBox.h}
        topPad={ctrl.surfaceShape.topPad}
        isBottom={ctrl.isBottomAnchor}
        bottomPad={ctrl.surfaceShape.bottomPad}
      />
      {/if}
      <div
        class="pill-slot"
        class:nook={ctrl.notchStyle === 'nook'}
        class:compact={!ctrl.isExpanded}
        class:bottom-anchor={ctrl.isBottomAnchor}
        style:top={ctrl.isBottomAnchor ? undefined : `${ctrl.surfaceShape.topPad}px`}
        style:bottom={ctrl.isBottomAnchor ? `${ctrl.surfaceShape.bottomPad}px` : undefined}
        style:width={`${ctrl.targetBox.w}px`}
        style:height={`${ctrl.targetBox.h}px`}
      >
        {#if ctrl.isHibernating}
          <PlainNotch micMuted={ctrl.effectiveMicMuted} centred={ctrl.idleMode === 'dot'} />
        {:else if ctrl.notchSnapshot && !ctrl.isExpanded}
          {#if ctrl.activeView.kind === 'volume'}
            <VolumeActivity activity={ctrl.activeView} barPercent={ctrl.pillSize.vol} instant={ctrl.isInstant} />
          {:else if ctrl.activeView.kind === 'switch'}
            <SwitchActivity activity={ctrl.activeView} instant={ctrl.isInstant} />
          {:else if ctrl.activeView.kind === 'plug'}
            <PlugActivity activity={ctrl.activeView} instant={ctrl.isInstant} />
          {:else if ctrl.activeView.kind === 'mic'}
            <MicActivity activity={ctrl.activeView} instant={ctrl.isInstant} />
          {:else if ctrl.activeView.kind === 'track'}
            <div class="track-with-lyrics" class:bottom-anchor={ctrl.isBottomAnchor}>
              <div class="track-head">
                <TrackActivity
                  activity={ctrl.activeView}
                  instant={ctrl.isInstant}
                  preset={ctrl.notchSnapshot.preset}
                  notchVisible={ctrl.isNotchVisible}
                />
              </div>
              {#if ctrl.showPillLyrics}
                <LyricsExtension
                  lyrics={ctrl.engineState.lyrics}
                  media={ctrl.effectiveMedia ?? ctrl.activeView.media}
                  mode="pill"
                  wordHighlight={ctrl.engineState.settings?.lyrics.wordHighlight ?? true}
                  estimatedWordSync={ctrl.engineState.settings?.lyrics.estimatedWordSync ?? false}
                  showRomaji={ctrl.engineState.settings?.lyrics.romaji ?? true}
                  instant={ctrl.isInstant}
                  notchVisible={ctrl.isNotchVisible}
                  delayMs={ctrl.lyricsDelayMs}
                  attachedTop={ctrl.isBottomAnchor}
                />
              {/if}
            </div>
          {:else}
            <IdlePill
              deviceLabel={ctrl.effectiveDeviceShort}
              volumePercent={ctrl.effectiveVolumePercent}
              muted={ctrl.effectiveVolumeMuted}
              micMuted={ctrl.effectiveMicMuted}
              media={ctrl.effectiveMedia}
              idleMode={ctrl.idleMode}
              preset={ctrl.notchSnapshot.preset}
              instant={ctrl.isInstant}
              notchVisible={ctrl.isNotchVisible}
            />
          {/if}
        {:else if ctrl.notchSnapshot && ctrl.isExpanded && ctrl.notchStyle === 'nook'}
          <NookExpandedNotch
            playback={ctrl.engineState.playback}
            settings={ctrl.engineState.settings}
            disconnected={ctrl.engineState.disconnected}
            musicServices={ctrl.musicServices}
            activeDeviceName={ctrl.effectiveDeviceName}
            spatialBadge={ctrl.effectiveSpatialBadge}
            volumePercent={ctrl.effectiveVolumePercent}
            muted={ctrl.effectiveVolumeMuted}
            barPercent={ctrl.pillSize.vol}
            micMuted={ctrl.effectiveMicMuted}
            lyricsDelayMs={ctrl.lyricsDelayMs}
            media={ctrl.effectiveMedia}
            lyrics={ctrl.engineState.lyrics}
            showLyrics={ctrl.showCardLyrics}
            lyricsDrawerOpen={ctrl.lyricsCardOpen}
            instant={ctrl.isInstant}
            notchVisible={ctrl.isNotchVisible}
            preset={ctrl.notchSnapshot.preset}
            expanded={ctrl.notchSnapshot.expanded}
            onToggleExpand={ctrl.toggleExpand}
            onToggleMute={ctrl.toggleMute}
            onSetVolume={ctrl.setVolume}
            onSwitchDevice={ctrl.switchDevice}
            onToggleMicMute={ctrl.toggleMicMute}
            onToggleLyrics={ctrl.toggleLyricsCard}
            onMediaAction={(action) => ctrl.sendMediaAction(action)}
            onOpenMusicService={ctrl.openMusicService}
            onOpenSettings={ctrl.openSettings}
            onHibernate={ctrl.hibernate}
            isBottom={ctrl.isBottomAnchor}
            widget={ctrl.widget}
          />
        {:else if ctrl.notchSnapshot && ctrl.isExpanded}
          <ExpandedNotch
            playback={ctrl.engineState.playback}
            settings={ctrl.engineState.settings}
            disconnected={ctrl.engineState.disconnected}
            musicServices={ctrl.musicServices}
            activeDeviceName={ctrl.effectiveDeviceName}
            spatialBadge={ctrl.effectiveSpatialBadge}
            volumePercent={ctrl.effectiveVolumePercent}
            muted={ctrl.effectiveVolumeMuted}
            barPercent={ctrl.pillSize.vol}
            micMuted={ctrl.effectiveMicMuted}
            lyricsDelayMs={ctrl.lyricsDelayMs}
            media={ctrl.effectiveMedia}
            lyrics={ctrl.engineState.lyrics}
            showLyrics={ctrl.showCardLyrics}
            lyricsDrawerOpen={ctrl.lyricsCardOpen}
            instant={ctrl.isInstant}
            notchVisible={ctrl.isNotchVisible}
            preset={ctrl.notchSnapshot.preset}
            expanded={ctrl.notchSnapshot.expanded}
            onToggleExpand={ctrl.toggleExpand}
            onToggleMute={ctrl.toggleMute}
            onSetVolume={ctrl.setVolume}
            onSwitchDevice={ctrl.switchDevice}
            onToggleMicMute={ctrl.toggleMicMute}
            onToggleLyrics={ctrl.toggleLyricsCard}
            onMediaAction={(action) => ctrl.sendMediaAction(action)}
            onOpenMusicService={ctrl.openMusicService}
            onOpenSettings={ctrl.openSettings}
            isBottom={ctrl.isBottomAnchor}
            widget={ctrl.widget}
          />
        {/if}
      </div>
    </div>
  </div>

  <!-- Crisp 1px SVG rim driven by spring (w, h, r) with vector-effect="non-scaling-stroke" -->
  <svg
    class="rim-svg"
    width={CANVAS_W}
    height={CANVAS_H}
    viewBox={`0 0 ${CANVAS_W} ${CANVAS_H}`}
    style:clip-path={ctrl.surfaceShape.rimKind === 'path' ? ctrl.surfaceShape.clipPath : undefined}
    aria-hidden="true"
  >
    {#if ctrl.surfaceShape.rimKind === 'path'}
      <path
        d={ctrl.surfaceShape.rimPathD}
        fill="none"
        stroke={ctrl.isLifted && ctrl.dragEffects.carryGlow ? ctrl.activeAccentCss : ctrl.rimStroke}
        stroke-width="2"
        vector-effect="non-scaling-stroke"
      />
      {#if ctrl.placedFlash && !ctrl.isInstant}
        {#key ctrl.placedFlashKey}
          <path
            class="rim-flash"
            d={ctrl.surfaceShape.rimPathD}
            fill="none"
            stroke="var(--ice-active-accent, var(--ice-accent))"
            stroke-width="2.5"
            vector-effect="non-scaling-stroke"
          />
        {/key}
      {/if}
    {:else}
      <rect
        x={ctrl.insetLeft + 0.5}
        y={ctrl.surfaceShape.insetTop + 0.5}
        width={Math.max(1, ctrl.pillSize.w - 1)}
        height={Math.max(1, ctrl.pillSize.h - 1)}
        rx={ctrl.pillSize.r}
        ry={ctrl.pillSize.r}
        fill="none"
        stroke={ctrl.isLifted && ctrl.dragEffects.carryGlow ? ctrl.activeAccentCss : ctrl.rimStroke}
        stroke-width="1"
        vector-effect="non-scaling-stroke"
      />
      {#if ctrl.placedFlash && !ctrl.isInstant}
        {#key ctrl.placedFlashKey}
          <rect
            class="rim-flash"
            x={ctrl.insetLeft + 0.5}
            y={ctrl.surfaceShape.insetTop + 0.5}
            width={Math.max(1, ctrl.pillSize.w - 1)}
            height={Math.max(1, ctrl.pillSize.h - 1)}
            rx={ctrl.pillSize.r}
            ry={ctrl.pillSize.r}
            fill="none"
            stroke="var(--ice-active-accent, var(--ice-accent))"
            stroke-width="1.5"
            vector-effect="non-scaling-stroke"
          />
        {/key}
      {/if}
    {/if}
  </svg>
  {#if ctrl.engineState.settings?.appearance.experiments.beatRim && !ctrl.isExpanded && !ctrl.isInstant && !ctrl.isHibernating && !ctrl.isLifted && ctrl.activeView.kind === 'track'}
    <BeatRim shape={ctrl.surfaceShape} insetLeft={ctrl.insetLeft} pill={ctrl.pillSize} accent={ctrl.activeAccentCss} />
  {/if}
  {#if ctrl.placedFlash && ctrl.dragEffects.landingRipple && !ctrl.isInstant}
    {#key ctrl.placedFlashKey}
      <LandingRipple shape={ctrl.surfaceShape} pill={ctrl.pillSize} accent={ctrl.activeAccentCss} />
    {/key}
  {/if}
  </div>
</div>

<style>
  .stage {
    position: relative;
    margin: 0 auto;
    background: transparent;
  }

  .stage.hidden {
    opacity: 0;
    pointer-events: none;
  }

  .track-with-lyrics {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .track-with-lyrics.bottom-anchor {
    flex-direction: column-reverse;
  }

  .track-head {
    width: 100%;
    height: 46px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  /* D-27 hibernate: a plain notch has no shadow or rim and a fixed black fill (`--ice-hibernate-bg`). */
  .stage.plain .pill-shadow,
  .stage.plain .rim-svg {
    opacity: 0;
  }

  .pill-surface.plain {
    background: var(--ice-hibernate-bg);
  }

  .stage.idle-hidden .pill-shadow,
  .stage.idle-hidden .pill-surface,
  .stage.idle-hidden .rim-svg {
    opacity: 0;
  }

  .pill-shadow {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    background: transparent;
    box-shadow: var(--ice-drop-shadow);
    pointer-events: none;
  }

  .pill-clip {
    position: absolute;
    inset: 0;
    background: transparent;
  }

  .pill-surface {
    position: absolute;
    inset: 0;
    background: var(--ice-bg);
    --ice-lyric-glow: color-mix(in srgb, var(--ice-active-accent, var(--ice-accent)) var(--ice-lyric-glow-mix), transparent);
  }

  .pill-slot {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    padding: 0 12px;
    overflow: hidden;
  }

  .pill-slot.nook.compact {
    padding-top: 2px;
  }

  .pill-slot.nook.compact.bottom-anchor {
    padding-top: 0;
    padding-bottom: 2px;
  }

  .rim-svg {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  /* Held / carried: the pill grows a little with the lift spring (compositor-only). */
  .notch-motion {
    position: absolute;
    inset: 0;
    transform-origin: 50% 18px;
    transform: scale(calc(1 + 0.045 * var(--lift, 0)));
  }

  .lift-shadow {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    background: transparent;
    box-shadow: var(--ice-lift-shadow);
    pointer-events: none;
  }

  /* The ring sits right on the pill's edge (1.5 px spread, no blur); the halo is two soft layers of the same colour,
     nudged away from the docked edge (`--glow-dy`). */
  .carry-glow {
    --glow-reach: 48px;
    --glow-dy: 3px;
    --glow-fade-dir: to bottom;
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    pointer-events: none;
    box-shadow: 0 0 0 1.5px color-mix(in srgb, var(--glow-c) 90%, transparent);
  }

  .carry-glow.bottom-anchor {
    --glow-dy: -3px;
    --glow-fade-dir: to top;
  }

  /* Holds the halo with `--glow-reach` to spare on every side; its mask is clear at the window edge on the docked side
     and full a little inside the pill (hidden there anyway), so that side of the halo fades out instead of clipping. */
  .carry-halo-fade {
    position: absolute;
    inset: calc(-1 * var(--glow-reach));
    mask-image: linear-gradient(
      var(--glow-fade-dir),
      transparent calc(var(--glow-reach) - var(--glow-room, 0px) + 1px),
      #000 calc(var(--glow-reach) + 14px)
    );
  }

  .carry-halo {
    position: absolute;
    inset: var(--glow-reach);
    border-radius: var(--glow-r);
    box-shadow:
      0 var(--glow-dy) 10px 2px color-mix(in srgb, var(--glow-c) 62%, transparent),
      0 calc(var(--glow-dy) * 2) 26px 7px color-mix(in srgb, var(--glow-c) 30%, transparent);
    animation: notch-carry-breathe 1.8s ease-in-out infinite alternate;
  }

  .carry-glow.still .carry-halo {
    animation: none;
  }

  @keyframes notch-carry-breathe {
    from {
      opacity: 0.55;
    }
    to {
      opacity: 1;
    }
  }

  .notch-motion.nook {
    transform-origin: 50% 0%;
  }

  .notch-motion.bottom-anchor {
    transform-origin: 50% calc(100% - 24px);
  }

  .notch-motion.bottom-anchor.nook {
    transform-origin: 50% 100%;
  }

  /* Landing impact (owner, 2026-11-26): squashed flat against the edge it docks on, one rebound, settle. The origin is
     that edge (see the transform-origin rules above), so it reads as hitting the bezel / taskbar. */
  .notch-motion.place-settle {
    animation: notch-place-settle 440ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }

  /* "Jelly landing" (Settings → Notch → Display): softer and wobblier, two rebounds. */
  .notch-motion.place-settle.jelly {
    animation: notch-place-jelly 640ms cubic-bezier(0.33, 1, 0.68, 1) both;
  }

  .rim-flash {
    filter: drop-shadow(0 0 3px var(--ice-active-accent, var(--ice-accent)));
    animation: notch-rim-flash 440ms ease-out both;
  }

  @keyframes notch-place-settle {
    0% {
      transform: scaleX(1.06) scaleY(0.86);
    }
    32% {
      transform: scaleX(0.975) scaleY(1.05);
    }
    60% {
      transform: scaleX(1.01) scaleY(0.985);
    }
    100% {
      transform: scaleX(1) scaleY(1);
    }
  }

  @keyframes notch-place-jelly {
    0% {
      transform: scaleX(1.1) scaleY(0.8);
    }
    20% {
      transform: scaleX(0.94) scaleY(1.1);
    }
    40% {
      transform: scaleX(1.04) scaleY(0.95);
    }
    60% {
      transform: scaleX(0.985) scaleY(1.025);
    }
    80% {
      transform: scaleX(1.005) scaleY(0.995);
    }
    100% {
      transform: scaleX(1) scaleY(1);
    }
  }

  @keyframes notch-rim-flash {
    0% {
      opacity: 1;
    }
    35% {
      opacity: 0.85;
    }
    100% {
      opacity: 0;
    }
  }
</style>
