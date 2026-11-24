<script lang="ts">
  import type { SpringPreset } from '../../lib/ipc/types';
  import type { ActivityView } from '../../lib/notch/scheduler';
  import { toMsNumber } from '../../lib/notch/scheduler';
  import { WIDE_BAR_COUNT } from '../../lib/notch/visualizer';
  import AlbumArtTile from './AlbumArtTile.svelte';
  import VisualizerBars from './VisualizerBars.svelte';

  type TrackView = Extract<ActivityView, { kind: 'track' }>;

  interface Props {
    activity: TrackView;
    instant: boolean;
    preset: SpringPreset;
    notchVisible: boolean;
  }

  let { activity, instant, preset, notchVisible }: Props = $props();

  const isPlaying = $derived(activity.media.status === 'playing');
  const hasPrevious = $derived(
    !instant && Boolean(activity.previousTitle || activity.previousArtist),
  );

  // No transport buttons here: hovering expands the notch before they can be reached (owner, 2026-11-24).
  // Instead a waveform scrubber: live bars, lit in the accent up to the played position.
  const durationMs = $derived(toMsNumber(activity.media.durationMs));
  // GSMTC sends the position only now and then; interpolate from the last report while playing.
  let anchor = $state({ posMs: 0, atMs: 0 });
  let nowMs = $state(0);
  $effect(() => {
    const posMs = toMsNumber(activity.media.positionMs) ?? 0;
    void activity.media.status;
    const at = performance.now();
    anchor = { posMs, atMs: at };
    nowMs = at;
  });
  const elapsedMs = $derived(anchor.posMs + (isPlaying ? Math.max(0, nowMs - anchor.atMs) : 0));
  const progress = $derived(durationMs ? Math.max(0, Math.min(1, elapsedMs / durationMs)) : null);

  // Re-render only when the next bar lights up (every duration / 14, ~17 s for a 4-minute song), not every second.
  $effect(() => {
    if (!isPlaying || !durationMs || !notchVisible) return;
    const step = durationMs / WIDE_BAR_COUNT;
    const nextBoundary = (Math.floor(elapsedMs / step) + 1) * step;
    const delay = Math.max(250, Math.min(60_000, nextBoundary - elapsedMs + 30));
    const id = window.setTimeout(() => {
      nowMs = performance.now();
    }, delay);
    return () => window.clearTimeout(id);
  });
</script>

<div class="track-row">
  <AlbumArtTile
    art={activity.media.art}
    previousArt={activity.previousArt}
    crossfadeKey={activity.trackSeq}
    size={34}
    radius="16px"
    {instant}
  />

  <div class="label-slot">
    {#key activity.trackSeq}
      {#if hasPrevious}
        <div class="label-stack outgoing" aria-hidden="true">
          <span class="track-title">{activity.previousTitle || 'Unknown track'}</span>
          <span class="track-artist">{activity.previousArtist || 'Unknown artist'}</span>
        </div>
      {/if}
      <div class="label-stack incoming" class:animate-in={!instant}>
        <span class="track-title">{activity.media.title || 'Unknown track'}</span>
        <span class="track-artist">{activity.media.artist || 'Unknown artist'}</span>
      </div>
    {/key}
  </div>

  <!-- Fixed right slot: never moves or remounts across track changes (D-13.1). -->
  <div class="wave-slot" class:paused={!isPlaying} title={isPlaying ? undefined : 'Paused'}>
    <VisualizerBars active={isPlaying && notchVisible} {preset} {instant} variant="wide" {progress} />
  </div>
</div>

<style>
  .track-row {
    width: calc(100% + 8px);
    height: 100%;
    margin-left: -6px;
    margin-right: -2px;
    display: flex;
    align-items: center;
    gap: 9px;
    white-space: nowrap;
  }

  .label-slot {
    position: relative;
    flex: 1;
    min-width: 0;
    height: 34px;
    display: flex;
    align-items: center;
    overflow: hidden;
  }

  .label-stack {
    width: 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 1px;
  }

  .label-stack.outgoing {
    position: absolute;
    inset: 0;
    pointer-events: none;
    animation: track-label-out 220ms cubic-bezier(0.4, 0, 1, 1) forwards;
  }

  .label-stack.incoming.animate-in {
    animation: track-label-in 260ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .track-title {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 1.2;
    color: var(--ice-fg);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-artist {
    font-size: 10.5px;
    font-weight: 500;
    line-height: 1.2;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .wave-slot {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding-right: 4px;
    transition: opacity 240ms ease;
  }

  .wave-slot.paused {
    opacity: 0.6;
  }

  @keyframes track-label-out {
    from {
      opacity: 1;
      transform: translateY(0);
      filter: blur(0);
    }
    to {
      opacity: 0;
      transform: translateY(-8px);
      filter: blur(2px);
    }
  }

  @keyframes track-label-in {
    from {
      opacity: 0;
      transform: translateY(8px);
      filter: blur(2px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
      filter: blur(0);
    }
  }
</style>
