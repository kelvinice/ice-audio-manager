<script lang="ts">
  import type { ActivityView } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';

  type MicView = Extract<ActivityView, { kind: 'mic' }>;

  interface Props {
    activity: MicView;
    instant: boolean;
  }

  let { activity, instant }: Props = $props();
</script>

{#key instant ? 0 : activity.entranceSeq}
  <div class="mic-row" class:instant>
    <span class="mic-icon" class:muted={activity.muted} class:live={!activity.muted}>
      <DeviceIcon kind={activity.muted ? 'mic-muted' : 'mic'} size={15} />
    </span>
    <span class="mic-label" class:muted={activity.muted}>
      {activity.muted ? 'Mic Muted' : 'Mic Live'}
    </span>
    <span class="mic-dot" class:muted={activity.muted} aria-hidden="true"></span>
  </div>
{/key}

<style>
  .mic-row {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }

  .mic-row:not(.instant) {
    animation: mic-enter 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .mic-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-surface);
    border: 1px solid var(--ice-border);
    flex-shrink: 0;
  }

  .mic-icon.muted {
    color: var(--ice-danger);
    border-color: var(--ice-danger);
  }

  .mic-icon.live {
    color: var(--ice-ok);
    border-color: var(--ice-ok);
  }

  .mic-label {
    flex: 1;
    color: var(--ice-fg);
  }

  .mic-label.muted {
    color: var(--ice-danger);
  }

  .mic-dot {
    width: 7px;
    height: 7px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-ok);
    flex-shrink: 0;
  }

  .mic-dot.muted {
    background: var(--ice-danger);
  }

  @keyframes mic-enter {
    0% {
      transform: scale(0.95);
      opacity: 0.7;
    }
    55% {
      transform: scale(1.03);
      opacity: 1;
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }
</style>
