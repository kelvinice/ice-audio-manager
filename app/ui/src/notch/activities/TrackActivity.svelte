<script lang="ts">
  import type { MediaAction } from '../../lib/ipc/engine';
  import type { ActivityView } from '../../lib/notch/scheduler';
  import AlbumArtTile from './AlbumArtTile.svelte';
  import DeviceIcon from './DeviceIcon.svelte';

  type TrackView = Extract<ActivityView, { kind: 'track' }>;

  interface Props {
    activity: TrackView;
    instant: boolean;
    onMediaAction: (action: MediaAction) => void;
  }

  let { activity, instant, onMediaAction }: Props = $props();

  const isPlaying = $derived(activity.media.status === 'playing');
  const hasPrevious = $derived(
    !instant && Boolean(activity.previousTitle || activity.previousArtist),
  );
</script>

<div class="track-row">
  <AlbumArtTile
    art={activity.media.art}
    previousArt={activity.previousArt}
    crossfadeKey={activity.trackSeq}
    size={32}
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

  <!-- Fixed right transport slot: never moves or remounts across track changes (D-13.1) -->
  <div class="transport-slot" role="group" aria-label="Playback controls">
    <button
      class="transport-btn"
      type="button"
      aria-label="Previous track"
      onclick={() => onMediaAction('previous')}
    >
      <DeviceIcon kind="prev" size={11} />
    </button>
    <button
      class="transport-btn primary-btn"
      type="button"
      aria-label={isPlaying ? 'Pause' : 'Play'}
      onclick={() => onMediaAction('playPause')}
    >
      <DeviceIcon kind={isPlaying ? 'pause' : 'play'} size={12} />
    </button>
    <button
      class="transport-btn"
      type="button"
      aria-label="Next track"
      onclick={() => onMediaAction('next')}
    >
      <DeviceIcon kind="next" size={11} />
    </button>
  </div>
</div>

<style>
  .track-row {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    white-space: nowrap;
  }

  .label-slot {
    position: relative;
    flex: 1;
    min-width: 0;
    height: 32px;
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
    line-height: 1.2;
    color: var(--ice-fg);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-artist {
    font-size: 10px;
    font-weight: 500;
    line-height: 1.2;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .transport-slot {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .transport-btn {
    appearance: none;
    width: 24px;
    height: 24px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--ice-radius-pill);
    border: 1px solid var(--ice-border);
    background: var(--ice-surface);
    color: var(--ice-fg);
    cursor: pointer;
  }

  .transport-btn.primary-btn {
    width: 26px;
    height: 26px;
    border-color: var(--ice-active-accent, var(--ice-accent));
    background: var(--ice-accent-soft);
    color: var(--ice-accent-strong);
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
