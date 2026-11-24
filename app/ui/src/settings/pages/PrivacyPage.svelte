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
  const settings = $derived(client.engine?.settings ?? null);
  const anyNetworkEnabled = $derived(Boolean(settings?.lyrics.onlineLookup));

  async function toggleOnlineLyrics(enabled: boolean): Promise<void> {
    await client.apply({ type: 'setLyricsOnlineLookup', enabled });
    if (enabled && settings && !settings.lyrics.enabled) {
      await client.apply({ type: 'setLyricsEnabled', enabled: true });
    }
  }
</script>

{#if settings}
  <Section
    title="Network features"
    description="Ice Audio Manager stays offline by default. Every network feature is listed here with the exact host and data it sends."
  >
    <SettingCard
      icon="shield"
      title="Keep all network features off"
      description={anyNetworkEnabled
        ? 'One network feature is currently enabled below. Turn everything off in one click.'
        : 'All optional network features are currently off. The app makes zero network requests.'}
    >
      {#snippet control()}
        <button
          class="btn"
          type="button"
          disabled={!anyNetworkEnabled}
          onclick={() => client.apply({ type: 'disableAllNetworkFeatures' })}
        >
          Turn all off
        </button>
      {/snippet}
    </SettingCard>

    <SettingCard
      icon="globe"
      title="Synced lyrics lookup (lrclib.net)"
      description="Looks up timestamped lyrics from https://lrclib.net when no local .lrc override or cached match exists. Sends only track title, artist, album, and duration — no account, cookies, or tracking."
    >
      {#snippet control()}
        <Toggle
          label="Allow synced lyrics lookup on lrclib.net"
          checked={settings.lyrics.onlineLookup}
          onchange={(enabled) => void toggleOnlineLyrics(enabled)}
        />
      {/snippet}
    </SettingCard>

    {#if settings.lyrics.onlineLookup && !settings.lyrics.enabled}
      <InfoBar
        tone="info"
        title="Lyrics display is currently turned off."
        text="Turn on Synced lyrics in Activities to show lyrics under the music pill and expanded card."
      />
    {/if}

    <InfoBar
      tone="info"
      title="Offline-first by design"
      text="Local .lrc files (Storage) and Japanese Romaji conversion run 100 % locally on your PC. Private Spotify or YouTube Music account APIs are never queried."
    />
  </Section>
{/if}
