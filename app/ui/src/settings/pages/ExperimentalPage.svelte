<script lang="ts">
  // Graduated on 2026-11-26 (owner): estimated word highlight → Activities, wavy progress line → Appearance, jelly
  // landing → Notch → Display. What stays here is still being tuned.
  import type { DragEffects, Experiments } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import InfoBar from '../controls/InfoBar.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';
  import WidgetEditor from './WidgetEditor.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();
  const settings = $derived(client.engine?.settings);
  const dragFixed = $derived(settings?.notchPlacement === 'fixed');

  type DragEffectKey = Exclude<keyof DragEffects, 'jellyLanding'>;
  const dragEffectCards: { key: DragEffectKey; title: string; description: string }[] = [
    {
      key: 'landingRipple',
      title: 'Landing ripple',
      description: 'Rings in the song’s colour rise off the notch’s edge and spread out where it lands.',
    },
    {
      key: 'carryGlow',
      title: 'Glow while carrying',
      description: 'The notch glows in the song’s colour while you hold or drag it.',
    },
  ];

  type ExperimentKey = keyof Experiments;
  const experimentCards: { key: ExperimentKey; title: string; description: string }[] = [
    {
      key: 'beatRim',
      title: 'Beat rim',
      description: 'The music pill’s edge lights up in the song’s colour on each bass beat.',
    },
    {
      key: 'coverTilt',
      title: 'Cover tilt',
      description: 'The cover on the open card leans toward your pointer, with a soft glare.',
    },
  ];

  function setExperiment(key: ExperimentKey, on: boolean): void {
    void client.apply({
      type: 'setExperiments',
      beatRim: key === 'beatRim' ? on : null,
      coverTilt: key === 'coverTilt' ? on : null,
    });
  }

  function setDragEffect(key: DragEffectKey, on: boolean): void {
    void client.apply({
      type: 'setDragEffects',
      landingRipple: key === 'landingRipple' ? on : null,
      jellyLanding: null,
      carryGlow: key === 'carryGlow' ? on : null,
    });
  }
</script>

{#if settings}
  <InfoBar
    tone="info"
    title="Experimental features"
    text="These options are off by default. They may behave unevenly or change in future updates."
  />

  <WidgetEditor {client} />

  <Section
    title="Notch looks"
    description="Extra motion on the animated notch. Off when Windows animation effects are off."
  >
    {#each experimentCards as card (card.key)}
      <SettingCard icon="flask" title={card.title} description={card.description}>
        {#snippet control()}
          <Toggle
            label={card.title}
            checked={settings.appearance.experiments[card.key]}
            onchange={(on) => setExperiment(card.key, on)}
          />
        {/snippet}
      </SettingCard>
    {/each}
  </Section>

  <Section
    title="Drag & drop"
    description="Extra effects when you pick the notch up and drop it somewhere else (the animated notch only). Off when Windows animation effects are off."
  >
    {#each dragEffectCards as card (card.key)}
      <SettingCard icon="flask" title={card.title} description={card.description} disabled={dragFixed}>
        {#snippet control()}
          <Toggle
            label={card.title}
            checked={settings.appearance.dragEffects[card.key]}
            disabled={dragFixed}
            onchange={(on) => setDragEffect(card.key, on)}
          />
        {/snippet}
      </SettingCard>
    {/each}
    {#if dragFixed}
      <InfoBar
        tone="info"
        title="Dragging is off."
        text="Choose Drag & place under Notch → Display → Movement to move the notch with the mouse."
      />
    {/if}
  </Section>
{/if}
