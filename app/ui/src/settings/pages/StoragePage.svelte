<script lang="ts">
  import { onMount } from 'svelte';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import Section from '../controls/Section.svelte';
  import Select from '../controls/Select.svelte';
  import SettingCard from '../controls/SettingCard.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();
  const settings = $derived(client.engine?.settings ?? null);
  const stats = $derived(client.lyricsStorage);

  const cacheLimitOptions = [
    { value: '10', label: '10 MB' },
    { value: '25', label: '25 MB' },
    { value: '50', label: '50 MB (Default)' },
    { value: '100', label: '100 MB' },
    { value: '250', label: '250 MB' },
    { value: '500', label: '500 MB' },
  ];

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }

  const maxMb = $derived(settings?.lyrics.cacheMaxMb ?? stats?.cacheMaxMb ?? 50);
  const usedBytes = $derived(stats?.cacheBytes ?? 0);
  const usedPct = $derived(Math.max(0, Math.min(100, (usedBytes / (maxMb * 1024 * 1024)) * 100)));

  async function onLimitChange(raw: string): Promise<void> {
    const parsed = Number.parseInt(raw, 10);
    if (!Number.isFinite(parsed)) return;
    await client.apply({ type: 'setLyricsCacheMaxMb', maxMb: parsed });
    await client.refreshLyricsStorage();
  }

  onMount(() => {
    void client.refreshLyricsStorage();
  });
</script>

{#if settings}
  <Section
    title="Lyrics cache"
    description="Fetched lyrics and Romaji readings are cached locally so each track is only looked up once. Oldest tracks are removed automatically when the limit is reached."
  >
    <div class="usage-card">
      <div class="usage-head">
        <span class="usage-title">Managed cache usage</span>
        <span class="usage-meta">
          {formatBytes(usedBytes)} of {maxMb} MB · {stats?.cacheFiles ?? 0} cached {stats?.cacheFiles === 1 ? 'track' : 'tracks'}
        </span>
      </div>
      <div class="meter-track" aria-hidden="true">
        <div class="meter-fill" class:warn={usedPct >= 85} style:width={`${usedPct.toFixed(1)}%`}></div>
      </div>
    </div>

    <SettingCard
      icon="storage"
      title="Maximum cache size"
      description="Cap the disk space used by cached lyrics (5–500 MB). Lowering this limit trims the oldest cached tracks right away."
    >
      {#snippet control()}
        <Select
          label="Maximum lyrics cache size"
          value={String(maxMb)}
          options={cacheLimitOptions}
          onchange={(v) => void onLimitChange(v)}
        />
      {/snippet}
    </SettingCard>

    <SettingCard
      icon="delete"
      title="Clear lyrics cache"
      description="Removes cached LRCLIB records. Your own local .lrc files are never touched."
    >
      {#snippet control()}
        <button
          class="btn"
          type="button"
          disabled={(stats?.cacheFiles ?? 0) === 0}
          onclick={() => void client.clearLyricsCache()}
        >
          Clear cache
        </button>
      {/snippet}
    </SettingCard>
  </Section>

  <Section
    title="Local lyrics folder"
    description="Drop standard or Enhanced (.lrc) files named “Artist - Title.lrc” or “Title.lrc” here for 100 % offline lyrics that always take priority over online lookups."
  >
    <SettingCard
      icon="page"
      title="Custom .lrc overrides"
      description={stats
        ? `${stats.customFiles} local .lrc ${stats.customFiles === 1 ? 'file' : 'files'} · ${stats.customDir}`
        : 'Checked before any cache or online lookup.'}
    >
      {#snippet control()}
        <button class="btn" type="button" onclick={() => void client.openLyricsFolder()}>
          Open folder
        </button>
      {/snippet}
    </SettingCard>
  </Section>
{/if}

<style>
  .usage-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    border-radius: var(--ice-set-radius);
    background: var(--ice-set-card);
    border: 1px solid var(--ice-set-card-border);
  }

  .usage-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }

  .usage-title {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--ice-set-fg);
  }

  .usage-meta {
    font-size: 12px;
    color: var(--ice-set-fg-muted);
    font-variant-numeric: tabular-nums;
  }

  .meter-track {
    width: 100%;
    height: 6px;
    border-radius: 999px;
    background: var(--ice-set-control);
    overflow: hidden;
  }

  .meter-fill {
    height: 100%;
    min-width: 2px;
    border-radius: 999px;
    background: var(--ice-set-accent);
    transition: width 220ms var(--ice-set-ease);
  }

  .meter-fill.warn {
    background: var(--ice-set-warn);
  }
</style>
