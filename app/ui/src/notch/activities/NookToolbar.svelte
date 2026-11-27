<script lang="ts">
  // Edge Nook header actions: music-service launchers on the left, then one capsule of notch toggles
  // (lyrics, mic, hibernate, pin, settings), so launchers and toggles no longer read as one flat row.
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import DeviceIcon from './DeviceIcon.svelte';
  import MusicServicesWidget from './MusicServicesWidget.svelte';
  import WidgetButtons from './WidgetButtons.svelte';
  import type { WidgetBinding } from '../../lib/notch/widget';

  interface Props {
    services: MusicServiceItem[];
    showServices: boolean;
    showLyricsToggle: boolean;
    lyricsOn: boolean;
    lyricsAvailable: boolean;
    micMuted: boolean;
    pinned: boolean;
    onOpenService: (service: MusicServiceId) => void;
    onToggleLyrics: () => void;
    onToggleMicMute: () => void;
    onHibernate: () => void;
    onTogglePin: () => void;
    onOpenSettings: () => void;
    /** Experimental widget: when set, its buttons fill the capsule instead of the built-in toggles. */
    widget?: WidgetBinding | null;
  }

  let {
    services,
    showServices,
    showLyricsToggle,
    lyricsOn,
    lyricsAvailable,
    micMuted,
    pinned,
    onOpenService,
    onToggleLyrics,
    onToggleMicMute,
    onHibernate,
    onTogglePin,
    onOpenSettings,
    widget = null,
  }: Props = $props();
</script>

<nav class="toolbar" aria-label="Edge Nook quick actions">
  {#if showServices && services.length > 0}
    <div class="launchers">
      <MusicServicesWidget {services} variant="rail" {onOpenService} />
    </div>
  {/if}
  <div class="capsule" role="group" aria-label="Notch controls">
    {#if widget}
      <WidgetButtons {...widget} variant="toolbar" />
    {:else}
    {#if showLyricsToggle}
      <button
        class="tb"
        class:on={lyricsOn && lyricsAvailable}
        type="button"
        title={lyricsAvailable ? (lyricsOn ? 'Hide lyrics' : 'Show lyrics') : 'No lyrics for this song'}
        aria-label={lyricsOn ? 'Hide lyrics' : 'Show lyrics'}
        aria-pressed={lyricsOn && lyricsAvailable}
        disabled={!lyricsAvailable}
        onclick={onToggleLyrics}
      >
        <DeviceIcon kind="lyrics" size={12} />
      </button>
    {/if}
    <button
      class="tb"
      class:danger={micMuted}
      type="button"
      title={micMuted ? 'Microphone is off: turn on' : 'Microphone is on: turn off'}
      aria-label={micMuted ? 'Unmute microphone' : 'Mute microphone'}
      aria-pressed={micMuted}
      onclick={onToggleMicMute}
    >
      <DeviceIcon kind={micMuted ? 'mic-muted' : 'mic'} size={12} />
    </button>
    <button
      class="tb"
      type="button"
      title="Hibernate: a plain notch for meetings and screen sharing. Wake it from the tray icon or Settings."
      aria-label="Hibernate the notch"
      onclick={onHibernate}
    >
      <DeviceIcon kind="moon" size={12} />
    </button>
    <button
      class="tb"
      class:on={pinned}
      type="button"
      title={pinned ? 'Unpin notch' : 'Pin notch open'}
      aria-label={pinned ? 'Unpin notch' : 'Pin notch open'}
      aria-pressed={pinned}
      onclick={onTogglePin}
    >
      <DeviceIcon kind="pin" size={12} />
    </button>
    <button class="tb" type="button" title="Settings" aria-label="Open Settings" onclick={onOpenSettings}>
      <DeviceIcon kind="settings" size={12} />
    </button>
    {/if}
  </div>
</nav>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .launchers :global(.services.rail) {
    flex-direction: row;
    gap: 2px;
  }

  .capsule {
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 2px;
    border: 1px solid var(--ice-control-border);
    border-radius: var(--ice-radius-pill);
    background: var(--ice-control-bg);
  }

  .tb {
    appearance: none;
    width: 22px;
    height: 22px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--ice-radius-pill);
    background: transparent;
    color: var(--ice-fg-muted);
    cursor: pointer;
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }

  .tb:hover {
    background: var(--ice-control-bg-hover);
    color: var(--ice-fg);
  }

  .tb:focus-visible {
    outline: 2px solid var(--ice-accent);
    outline-offset: 1px;
  }

  .tb.on {
    background: var(--ice-control-bg-active);
    color: var(--ice-accent-strong);
  }

  .tb.danger {
    background: color-mix(in srgb, var(--ice-danger) 18%, transparent);
    color: var(--ice-danger);
  }

  .tb:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .tb:disabled:hover {
    background: transparent;
    color: var(--ice-fg-muted);
  }
</style>
