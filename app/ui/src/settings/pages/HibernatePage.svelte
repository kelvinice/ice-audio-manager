<script lang="ts">
  // D-27 hibernate: its own page (owner, 2026-11-25). Hibernate is session state: the app always starts awake.
  import type { HibernateLook } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import ChoiceGroup from '../controls/ChoiceGroup.svelte';
  import InfoBar from '../controls/InfoBar.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();

  const settings = $derived(client.engine?.settings ?? null);

  const looks: { value: HibernateLook; label: string; description: string; badge?: string }[] = [
    {
      value: 'plain',
      label: 'Plain notch',
      badge: 'Default',
      description: 'A small black notch, like hardware. A red dot shows while your microphone is muted.',
    },
    {
      value: 'hidden',
      label: 'Hide the notch',
      description: 'Nothing on screen at all until you turn hibernate off.',
    },
  ];
</script>

{#if settings}
  <Section
    title="Hibernate"
    description="For meetings and screen sharing: the notch stops showing songs, covers, lyrics and pop-ups and stays out of your way."
  >
    <SettingCard
      icon="moon"
      title="Hibernate notch"
      description="Clicks go straight through the notch and volume keys still work. Turn it off here or from the tray icon's menu. The app always starts awake."
    >
      {#snippet control()}
        <Toggle
          label="Hibernate notch"
          checked={settings.appearance.hibernate}
          onchange={(enabled) => client.apply({ type: 'setHibernate', enabled })}
        />
      {/snippet}
    </SettingCard>
  </Section>

  <Section title="While hibernating" description="What the notch looks like while it hibernates.">
    <ChoiceGroup
      label="Hibernating notch"
      value={settings.appearance.hibernateLook}
      choices={looks}
      onchange={(look) => client.apply({ type: 'setHibernateLook', look })}
    />
    {#if settings.appearance.hibernate && settings.appearance.hibernateLook === 'hidden'}
      <InfoBar tone="info" title="The notch is hidden." text="Turn hibernate off above or from the tray icon's menu to bring it back." />
    {/if}
  </Section>

  <Section title="Screen sharing" description="Keep the notch out of what others see.">
    <SettingCard
      icon="shield"
      title="Hide from screen sharing"
      description="While hibernating, leave the notch out of screen shares and recordings (Teams, Zoom, Meet, OBS). You still see it on your screen."
    >
      {#snippet control()}
        <Toggle
          label="Hide from screen sharing"
          checked={settings.appearance.hibernateHideFromCapture}
          onchange={(enabled) => client.apply({ type: 'setHibernateHideFromCapture', enabled })}
        />
      {/snippet}
    </SettingCard>
  </Section>
{/if}
