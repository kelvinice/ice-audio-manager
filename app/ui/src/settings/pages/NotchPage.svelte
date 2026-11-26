<script lang="ts">
  import type { IdleStyle, NotchAnchor, NotchPlacement, NotchStyle, NowPlayingPill } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import ChoiceGroup from '../controls/ChoiceGroup.svelte';
  import InfoBar from '../controls/InfoBar.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';
  import MonitorMap from '../previews/MonitorMap.svelte';
  import NotchPreview from '../previews/NotchPreview.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();

  const settings = $derived(client.engine?.settings ?? null);
  const monitors = $derived(client.notch?.monitors ?? []);
  const pinnedMissing = $derived(
    Boolean(
      settings?.pinnedMonitor &&
        !monitors.some((m) => m.displayPath.toLowerCase() === settings.pinnedMonitor?.toLowerCase()),
    ),
  );

  const notchStyles: { value: NotchStyle; label: string; description: string; badge?: string }[] = [
    {
      value: 'nook',
      label: 'Edge Nook',
      badge: 'Default',
      description: 'Docked flush to the top bezel or taskbar edge with curved ears and a side-by-side media & lyrics layout.',
    },
    {
      value: 'floating',
      label: 'Floating',
      description: 'Classic floating pill inset from the screen edge, with stacked controls and lyrics.',
    },
  ];

  const idleStyles: { value: IdleStyle; label: string; description: string; badge?: string }[] = [
    { value: 'mini_pill', label: 'Mini pill', badge: 'Default', description: 'The device and volume, small and readable.' },
    { value: 'dot', label: 'Dot', description: 'Just a dot. It grows when something happens.' },
    { value: 'hidden', label: 'Hidden', description: 'Nothing until the volume, device or track changes.' },
  ];

  // D-20: top & bottom positions. Twin: `NotchAnchor::label` in crates/ice-core/src/placement.rs.
  const anchors: { value: NotchAnchor; label: string; description: string; badge?: string }[] = [
    { value: 'top_left', label: 'Top left', description: 'Out of the way of centred title bars and tabs.' },
    { value: 'top_center', label: 'Top centre', badge: 'Default', description: 'Where a hardware notch would be.' },
    { value: 'top_right', label: 'Top right', description: 'Near the window controls, away from the middle.' },
    {
      value: 'bottom_left',
      label: 'Bottom left',
      description: 'Above the taskbar, expanding upwards.',
    },
    {
      value: 'bottom_center',
      label: 'Bottom centre',
      description: 'Centred above the taskbar, expanding upwards.',
    },
    {
      value: 'bottom_right',
      label: 'Bottom right',
      description: 'Above the notification area, expanding upwards.',
    },
  ];

  const placements: { value: NotchPlacement; label: string; description: string; badge?: string }[] = [
    {
      value: 'drag_snap',
      label: 'Drag & place',
      badge: 'Default',
      description: 'Drag the pill or card anywhere along the top or bottom edge on any display. Press Esc to cancel.',
    },
    {
      value: 'fixed',
      label: 'Fixed position',
      description: 'Only move the notch from Settings or the tray menu.',
    },
  ];

  const nowPlayingModes: { value: NowPlayingPill; label: string; description: string; badge?: string }[] = [
    {
      value: 'while_playing',
      label: 'While playing',
      badge: 'Default',
      description: 'Keeps the cover, waveform and lyrics visible while music plays. Pause to shrink back to idle.',
    },
    {
      value: 'brief',
      label: 'Brief, then small',
      description: 'Pops up with the cover and waveform for a few seconds when the track changes, then shrinks to idle.',
    },
    { value: 'off', label: 'Off', description: 'No music pop-up. The idle notch just shows the current track.' },
  ];
</script>

{#if settings}
  <Section
    title="Notch style"
    description="Choose whether the notch floats as a pill or docks flush to the top or bottom edge of the screen."
  >
    <ChoiceGroup
      label="Notch style"
      value={settings.appearance.notchStyle}
      choices={notchStyles}
      onchange={(style) => client.apply({ type: 'setNotchStyle', style })}
    >
      {#snippet preview(choice)}
        <NotchPreview theme={settings.appearance.theme} mode="volume" notchStyle={choice.value} />
      {/snippet}
    </ChoiceGroup>
  </Section>

  <Section title="When nothing is happening" description="What the notch shows between activities.">
    <ChoiceGroup
      label="Idle notch"
      value={settings.appearance.idleStyle}
      choices={idleStyles}
      onchange={(style) => client.apply({ type: 'setIdleStyle', style })}
    >
      {#snippet preview(choice)}
        <NotchPreview theme={settings.appearance.theme} mode={choice.value} notchStyle={settings.appearance.notchStyle} />
      {/snippet}
    </ChoiceGroup>
  </Section>

  <Section title="Now playing" description="How the notch shows music playing from Spotify, YouTube Music or any media player.">
    <ChoiceGroup
      label="Now playing pill"
      value={settings.appearance.nowPlaying}
      choices={nowPlayingModes}
      onchange={(mode) => client.apply({ type: 'setNowPlayingPill', mode })}
    />
  </Section>

  <Section title="Open card" description="The card that opens when you point at or click the notch.">
    <SettingCard
      icon="volume"
      title="Volume as a button"
      description="Always show the volume as a small speaker button that opens the slider, so a stray click on the card can’t change the volume. Scrolling over the notch still changes it."
    >
      {#snippet control()}
        <Toggle
          label="Volume as a button"
          checked={settings.appearance.volumeButton}
          onchange={(enabled) => client.apply({ type: 'setVolumeButton', enabled })}
        />
      {/snippet}
    </SettingCard>
  </Section>

  <Section
    title="Display"
    description="Pick the display, screen position, or drag the notch anywhere along the top or bottom edge. It follows Windows’ scaling on each screen and stays clear of the taskbar."
  >
    <ChoiceGroup
      label="Position"
      value={settings.notchXRatio == null ? settings.notchAnchor : null}
      choices={anchors}
      columns={3}
      onchange={(anchor) => client.apply({ type: 'setNotchAnchor', anchor })}
    />
    {#if settings.notchXRatio != null}
      <InfoBar
        tone="info"
        title={`Custom position (${Math.round(settings.notchXRatio * 100)}%)`}
        text="Positioned along the monitor edge via drag (clamped to screen bounds so it never clips). Pick any preset above to snap back to Left, Centre, or Right."
      />
    {/if}
    <ChoiceGroup
      label="Movement"
      value={settings.notchPlacement}
      choices={placements}
      onchange={(placement) => client.apply({ type: 'setNotchPlacement', placement })}
    />
    <SettingCard
      icon="bolt"
      title="Jelly landing"
      description="A softer, wobblier landing that bounces twice before it settles when you drop the notch. Off when Windows animation effects are off."
      disabled={settings.notchPlacement === 'fixed'}
    >
      {#snippet control()}
        <Toggle
          label="Jelly landing"
          checked={settings.appearance.dragEffects.jellyLanding}
          disabled={settings.notchPlacement === 'fixed'}
          onchange={(on) =>
            client.apply({ type: 'setDragEffects', landingRipple: null, jellyLanding: on, carryGlow: null })}
        />
      {/snippet}
    </SettingCard>
    {#if pinnedMissing}
      <InfoBar
        tone="info"
        title="Your chosen display isn’t connected."
        text="The notch is on the main display for now and moves back when it returns."
      />
    {/if}
    {#if monitors.length}
      <div class="map">
        <MonitorMap
          {monitors}
          pinned={settings.pinnedMonitor}
          onpick={(displayPath) => client.apply({ type: 'setPinnedMonitor', displayPath })}
        />
      </div>
    {/if}
  </Section>
{/if}

<style>
  .map {
    border-radius: var(--ice-set-radius);
    border: 1px solid var(--ice-set-card-border);
    background: var(--ice-set-card);
  }
</style>
