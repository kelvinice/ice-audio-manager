<script lang="ts">
  import { onMount } from 'svelte';
  import type { MusicServiceId } from '../../lib/ipc/engine';
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

  // Fetched on open only (it also probes whether Windows can read kanji for Romaji); no polling.
  onMount(() => {
    void client.refreshLyricsStorage();
  });

  function patchLyrics(patch: {
    showOnPill?: boolean;
    showOnCard?: boolean;
    wordHighlight?: boolean;
    romaji?: boolean;
  }): void {
    void client.apply({
      type: 'setLyricsDisplay',
      showOnPill: patch.showOnPill ?? null,
      showOnCard: patch.showOnCard ?? null,
      wordHighlight: patch.wordHighlight ?? null,
      romaji: patch.romaji ?? null,
    });
  }

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

  <Section
    title="Synced lyrics"
    description="Attach live synced lyrics to the bottom of the music pill and expanded card. Works offline with local .lrc files (Storage) or optionally with lrclib.net (Privacy)."
  >
    <SettingCard
      icon="lyrics"
      title="Show synced lyrics"
      description="Extend the notch downward with live lyrics whenever a matching track is playing."
    >
      {#snippet control()}
        <Toggle
          label="Show synced lyrics"
          checked={settings.lyrics.enabled}
          onchange={(enabled) => client.apply({ type: 'setLyricsEnabled', enabled })}
        />
      {/snippet}
    </SettingCard>

    <SettingCard
      icon="music"
      title="Extend compact music pill"
      description="Show the current lyric line right beneath the compact Now Playing pill."
    >
      {#snippet control()}
        <Toggle
          label="Extend compact music pill with lyrics"
          checked={settings.lyrics.showOnPill}
          disabled={!settings.lyrics.enabled}
          onchange={(showOnPill) => patchLyrics({ showOnPill })}
        />
      {/snippet}
    </SettingCard>

    <SettingCard
      icon="page"
      title="Extend expanded card"
      description="Show a 3-line synced lyrics drawer below the expanded card. Click any line to seek."
    >
      {#snippet control()}
        <Toggle
          label="Extend expanded card with lyrics"
          checked={settings.lyrics.showOnCard}
          disabled={!settings.lyrics.enabled}
          onchange={(showOnCard) => patchLyrics({ showOnCard })}
        />
      {/snippet}
    </SettingCard>

    <SettingCard
      icon="bolt"
      title="Active word highlight"
      description="Highlights each word as it's sung on word-synced (Enhanced LRC) lyrics. Standard line-synced tracks highlight the whole line unless Estimated word highlight is turned on in Experimental."
    >
      {#snippet control()}
        <Toggle
          label="Highlight active word in lyrics"
          checked={settings.lyrics.wordHighlight}
          disabled={!settings.lyrics.enabled}
          onchange={(wordHighlight) => patchLyrics({ wordHighlight })}
        />
      {/snippet}
    </SettingCard>

    <SettingCard
      icon="globe"
      title="Romaji subtitles for Japanese songs"
      description="An offline Romaji line under Japanese lyrics, read by Windows' Japanese language data. Translations paired in a .lrc file always show."
    >
      {#snippet control()}
        <Toggle
          label="Show Romaji subtitles for Japanese songs"
          checked={settings.lyrics.romaji}
          disabled={!settings.lyrics.enabled}
          onchange={(romaji) => patchLyrics({ romaji })}
        />
      {/snippet}
    </SettingCard>

    {#if settings.lyrics.enabled && settings.lyrics.romaji && client.lyricsStorage && !client.lyricsStorage.romajiReadsKanji}
      <InfoBar
        tone="info"
        title="Lines with kanji get no Romaji on this PC."
        text="Windows needs its Japanese language data to read kanji. Add Japanese under Language & region (free, from Microsoft; you don't have to switch your display language). Romaji appears from the next song, no restart needed."
      >
        {#snippet actions()}
          <button class="btn accent" type="button" onclick={() => void client.openLanguageSettings()}>
            Open language settings
          </button>
          <button class="btn" type="button" onclick={() => void client.refreshLyricsStorage()}>Check again</button>
        {/snippet}
      </InfoBar>
    {/if}

    {#if settings.lyrics.enabled && !settings.lyrics.onlineLookup}
      <InfoBar
        tone="info"
        title="Online lyrics lookup is off."
        text="Only local .lrc files (Storage) and cached tracks will appear until you opt in to lrclib.net on the Privacy page."
      />
    {/if}
  </Section>
{/if}
