<script lang="ts">
  import type { MusicServiceId } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();
  const settings = $derived(client.engine?.settings);

  // D-22: one switch per supported music service (owner, 2026-11-23: both on by default).
  const services: { id: MusicServiceId; key: 'spotify' | 'youtubeMusic'; title: string; description: string }[] = [
    {
      id: 'spotify',
      key: 'spotify',
      title: 'Spotify',
      description: 'A button that opens the Spotify app (or the web player if it isn’t installed).',
    },
    {
      id: 'youtube_music',
      key: 'youtubeMusic',
      title: 'YouTube Music',
      description: 'A button that opens the installed YouTube Music app, or music.youtube.com.',
    },
  ];
</script>

{#if settings}
  <Section
    title="Music services"
    description="Launch buttons in the notch: in the side rail while music plays, and in the card when nothing is playing. Any player still shows up in the notch either way."
  >
    {#each services as svc (svc.id)}
      <SettingCard icon="music" title={svc.title} description={svc.description}>
        {#snippet control()}
          <Toggle
            label={`Show ${svc.title} in the notch`}
            checked={settings.musicServices[svc.key]}
            onchange={(enabled) => client.apply({ type: 'setMusicService', service: svc.id, enabled })}
          />
        {/snippet}
      </SettingCard>
    {/each}
  </Section>
{/if}
