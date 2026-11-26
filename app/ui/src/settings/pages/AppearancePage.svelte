<script lang="ts">
  import type { SpringPreset, ThemeKind } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import ChoiceGroup from '../controls/ChoiceGroup.svelte';
  import InfoBar from '../controls/InfoBar.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';
  import NotchPreview from '../previews/NotchPreview.svelte';
  import SpringDemo from '../previews/SpringDemo.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();

  const settings = $derived(client.engine?.settings ?? null);
  const reduceMotion = $derived(client.notch?.systemReduceMotion ?? false);

  const themes: { value: ThemeKind; label: string; description: string; badge?: string }[] = [
    { value: 'ice', label: 'Ice', badge: 'Default', description: 'Frosted glass with a cool blue glow.' },
    { value: 'ice_light', label: 'Ice Light', description: 'Frosted snow glass with deep ocean accents.' },
    { value: 'midnight', label: 'Midnight', description: 'True black with a quiet white rim.' },
    { value: 'fluent', label: 'Fluent', description: 'Windows 11’s neutral greys.' },
    { value: 'minimal', label: 'Minimal', description: 'A solid surface. No glow.' },
  ];

  const presets: { value: SpringPreset; label: string; description: string; badge?: string }[] = [
    { value: 'bouncy', label: 'Bouncy', badge: 'Default', description: 'A playful overshoot.' },
    { value: 'smooth', label: 'Smooth', description: 'Gentle, with no bounce.' },
    { value: 'snappy', label: 'Snappy', description: 'Quick and crisp.' },
    { value: 'off', label: 'Off', description: 'No motion. Changes appear at once.' },
  ];
</script>

{#if settings}
  <Section title="Theme" description="The notch’s colours. Album art still tints it while music plays.">
    <ChoiceGroup
      label="Theme"
      value={settings.appearance.theme}
      choices={themes}
      onchange={(theme) => client.apply({ type: 'setTheme', theme })}
    >
      {#snippet preview(choice)}
        <NotchPreview theme={choice.value} mode="volume" notchStyle={settings.appearance.notchStyle} />
      {/snippet}
    </ChoiceGroup>
  </Section>

  <Section title="Animation" description="How the notch moves. Point at a choice to see it.">
    {#if reduceMotion}
      <InfoBar
        tone="info"
        title="Animation effects are off in Windows."
        text="The notch follows that setting, so it won’t animate until you turn them back on in Settings → Accessibility → Visual effects."
      />
    {/if}
    <ChoiceGroup
      label="Animation"
      value={settings.appearance.preset}
      choices={presets}
      onchange={(preset) => client.apply({ type: 'setPreset', preset })}
    >
      {#snippet preview(choice)}
        <div class="themed" data-theme={settings.appearance.theme}>
          <SpringDemo preset={choice.value} {reduceMotion} />
        </div>
      {/snippet}
    </ChoiceGroup>
  </Section>

  <Section title="Background" description="Ambient lava-lamp motion behind the music pill and expanded card.">
    <SettingCard
      icon="color"
      title="Moving background"
      description="Slowly drift the album-art wash across the music pill and expanded card while music plays, paced by the beat. Pauses automatically when playback stops or animation effects are off."
    >
      {#snippet control()}
        <Toggle
          label="Moving background"
          checked={settings.appearance.movingBackdrop}
          disabled={reduceMotion || settings.appearance.preset === 'off' || settings.appearance.theme === 'minimal'}
          onchange={(enabled) => client.apply({ type: 'setMovingBackdrop', enabled })}
        />
      {/snippet}
    </SettingCard>
  </Section>

  <Section title="Song progress" description="How the seek bar on the open card shows the time already played.">
    <SettingCard
      icon="music"
      title="Wavy progress line"
      description="While music plays, the played part of the seek bar becomes a gently moving wave in the song’s colour. It flattens when you pause, and stays still when animation effects are off."
    >
      {#snippet control()}
        <Toggle
          label="Wavy progress line"
          checked={settings.appearance.wavyProgress}
          onchange={(enabled) => client.apply({ type: 'setWavyProgress', enabled })}
        />
      {/snippet}
    </SettingCard>
  </Section>
{/if}

<style>
  .themed {
    height: 100%;
  }
</style>
