<script lang="ts">
  import type { PerformanceProfile, RoleSet } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import ChoiceGroup from '../controls/ChoiceGroup.svelte';
  import Section from '../controls/Section.svelte';
  import Select from '../controls/Select.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();
  const settings = $derived(client.engine?.settings);

  const profiles: { value: PerformanceProfile; label: string; description: string; badge?: string }[] = [
    {
      value: 'instant',
      label: 'Instant',
      badge: 'Default',
      description: 'The notch stays ready and opens with no delay. Uses the most memory.',
    },
    {
      value: 'balanced',
      label: 'Balanced',
      description: 'The notch starts when something happens. Less memory; its first animation can take a moment.',
    },
    {
      value: 'lite',
      label: 'Lite',
      description: 'No animated notch: a small native pill and the tray. The lightest option.',
    },
  ];

  // D-21.1 L1 (config `idle_linger_secs`, 5–600 s). Only Balanced suspends, so only Balanced shows the choice.
  const LINGER_PRESETS = [5, 15, 30, 60, 120, 300];

  function lingerLabel(secs: number): string {
    const base = secs < 60 ? `${secs} seconds` : secs === 60 ? '1 minute' : `${secs / 60} minutes`;
    return secs === 15 ? `${base} (default)` : base;
  }

  function lingerValue(secs: number): string {
    return String(secs);
  }

  /** The presets, plus the saved value if it was hand-edited to something else, so the select never lies. */
  function lingerOptions(current: number): { value: string; label: string }[] {
    const all = LINGER_PRESETS.includes(current) ? LINGER_PRESETS : [...LINGER_PRESETS, current].sort((a, b) => a - b);
    return all.map((secs) => ({ value: String(secs), label: lingerLabel(secs) }));
  }

  const roles: { value: RoleSet; label: string }[] = [
    { value: 'default', label: 'Default device' },
    { value: 'communications', label: 'Communications device' },
    { value: 'both', label: 'Both' },
  ];
</script>

<Section title="Startup">
  <SettingCard
    icon="power"
    title="Launch at sign-in"
    description={client.launchAtSignIn === null
      ? 'Windows manages startup for this version of the app (Settings → Apps → Startup).'
      : 'Start quietly in the tray when you sign in to Windows.'}
  >
    {#snippet control()}
      <Toggle
        label="Launch at sign-in"
        checked={client.launchAtSignIn === true}
        disabled={client.launchAtSignIn === null}
        onchange={(on) => client.setLaunchAtSignIn(on)}
      />
    {/snippet}
  </SettingCard>
</Section>

{#if settings}
  <Section title="Performance" description="How the notch trades speed for memory. The tray, shortcuts and switching work the same in every mode.">
    <ChoiceGroup
      label="Performance profile"
      value={settings.performanceProfile}
      choices={profiles}
      onchange={(profile) => client.apply({ type: 'setPerformanceProfile', profile })}
    />
    {#if settings.performanceProfile === 'balanced'}
      <SettingCard
        icon="power"
        title="Sleep the notch after"
        description="How long the notch stays ready when nothing happens, before the small native pill takes over."
      >
        {#snippet control()}
          <Select
            label="Sleep the notch after"
            value={lingerValue(settings.idleLingerSecs)}
            options={lingerOptions(settings.idleLingerSecs)}
            onchange={(secs) => client.apply({ type: 'setIdleLinger', secs: Number(secs) })}
          />
        {/snippet}
      </SettingCard>
    {/if}
  </Section>

  <Section title="Switching">
    <SettingCard
      icon="switch"
      title="When switching, change"
      description="Which Windows default a switch sets, unless a shortcut picks its own."
    >
      {#snippet control()}
        <Select
          label="When switching, change"
          value={settings.defaultRoles}
          options={roles}
          onchange={(roles) => client.apply({ type: 'setDefaultRoles', roles })}
        />
      {/snippet}
    </SettingCard>
    <SettingCard
      icon="view"
      title="Show disconnected devices"
      description="List unplugged devices, greyed out, in the tray. They can’t be selected."
    >
      {#snippet control()}
        <Toggle
          label="Show disconnected devices"
          checked={settings.showDisconnected}
          onchange={(enabled) => client.apply({ type: 'setShowDisconnected', enabled })}
        />
      {/snippet}
    </SettingCard>
  </Section>
{/if}
