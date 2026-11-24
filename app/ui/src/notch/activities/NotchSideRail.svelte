<script lang="ts">
  import type { MusicServiceId, MusicServiceItem } from '../../lib/notch/backdrop';
  import DeviceIcon from './DeviceIcon.svelte';
  import MusicServicesWidget from './MusicServicesWidget.svelte';

  interface Props {
    /** Pinned = the notch stays expanded (`NotchSnapshot.expanded`). */
    pinned: boolean;
    micMuted: boolean;
    /** Services sit here only while media plays; the empty card shows them as labelled chips instead. */
    services: MusicServiceItem[];
    showServices: boolean;
    showLyricsToggle?: boolean;
    lyricsOpen?: boolean;
    lyricsAvailable?: boolean;
    onTogglePin: () => void;
    onToggleMicMute: () => void;
    onToggleLyrics?: () => void;
    onOpenService: (service: MusicServiceId) => void;
    onOpenSettings: () => void;
  }

  let {
    pinned,
    micMuted,
    services,
    showServices,
    showLyricsToggle = false,
    lyricsOpen = false,
    lyricsAvailable = false,
    onTogglePin,
    onToggleMicMute,
    onToggleLyrics = () => {},
    onOpenService,
    onOpenSettings,
  }: Props = $props();
</script>

<nav class="rail" aria-label="Notch actions">
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

  {#if showServices && services.length > 0}
    <MusicServicesWidget {services} variant="rail" {onOpenService} />
  {/if}

  <div class="group">
    {#if showLyricsToggle}
      <button
        class="rail-btn"
        class:on={lyricsOpen && lyricsAvailable}
        class:dim={!lyricsAvailable}
        type="button"
        title={lyricsAvailable ? (lyricsOpen ? 'Hide lyrics' : 'Show lyrics') : 'No lyrics available'}
        aria-label={lyricsOpen ? 'Hide lyrics' : 'Show lyrics'}
        aria-pressed={lyricsOpen && lyricsAvailable}
        disabled={!lyricsAvailable}
        onclick={onToggleLyrics}
      >
        <DeviceIcon kind="lyrics" size={13} />
      </button>
    {/if}
    <button class="rail-btn" type="button" title="Settings" aria-label="Open Settings" onclick={onOpenSettings}>
      <DeviceIcon kind="settings" size={13} />
    </button>
  </div>
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

  .rail-btn.dim {
    opacity: 0.4;
    cursor: default;
  }
</style>
