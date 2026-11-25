<script lang="ts">
  import type { ActivityView } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';

  type VolumeView = Extract<ActivityView, { kind: 'volume' }>;

  interface Props {
    activity: VolumeView;
    barPercent: number;
    instant: boolean;
  }

  let { activity, barPercent, instant }: Props = $props();

  const iconKind = $derived.by(() => {
    if (activity.channel === 'mic') {
      return activity.muted ? 'mic-muted' : 'mic';
    }
    return activity.muted ? 'speaker-muted' : 'speaker';
  });

  const clampedScale = $derived(Math.max(0, Math.min(1, barPercent / 100)));
</script>

{#key instant ? 0 : activity.bumpSeq}
  <div
    class="volume-row"
    class:bump-low={!instant && activity.boundBump === -1}
    class:bump-high={!instant && activity.boundBump === 1}
  >
    {#key instant ? 0 : activity.entrySeq}
      <span class="icon-wrap" class:pulse={!instant} class:muted={activity.muted}>
        <DeviceIcon kind={iconKind} size={15} />
      </span>
    {/key}

    {#if activity.channel === 'mic'}
      <span class="chan-tag">Mic</span>
    {/if}

    <!-- D-28 C1: the value leads, so the pop-up reads as "volume 13 %", never as a song's progress bar. -->
    <span class="pct" class:muted={activity.muted}>
      {activity.muted ? 'Mute' : `${activity.percent}%`}
    </span>

    <div class="bar-track" class:mic-channel={activity.channel === 'mic'}>
      <div
        class="bar-fill"
        class:muted={activity.muted}
        style:transform={`scaleX(${clampedScale.toFixed(4)})`}
      ></div>
    </div>
  </div>
{/key}

<style>
  .volume-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }

  .volume-row.bump-low {
    animation: nudge-low 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .volume-row.bump-high {
    animation: nudge-high 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .icon-wrap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--ice-accent-strong);
    flex-shrink: 0;
  }

  .icon-wrap.muted {
    color: var(--ice-danger);
  }

  .icon-wrap.pulse {
    animation: micro-pulse 210ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .chan-tag {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-accent-soft);
    color: var(--ice-accent-strong);
    border: 1px solid var(--ice-border);
    flex-shrink: 0;
  }

  .bar-track {
    flex: 1;
    height: var(--ice-bar-height);
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
  }

  .bar-fill {
    width: 100%;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    transform-origin: left center;
    background: linear-gradient(90deg, var(--ice-accent), var(--ice-accent-strong));
  }

  .bar-fill.muted {
    background: var(--ice-danger);
  }

  .pct {
    min-width: 38px;
    text-align: left;
    font-family: var(--ice-mono);
    font-size: 14px;
    font-weight: 650;
    color: var(--ice-fg);
    flex-shrink: 0;
  }

  .pct.muted {
    color: var(--ice-danger);
  }

  @keyframes micro-pulse {
    0% {
      transform: scale(0.92);
    }
    55% {
      transform: scale(1.06);
    }
    100% {
      transform: scale(1);
    }
  }

  @keyframes nudge-low {
    0% {
      transform: translateX(0) scaleX(1);
    }
    40% {
      transform: translateX(-2px) scaleX(0.98);
    }
    100% {
      transform: translateX(0) scaleX(1);
    }
  }

  @keyframes nudge-high {
    0% {
      transform: translateX(0) scaleX(1);
    }
    40% {
      transform: translateX(2px) scaleX(1.02);
    }
    100% {
      transform: translateX(0) scaleX(1);
    }
  }
</style>
