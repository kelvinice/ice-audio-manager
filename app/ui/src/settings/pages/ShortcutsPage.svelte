<script lang="ts">
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import Icon from '../controls/Icon.svelte';
  import InfoBar from '../controls/InfoBar.svelte';
  import KeyCaps from '../controls/KeyCaps.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';
  import ShortcutCard from './ShortcutCard.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();

  const settings = $derived(client.engine?.settings ?? null);
  const conflicts = $derived(
    (client.engine?.notices ?? []).flatMap((n) => (n.type === 'hotkeyConflict' ? [n.keys] : [])),
  );

  let addingPair = $state(false);
  let addingBinding = $state(false);

  // F31 defaults (`ice_core::config::KnobGestures`); the modifiers aren't configurable yet.
  const gestures = [
    { chord: 'Ctrl', action: 'Switch to the next device in the notch' },
    { chord: 'Shift', action: 'Change the volume in 1% steps' },
    { chord: 'Alt', action: 'Change the microphone volume' },
  ];
</script>

<div class="trust">
  <Icon name="shield" size={18} />
  <p>
    Shortcuts are registered with Windows, which tells Ice Audio Manager only when one of them is pressed. It never
    sees what you type in other apps.
  </p>
</div>

{#each conflicts as keys (keys)}
  <InfoBar tone="warning" title={`${keys} is taken by another app.`} text="Pick a different shortcut, or close the other app and restart Ice Audio Manager." />
{/each}

{#if settings}
  <Section title="Toggle pairs" description="One shortcut flips between two devices, like speakers and headphones.">
    {#each settings.togglePairs as pair (pair.id)}
      <ShortcutCard {client} kind="pair" {pair} />
    {/each}
    {#if addingPair}
      <ShortcutCard {client} kind="pair" ondone={() => (addingPair = false)} />
    {:else}
      <button type="button" class="add" onclick={() => (addingPair = true)}>
        <Icon name="add" size={14} /> Add a toggle pair
      </button>
    {/if}
  </Section>

  <Section title="Device shortcuts" description="Switch straight to one device.">
    {#each settings.bindings as binding (binding.id)}
      <ShortcutCard {client} kind="binding" {binding} />
    {/each}
    {#if addingBinding}
      <ShortcutCard {client} kind="binding" ondone={() => (addingBinding = false)} />
    {:else}
      <button type="button" class="add" onclick={() => (addingBinding = true)}>
        <Icon name="add" size={14} /> Add a device shortcut
      </button>
    {/if}
  </Section>

  <Section title="Volume knob">
    <SettingCard
      icon="volume"
      title="Knob gestures"
      description="Hold a key while turning your keyboard’s volume knob. A plain turn still changes the volume."
    >
      {#snippet control()}
        <Toggle
          label="Knob gestures"
          checked={settings.knobGestures}
          onchange={(enabled) => client.apply({ type: 'setKnobGestures', enabled })}
        />
      {/snippet}
    </SettingCard>
    <ul class="gestures" class:off={!settings.knobGestures}>
      {#each gestures as g (g.chord)}
        <li><KeyCaps chord={`${g.chord}+Turn`} /><span>{g.action}</span></li>
      {/each}
    </ul>
  </Section>
{/if}

<style>
  .trust {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    margin-bottom: 24px;
    padding: 14px 16px;
    border-radius: var(--ice-set-radius);
    background: var(--ice-set-accent-soft);
    color: var(--ice-set-fg);
  }

  .trust :global(.icon) {
    color: var(--ice-set-accent);
    margin-top: 1px;
  }

  .trust p {
    margin: 0;
    font-size: 13px;
  }

  .add {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 48px;
    padding: 0 16px;
    border-radius: var(--ice-set-radius);
    border: 1px dashed var(--ice-set-control-border);
    background: none;
    color: var(--ice-set-accent);
    cursor: pointer;
    text-align: left;
  }

  .add:hover {
    background: var(--ice-set-card-hover);
  }

  .gestures {
    list-style: none;
    margin: 0;
    padding: 6px 16px 6px 56px;
    border-radius: var(--ice-set-radius);
    border: 1px solid var(--ice-set-card-border);
    background: var(--ice-set-card);
  }

  .gestures.off {
    opacity: 0.55;
  }

  .gestures li {
    display: grid;
    grid-template-columns: 150px 1fr;
    align-items: center;
    min-height: 40px;
    color: var(--ice-set-fg-muted);
  }
</style>
