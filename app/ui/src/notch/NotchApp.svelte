<script lang="ts">
  import { onMount } from 'svelte';
  import { createNotchController } from '../lib/notch/controller.svelte';
  import { BOTTOM_PAD, CANVAS_H, CANVAS_W } from '../lib/notch/geometry';
  import ExpandedNotch from './activities/ExpandedNotch.svelte';
  import IdlePill from './activities/IdlePill.svelte';
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
    class="pill-shadow"
    style:top={ctrl.isBottomAnchor ? undefined : `${ctrl.surfaceShape.topPad}px`}
    style:bottom={ctrl.isBottomAnchor ? `${BOTTOM_PAD}px` : undefined}
    style:width={`${ctrl.pillSize.w}px`}
    style:height={`${ctrl.pillSize.h}px`}
    style:border-radius={ctrl.surfaceShape.shadowRadiusCss}
    aria-hidden="true"
  ></div>
  <div
    class="pill-clip"
    style:clip-path={ctrl.surfaceShape.clipPath}
    onpointerdown={ctrl.onPointerDown}
    onpointerenter={ctrl.onPointerEnter}
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
        bottomPad={BOTTOM_PAD}
      />
      {/if}
      <div
        class="pill-slot"
        class:nook={ctrl.notchStyle === 'nook'}
        class:compact={!ctrl.isExpanded}
        style:top={ctrl.isBottomAnchor ? undefined : `${ctrl.surfaceShape.topPad}px`}
        style:bottom={ctrl.isBottomAnchor ? `${BOTTOM_PAD}px` : undefined}
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
        stroke={ctrl.rimStroke}
        stroke-width="2"
        vector-effect="non-scaling-stroke"
      />
    {:else}
      <rect
        x={ctrl.insetLeft + 0.5}
        y={ctrl.surfaceShape.insetTop + 0.5}
        width={Math.max(1, ctrl.pillSize.w - 1)}
        height={Math.max(1, ctrl.pillSize.h - 1)}
        rx={ctrl.pillSize.r}
        ry={ctrl.pillSize.r}
        fill="none"
        stroke={ctrl.rimStroke}
        stroke-width="1"
        vector-effect="non-scaling-stroke"
      />
    {/if}
  </svg>
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

  .rim-svg {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
</style>
