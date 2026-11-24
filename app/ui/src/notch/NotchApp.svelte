<script lang="ts">
  import { onMount } from 'svelte';
  import { createNotchController } from '../lib/notch/controller.svelte';
  import { CANVAS_H, CANVAS_W, TOP_PAD } from '../lib/notch/geometry';
  import ExpandedNotch from './activities/ExpandedNotch.svelte';
  import IdlePill from './activities/IdlePill.svelte';
  import LyricsExtension from './activities/LyricsExtension.svelte';
  import MicActivity from './activities/MicActivity.svelte';
  import NotchBackdrop from './activities/NotchBackdrop.svelte';
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
  style:width={`${CANVAS_W}px`}
  style:height={`${CANVAS_H}px`}
>
  <div
    class="pill-shadow"
    style:top={`${TOP_PAD}px`}
    style:width={`${ctrl.pillSize.w}px`}
    style:height={`${ctrl.pillSize.h}px`}
    style:border-radius={`${ctrl.pillSize.r}px`}
    aria-hidden="true"
  ></div>
  <div
    class="pill-clip"
    style:clip-path={ctrl.clipPath}
    onpointerenter={ctrl.onPointerEnter}
    onpointerleave={ctrl.onPointerLeave}
    onwheel={ctrl.onWheel}
    onauxclick={ctrl.onAuxClick}
    role="region"
    aria-label="Ice Audio Notch"
  >
    <div
      class="pill-surface"
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
      <NotchBackdrop
        art={ctrl.displayedMedia?.art}
        hasArt={ctrl.hasBackdropArt}
        instant={ctrl.isInstant}
        ambientState={ctrl.ambientMotionState}
        compactPill={!ctrl.isExpanded}
        boxW={ctrl.targetBox.w}
        boxH={ctrl.targetBox.h}
        topPad={TOP_PAD}
      />
      <div
        class="pill-slot"
        style:top={`${TOP_PAD}px`}
        style:width={`${ctrl.targetBox.w}px`}
        style:height={`${ctrl.targetBox.h}px`}
      >
        {#if ctrl.notchSnapshot && !ctrl.isExpanded}
          {#if ctrl.activeView.kind === 'volume'}
            <VolumeActivity activity={ctrl.activeView} barPercent={ctrl.pillSize.vol} instant={ctrl.isInstant} />
          {:else if ctrl.activeView.kind === 'switch'}
            <SwitchActivity activity={ctrl.activeView} instant={ctrl.isInstant} />
          {:else if ctrl.activeView.kind === 'plug'}
            <PlugActivity activity={ctrl.activeView} instant={ctrl.isInstant} />
          {:else if ctrl.activeView.kind === 'mic'}
            <MicActivity activity={ctrl.activeView} instant={ctrl.isInstant} />
          {:else if ctrl.activeView.kind === 'track'}
            <div class="track-with-lyrics">
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
                  media={ctrl.activeView.media}
                  mode="pill"
                  wordHighlight={ctrl.engineState.settings?.lyrics.wordHighlight ?? true}
                  showRomaji={ctrl.engineState.settings?.lyrics.romaji ?? true}
                  instant={ctrl.isInstant}
                  notchVisible={ctrl.isNotchVisible}
                />
              {/if}
            </div>
          {:else}
            <IdlePill
              deviceLabel={ctrl.effectiveDeviceShort}
              volumePercent={ctrl.effectiveVolumePercent}
              muted={ctrl.effectiveVolumeMuted}
              micMuted={ctrl.effectiveMicMuted}
              media={ctrl.engineState.media}
              idleMode={ctrl.idleMode}
              preset={ctrl.notchSnapshot.preset}
              instant={ctrl.isInstant}
              notchVisible={ctrl.isNotchVisible}
            />
          {/if}
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
            media={ctrl.engineState.media}
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
          />
        {/if}
      </div>
    </div>
  </div>

  <!-- Crisp 1px SVG rim driven by spring (w, h, r) with vector-effect="non-scaling-stroke" -->
  <svg class="rim-svg" width={CANVAS_W} height={CANVAS_H} viewBox={`0 0 ${CANVAS_W} ${CANVAS_H}`} aria-hidden="true">
    <rect
      x={ctrl.insetLeft + 0.5}
      y={TOP_PAD + 0.5}
      width={Math.max(1, ctrl.pillSize.w - 1)}
      height={Math.max(1, ctrl.pillSize.h - 1)}
      rx={ctrl.pillSize.r}
      ry={ctrl.pillSize.r}
      fill="none"
      stroke={ctrl.rimStroke}
      stroke-width="1"
      vector-effect="non-scaling-stroke"
    />
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

  .track-head {
    width: 100%;
    height: 46px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
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

  .rim-svg {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
</style>
