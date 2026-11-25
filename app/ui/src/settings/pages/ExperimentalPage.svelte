<script lang="ts">
  import type { DragEffects } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import InfoBar from '../controls/InfoBar.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();
  const settings = $derived(client.engine?.settings);
  const lyricsWordDisabled = $derived(!settings?.lyrics.enabled || !settings?.lyrics.wordHighlight);
  const dragFixed = $derived(settings?.notchPlacement === 'fixed');

  type DragEffectKey = keyof DragEffects;
  const dragEffectCards: { key: DragEffectKey; title: string; description: string }[] = [
    {
      key: 'landingRipple',
      title: 'Landing ripple',
      description: 'A ring in the song’s colour spreads out from the edge where the notch lands.',
    },
    {
      key: 'jellyLanding',
      title: 'Jelly landing',
      description: 'A softer, wobblier landing that bounces twice before it settles.',
    },
    {
      key: 'carryGlow',
      title: 'Glow while carrying',
      description: 'The notch glows in the song’s colour while you hold or drag it.',
    },
  ];

  function setDragEffect(key: DragEffectKey, on: boolean): void {
    void client.apply({
      type: 'setDragEffects',
      landingRipple: key === 'landingRipple' ? on : null,
      jellyLanding: key === 'jellyLanding' ? on : null,
      carryGlow: key === 'carryGlow' ? on : null,
    });
  }
</script>

{#if settings}
  <InfoBar
    tone="info"
    title="Experimental features"
    text="These options are off by default. They may behave unevenly across tracks or change in future updates."
  />

  <Section
    title="Lyrics"
    description="Optional lyric behaviours that rely on estimates and may not match every song."
  >
    <SettingCard
      icon="flask"
      title="Estimated word highlight (line-synced lyrics)"
      description="Estimates word-by-word timing for standard line-synced lyrics (such as most lrclib.net tracks) that don't include real word timestamps. May drift on held notes or tempo changes; true Enhanced LRC files always highlight word-by-word."
      disabled={lyricsWordDisabled}
    >
      {#snippet control()}
        <Toggle
          label="Estimated word highlight for line-synced lyrics"
          checked={settings.lyrics.estimatedWordSync}
          disabled={lyricsWordDisabled}
          onchange={(enabled) => client.apply({ type: 'setLyricsEstimatedWordSync', enabled })}
        />
      {/snippet}
    </SettingCard>

    {#if lyricsWordDisabled}
      <InfoBar
        tone="info"
        title="Turn on synced lyrics and word highlight first."
        text="Enable both Show synced lyrics and Active word highlight on the Activities page for this option to take effect."
      />
    {/if}
  </Section>

  <Section title="Song progress" description="How the seek bar on the open card shows the time already played.">
    <SettingCard
      icon="flask"
      title="Wavy progress line"
      description="While music plays, the played part of the seek bar becomes a gently moving wave in the song’s colour. It flattens when you pause. Still when Windows animation effects are off."
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
