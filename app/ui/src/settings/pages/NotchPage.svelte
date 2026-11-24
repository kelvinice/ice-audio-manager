<script lang="ts">
  import type { IdleStyle, NotchStyle, NowPlayingPill } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import ChoiceGroup from '../controls/ChoiceGroup.svelte';
  import InfoBar from '../controls/InfoBar.svelte';
  import Section from '../controls/Section.svelte';
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
      value: 'floating',
      label: 'Floating',
      badge: 'Default',
      description: 'Classic floating pill below the top bezel, with stacked controls and lyrics.',
    },
    {
      value: 'nook',
      label: 'Edge Nook',
      description: 'Docked flush to the top bezel with curved ears and a side-by-side media & lyrics layout.',
    },
  ];

  const idleStyles: { value: IdleStyle; label: string; description: string; badge?: string }[] = [
    { value: 'mini_pill', label: 'Mini pill', badge: 'Default', description: 'The device and volume, small and readable.' },
    { value: 'dot', label: 'Dot', description: 'Just a dot. It grows when something happens.' },
    { value: 'hidden', label: 'Hidden', description: 'Nothing until the volume, device or track changes.' },
  ];

  const nowPlayingModes: { value: NowPlayingPill; label: string; description: string; badge?: string }[] = [
    {
      value: 'brief',
      label: 'Brief, then small',
      badge: 'Default',
      description: 'Pops up with the cover and controls for a few seconds, then shrinks to the idle notch.',
    },
    {
      value: 'while_playing',
      label: 'While playing',
      description: 'Stays as the cover-and-controls pill as long as music plays. Pause to shrink it.',
    },
    { value: 'off', label: 'Off', description: 'No pop-up. The idle notch just shows the new title.' },
  ];
</script>

{#if settings}
  <Section
    title="Notch style"
    description="Choose whether the notch floats as a pill or docks flush to the top edge of the screen."
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

  <Section title="When the song changes" description="How the notch shows a new track from Spotify, YouTube Music or any player.">
    <ChoiceGroup
      label="Now playing pill"
      value={settings.appearance.nowPlaying}
      choices={nowPlayingModes}
      onchange={(mode) => client.apply({ type: 'setNowPlayingPill', mode })}
    />
  </Section>

  <Section
    title="Display"
    description="The notch sits at the top centre of the display you pick. It follows Windows’ scaling on each one."
  >
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
