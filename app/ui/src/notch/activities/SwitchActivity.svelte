<script lang="ts">
  import { shortDeviceName, type ActivityView } from '../../lib/notch/scheduler';
  import DeviceIcon from './DeviceIcon.svelte';

  type SwitchView = Extract<ActivityView, { kind: 'switch' }>;

  interface Props {
    activity: SwitchView;
    instant: boolean;
  }

  let { activity, instant }: Props = $props();

  const caption = $derived.by(() => {
    if (activity.relinked) return 'Re-linked';
    if (activity.mergedPlug) return 'Connected';
    return null;
  });
</script>

{#key activity.switchSeq}
  <div class="switch-row">
    {#if !instant}
      <span class="rim-sweep" aria-hidden="true"></span>
    {/if}

    <span class="icon-badge" class:instant>
      <DeviceIcon kind="auto" formFactor={activity.device.formFactor} size={15} />
    </span>

    <div class="name-stage">
      {#if !instant && activity.previousDevice}
        <span class="prev-name">{shortDeviceName(activity.previousDevice)}</span>
      {/if}
      <span class="curr-name" class:instant class:with-caption={caption !== null}>
        {activity.device.name}
      </span>
      {#if caption}
        <span class="sub-caption">{caption}</span>
      {/if}
    </div>

    {#if activity.spatialBadge}
      <span class="spatial-pill">{activity.spatialBadge}</span>
    {/if}
  </div>
{/key}

<style>
  .switch-row {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }

  .rim-sweep {
    position: absolute;
    top: -1px;
    left: 0;
    width: 45%;
    height: 2px;
    border-radius: var(--ice-radius-pill);
    background: linear-gradient(90deg, transparent, var(--ice-accent-strong), transparent);
    pointer-events: none;
    animation: rim-glide 420ms cubic-bezier(0.22, 1, 0.36, 1) forwards;
  }

  .icon-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-accent-soft);
    border: 1px solid var(--ice-border);
    color: var(--ice-accent-strong);
    flex-shrink: 0;
  }

  .icon-badge:not(.instant) {
    animation: icon-pop 240ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .name-stage {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .prev-name {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    color: var(--ice-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
    animation: slide-out 220ms cubic-bezier(0.4, 0, 1, 1) forwards;
  }

  .curr-name {
    color: var(--ice-fg);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .curr-name.with-caption {
    font-size: 11px;
    line-height: 1.2;
  }

  .curr-name:not(.instant) {
    animation: slide-in 260ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .sub-caption {
    font-size: 10px;
    line-height: 1.15;
    color: var(--ice-accent);
  }

  .spatial-pill {
    padding: 2px 7px;
    border-radius: var(--ice-radius-pill);
    font-size: 10px;
    font-weight: 600;
    background: var(--ice-accent-soft);
    color: var(--ice-accent-strong);
    border: 1px solid var(--ice-border);
    flex-shrink: 0;
  }

  @keyframes slide-out {
    from {
      opacity: 1;
      transform: translateX(0);
      filter: blur(0);
    }
    to {
      opacity: 0;
      transform: translateX(-14px);
      filter: blur(3px);
    }
  }

  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateX(14px);
      filter: blur(2px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
      filter: blur(0);
    }
  }

  @keyframes icon-pop {
    from {
      transform: scale(0.88);
    }
    to {
      transform: scale(1);
    }
  }

  @keyframes rim-glide {
    from {
      opacity: 0.9;
      transform: translateX(-60%);
    }
    to {
      opacity: 0;
      transform: translateX(260%);
    }
  }
</style>
