<script lang="ts">
  import { formatCount, formatMb } from "../lib/format";
  import { artworkCacheBytes, clearArtworkCache } from "../lib/ipc";
  import type { AppPaths, LibraryState, PandoraSettings } from "../lib/types";
  import Icon from "./Icon.svelte";

  /**
   * Settings → Storage: what Pandora keeps on disk. Downloads have no size cap (they stay until removed; the songs are
   * listed in Library → Downloads); the artwork cache has a budget and trims its oldest covers. Reset sits in its own
   * card, so it is never next to a routine button.
   */
  interface Props {
    settings: PandoraSettings;
    library: LibraryState;
    appPaths: AppPaths | null;
    onUpdateSettings: (patch: Partial<PandoraSettings>) => void;
    onOpenDownloads: () => void;
    onClearCache: () => void;
    onRescanLocal: () => void;
    onResetSettings: () => void;
    onNotify: (msg: string) => void;
  }

  let {
    settings,
    library,
    appPaths,
    onUpdateSettings,
    onOpenDownloads,
    onClearCache,
    onRescanLocal,
    onResetSettings,
    onNotify,
  }: Props = $props();

  const ARTWORK_BUDGETS = [0, 128, 256, 512, 1024];

  let artworkBytes = $state(0);
  let confirmingReset = $state(false);
  let confirmingRemoveAll = $state(false);

  $effect(() => {
    void artworkCacheBytes().then((bytes) => (artworkBytes = bytes)).catch(() => {});
  });

  let downloadCount = $derived(library.cachedIds.length);
  let totalBytes = $derived(library.cacheBytes + artworkBytes);
  let artworkLimit = $derived(budgetLabel(settings.artworkCacheMb));

  function budgetLabel(mb: number): string {
    if (mb === 0) return "Off";
    return mb >= 1024 ? `${mb / 1024} GB` : `${mb} MB`;
  }

  async function clearArtwork() {
    try {
      const freed = await clearArtworkCache();
      artworkBytes = 0;
      onNotify(`Cleared ${formatMb(freed)} of artwork`);
    } catch (e) {
      onNotify(e instanceof Error ? e.message : String(e));
    }
  }
</script>

<section class="panel card-glass">
  <h2>Storage</h2>

  <div class="usage">
    <p class="total"><strong>{formatMb(totalBytes)}</strong> used by Pandora on this PC</p>
    <!-- Each part grows by its size; a part that holds anything keeps a sliver so it never vanishes. -->
    <div class="bar" role="img" aria-label={`Downloads ${formatMb(library.cacheBytes)}, artwork ${formatMb(artworkBytes)}`}>
      {#if library.cacheBytes > 0}<span class="seg seg-dl" style:flex-grow={library.cacheBytes}></span>{/if}
      {#if artworkBytes > 0}<span class="seg seg-art" style:flex-grow={artworkBytes}></span>{/if}
    </div>
    <div class="legend">
      <span><i class="key key-dl"></i>Downloads {formatMb(library.cacheBytes)}</span>
      <span><i class="key key-art"></i>Artwork {formatMb(artworkBytes)}</span>
    </div>
  </div>

  <div class="item">
    <span class="badge tone-dl"><Icon name="download" size={18} /></span>
    <div class="text">
      <div class="title">Downloads</div>
      <div class="sub">
        {formatCount(downloadCount, "song")} • {formatMb(library.cacheBytes)}. No size limit: downloads stay until you
        remove them.
      </div>
    </div>
    <div class="actions">
      {#if confirmingRemoveAll}
        <button
          class="btn-danger"
          onclick={() => {
            confirmingRemoveAll = false;
            onClearCache();
          }}>Remove all</button
        >
        <button class="btn-ghost" onclick={() => (confirmingRemoveAll = false)}>Cancel</button>
      {:else}
        <button class="btn-ghost" disabled={downloadCount === 0} onclick={onOpenDownloads}>View</button>
        <button class="btn-ghost danger" disabled={downloadCount === 0} onclick={() => (confirmingRemoveAll = true)}>
          Remove all
        </button>
      {/if}
    </div>
  </div>

  <div class="item">
    <span class="badge tone-art"><Icon name="sparkles" size={18} /></span>
    <div class="text">
      <div class="title">Artwork cache</div>
      <div class="sub">
        {#if settings.artworkCacheMb === 0}
          Off: covers load from the network each time.
        {:else}
          {formatMb(artworkBytes)} of {artworkLimit}. Covers open instantly and work offline; the oldest make room when it
          is full.
        {/if}
      </div>
    </div>
    <div class="actions">
      <select
        aria-label="Artwork cache limit"
        value={settings.artworkCacheMb}
        onchange={(e) => onUpdateSettings({ artworkCacheMb: Number((e.target as HTMLSelectElement).value) })}
      >
        {#each ARTWORK_BUDGETS as mb (mb)}
          <option value={mb}>{budgetLabel(mb)}</option>
        {/each}
      </select>
      <button class="btn-ghost" disabled={artworkBytes === 0} onclick={() => void clearArtwork()}>Clear</button>
    </div>
  </div>

  <div class="item">
    <span class="badge"><Icon name="folder" size={18} /></span>
    <div class="text">
      <div class="title">Local files</div>
      <div class="sub">
        {formatCount(library.localTracks.length, "song")} from {formatCount(library.localFolders.length, "folder")}. Played
        in place, never copied.
      </div>
    </div>
    <div class="actions">
      <button class="btn-ghost" disabled={library.localFolders.length === 0} onclick={onRescanLocal}>
        <Icon name="refresh" size={14} />
        <span>Rescan</span>
      </button>
    </div>
  </div>

  <div class="item">
    <span class="badge"><Icon name="library" size={18} /></span>
    <div class="text">
      <div class="title">Library</div>
      <div class="sub">
        {formatCount(library.liked.length, "favorite")} • {formatCount(library.playlists.length, "playlist")} •
        {formatCount(library.history.length, "song")} in history
      </div>
    </div>
  </div>

  {#if appPaths}
    <details class="paths">
      <summary>File locations</summary>
      <dl>
        <dt>Library</dt><dd><code>{appPaths.libraryPath}</code></dd>
        <dt>Downloads</dt><dd><code>{appPaths.cacheDir}</code></dd>
        <dt>Artwork</dt><dd><code>{appPaths.artworkDir}</code></dd>
        <dt>Ice config</dt><dd><code>{appPaths.iceConfigPath}</code></dd>
        <dt>Ice bridge</dt><dd><code>{appPaths.iceBridgePath}</code></dd>
      </dl>
    </details>
  {/if}
</section>

<section class="panel card-glass reset">
  <span class="badge tone-danger"><Icon name="refresh" size={18} /></span>
  <div class="text">
    <div class="title">Reset settings</div>
    <div class="sub">
      {confirmingReset
        ? "Reset every setting and the equalizer now? This can't be undone."
        : "Puts Pandora's settings and equalizer back to defaults. Your library, playlists and downloads stay."}
    </div>
  </div>
  <div class="actions">
    {#if confirmingReset}
      <button class="btn-ghost" onclick={() => (confirmingReset = false)}>Cancel</button>
      <button
        class="btn-danger"
        onclick={() => {
          confirmingReset = false;
          onResetSettings();
        }}>Reset</button
      >
    {:else}
      <button class="btn-ghost danger" onclick={() => (confirmingReset = true)}>Reset…</button>
    {/if}
  </div>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    padding: 20px 22px;
  }

  h2 {
    margin: 0 0 12px;
    font-size: 16px;
  }

  .usage {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    margin-bottom: 8px;
    border-radius: 12px;
    background: var(--p-fill);
  }

  .total {
    margin: 0;
    font-size: 13px;
    color: var(--p-text-muted);
  }

  .total strong {
    font-size: 22px;
    font-weight: 750;
    color: var(--p-text);
    margin-right: 4px;
  }

  .bar {
    display: flex;
    gap: 3px;
    height: 10px;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--p-text) 8%, transparent);
  }

  .seg {
    flex-basis: 0;
    min-width: 6px;
    height: 100%;
  }

  .seg:first-child {
    border-radius: 999px 0 0 999px;
  }

  .seg:last-child {
    border-radius: 0 999px 999px 0;
  }

  .seg:only-child {
    border-radius: 999px;
  }

  .seg-dl,
  .key-dl,
  .tone-dl {
    background: var(--p-accent);
  }

  .seg-art,
  .key-art,
  .tone-art {
    background: var(--p-companion);
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .key {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .item,
  .reset {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 14px;
  }

  .item {
    padding: 14px 0;
    border-top: 1px solid var(--p-border);
  }

  .badge {
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: var(--p-fill);
    color: var(--p-text-muted);
  }

  .tone-dl,
  .tone-art {
    color: var(--p-on-accent);
  }

  .tone-danger {
    background: color-mix(in srgb, var(--p-danger) 14%, var(--p-fill));
    color: var(--p-danger);
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  .title {
    font-size: 14px;
    font-weight: 650;
  }

  .sub {
    margin-top: 3px;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--p-text-dim);
  }

  .actions {
    display: flex;
    flex-shrink: 0;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  /* The confirming button is solid, so the second step reads clearly different from the first. */
  .actions .btn-danger {
    background: var(--p-danger);
    color: var(--p-bg);
  }

  select {
    height: 36px;
    padding: 0 12px;
    border: none;
    border-radius: 999px;
    background: color-mix(in srgb, var(--p-text) 10%, transparent);
    font-size: 13px;
    font-weight: 600;
  }

  .paths {
    padding-top: 12px;
    border-top: 1px solid var(--p-border);
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .paths summary {
    cursor: pointer;
    font-weight: 600;
  }

  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 6px 12px;
    margin: 10px 0 0;
  }

  dd {
    margin: 0;
    min-width: 0;
  }

  code {
    padding: 2px 6px;
    border-radius: 5px;
    background: var(--p-fill);
    word-break: break-all;
  }

  .reset {
    margin-top: 16px;
    padding: 16px 22px;
  }

  @media (max-width: 720px) {
    .item,
    .reset {
      flex-wrap: wrap;
    }

    .actions {
      width: 100%;
      justify-content: flex-start;
      padding-left: 54px;
    }
  }
</style>
