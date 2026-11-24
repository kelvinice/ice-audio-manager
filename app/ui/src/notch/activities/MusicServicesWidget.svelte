<script lang="ts">
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import DeviceIcon from './DeviceIcon.svelte';

  interface Props {
    services: MusicServiceItem[];
    /** `launch`: labelled chips in the empty card. `rail`: icon-only buttons in the side rail. */
    variant: 'launch' | 'rail';
    onOpenService: (service: MusicServiceId) => void;
  }

  let { services, variant, onOpenService }: Props = $props();
</script>

{#if services.length > 0}
  <div class="services" class:rail={variant === 'rail'} role="group" aria-label="Music services">
    {#each services as svc (svc.id)}
      <button
        class="svc"
        class:active={svc.active}
        type="button"
        title={`Open ${svc.label}`}
        aria-label={`Open ${svc.label}`}
        onclick={() => onOpenService(svc.id)}
      >
        <DeviceIcon kind={svc.id === 'spotify' ? 'spotify' : 'youtube-music'} size={variant === 'rail' ? 14 : 12} />
        {#if variant === 'launch'}
          <span>{svc.label}</span>
        {/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .services {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .services.rail {
    flex-direction: column;
    gap: 2px;
  }

  .svc {
    appearance: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    height: 24px;
    padding: 0 10px 0 8px;
    border: 1px solid var(--ice-control-border);
    border-radius: var(--ice-radius-pill);
    background: var(--ice-control-bg);
    color: var(--ice-fg);
    font-family: inherit;
    font-size: 10.5px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .svc:hover {
    background: var(--ice-control-bg-hover);
  }

  .rail .svc {
    width: 24px;
    padding: 0;
    border-color: transparent;
    background: transparent;
    color: var(--ice-fg-muted);
  }

  .rail .svc:hover {
    background: var(--ice-control-bg-hover);
    color: var(--ice-fg);
  }

  .svc.active,
  .rail .svc.active {
    color: var(--ice-accent-strong);
  }

  .services:not(.rail) .svc.active {
    border-color: var(--ice-accent);
    background: var(--ice-control-bg-active);
    color: var(--ice-fg);
  }
</style>
