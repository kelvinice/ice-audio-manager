<script lang="ts">
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
{/if}
