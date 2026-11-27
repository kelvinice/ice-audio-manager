<script lang="ts">
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import { fitRailServices } from '../../lib/notch/backdrop';
  import { MAX_WIDGET_BUTTONS, type WidgetBinding } from '../../lib/notch/widget';
  import DeviceIcon from './DeviceIcon.svelte';
  import MusicServicesWidget from './MusicServicesWidget.svelte';
  import WidgetButtons from './WidgetButtons.svelte';

  interface Props {
    /** Pinned = the notch stays expanded (`NotchSnapshot.expanded`). */
    pinned: boolean;
    micMuted: boolean;
    /** Services sit here only while media plays; the empty card shows them as labelled chips instead. */
    services: MusicServiceItem[];
    showServices: boolean;
    onTogglePin: () => void;
    onToggleMicMute: () => void;
    onOpenService: (service: MusicServiceId) => void;
    onOpenSettings: () => void;
    /** Experimental widget: when set, its buttons replace pin, mic and Settings. */
    widget?: WidgetBinding | null;
  }

  let {
    pinned,
    micMuted,
    services,
    showServices,
    onTogglePin,
    onToggleMicMute,
    onOpenService,
    onOpenSettings,
    widget = null,
  }: Props = $props();

  // Five 24 px buttons fill the 140 px rail: pin, mic and Settings leave two for services; the widget leaves the rest.
  const railServices = $derived(
    showServices ? fitRailServices(services, widget ? MAX_WIDGET_BUTTONS - widget.buttons.length : 2) : [],
  );
</script>

<nav class="rail" aria-label="Notch actions">
  {#if widget}
    <!-- Experimental widget: the user's buttons (five at most), then the services while they still fit. -->
    <div class="group">
      <WidgetButtons {...widget} variant="rail" />
    </div>
    {#if railServices.length > 0}
      <MusicServicesWidget services={railServices} variant="rail" {onOpenService} />
    {/if}
  {:else}
  <div class="group">
    <button
      class="rail-btn"
      class:on={pinned}
      type="button"
      title={pinned ? 'Unpin notch' : 'Pin notch open'}
      aria-label={pinned ? 'Unpin notch' : 'Pin notch open'}
      aria-pressed={pinned}
      onclick={onTogglePin}
    >
      <DeviceIcon kind="pin" size={13} />
    </button>
    <button
      class="rail-btn"
      class:danger={micMuted}
      type="button"
      title={micMuted ? 'Microphone is off: turn on' : 'Microphone is on: turn off'}
      aria-label={micMuted ? 'Unmute microphone' : 'Mute microphone'}
      aria-pressed={micMuted}
      onclick={onToggleMicMute}
    >
      <DeviceIcon kind={micMuted ? 'mic-muted' : 'mic'} size={13} />
    </button>
  </div>

  {#if railServices.length > 0}
    <MusicServicesWidget services={railServices} variant="rail" {onOpenService} />
  {/if}

  <!-- Five buttons at most (pin, mic, 2 services, settings): 5 × 24 px fits the 140 px rail, so a third service shows
       only while it plays (`fitRailServices`). The lyrics toggle lives in the media card's transport row (2026-11-24:
       a sixth button pushed Settings off the bottom). -->
  <button class="rail-btn" type="button" title="Settings" aria-label="Open Settings" onclick={onOpenSettings}>
    <DeviceIcon kind="settings" size={13} />
  </button>
  {/if}
</nav>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    gap: 4px;
    width: 24px;
    padding-left: 9px;
    border-left: 1px solid var(--ice-control-border);
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .rail-btn {
    appearance: none;
    width: 24px;
    height: 24px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid transparent;
    border-radius: var(--ice-radius-pill);
    background: transparent;
    color: var(--ice-fg-muted);
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .rail-btn:hover {
    background: var(--ice-control-bg-hover);
    color: var(--ice-fg);
  }

  .rail-btn.on {
    border-color: var(--ice-accent);
    background: var(--ice-control-bg-active);
    color: var(--ice-accent-strong);
  }

  .rail-btn.danger {
    background: color-mix(in srgb, var(--ice-danger) 16%, transparent);
    color: var(--ice-danger);
  }
</style>
