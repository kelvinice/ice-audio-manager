<script lang="ts">
  import type { ActivityView } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';

  type PlugView = Extract<ActivityView, { kind: 'plug' }>;

  interface Props {
    activity: PlugView;
    instant: boolean;
  }

  let { activity, instant }: Props = $props();

  const arrived = $derived(activity.direction === 'arrived');
</script>

{#key activity.plugSeq}
  <div class="plug-row" class:instant class:left={!arrived}>
    <span class="plug-icon" class:arrived class:left={!arrived}>
      <DeviceIcon
        kind={arrived ? 'auto' : 'plug-out'}
        formFactor={activity.device.formFactor}
        size={15}
      />
    </span>

    <span class="dev-name">{activity.device.name}</span>

    <span class="status-pill" class:arrived class:left={!arrived}>
      {arrived ? 'Connected' : 'Removed'}
    </span>
  </div>
{/key}

<style>
  .plug-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }

  .plug-row:not(.instant) {
    animation: plug-in 240ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .plug-icon {
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

  .plug-icon.arrived {
    color: var(--ice-ok);
    border-color: var(--ice-ok);
  }

  .plug-icon.left {
    color: var(--ice-warn);
    border-color: var(--ice-warn);
  }

  .dev-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ice-fg);
    font-weight: 600;
  }

  .status-pill {
    padding: 2px 7px;
    border-radius: var(--ice-radius-pill);
    font-size: 10px;
    font-weight: 600;
    background: var(--ice-surface);
    border: 1px solid var(--ice-border);
    flex-shrink: 0;
  }

  .status-pill.arrived {
    color: var(--ice-ok);
    border-color: var(--ice-ok);
  }

  .status-pill.left {
    color: var(--ice-warn);
    border-color: var(--ice-warn);
  }

  @keyframes plug-in {
    from {
      opacity: 0;
      transform: translateX(10px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }
</style>
