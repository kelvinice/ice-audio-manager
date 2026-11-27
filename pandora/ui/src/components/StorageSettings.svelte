<script lang="ts">
  import { formatCount, formatMb } from "../lib/format";
  import { artworkCacheBytes, clearArtworkCache } from "../lib/ipc";
  import { resolveArtworkUrl } from "../lib/theme";
  import type { AppPaths, LibraryState, PandoraSettings, Track } from "../lib/types";
  import Icon from "./Icon.svelte";

  /**
   * Settings → Storage: what Pandora keeps on disk. Downloads have no size cap (they stay until removed); the artwork
   * cache has a budget and trims its oldest covers. Reset lives apart, so it is never next to a routine button.
   */
  interface Props {
    settings: PandoraSettings;
    library: LibraryState;
    appPaths: AppPaths | null;
    proxyPort: number;
    onUpdateSettings: (patch: Partial<PandoraSettings>) => void;
    onOpenDownloads: () => void;
    onDeleteCached: (trackId: string) => void;
    onClearCache: () => void;
    onRescanLocal: () => void;
    onResetSettings: () => void;
    onNotify: (msg: string) => void;
  }

  let {
    settings,
    library,
    appPaths,
    proxyPort,
    onUpdateSettings,
    onOpenDownloads,
    onDeleteCached,
    onClearCache,
    onRescanLocal,
    onResetSettings,
    onNotify,
  }: Props = $props();

  const ARTWORK_BUDGETS = [0, 128, 256, 512, 1024];
  /** Downloads listed here before "Show all" sends you to the Library tab. */
  const LISTED = 5;

  let artworkBytes = $state(0);
  let confirmingReset = $state(false);
  let confirmingRemoveAll = $state(false);

  $effect(() => {
    void artworkCacheBytes().then((bytes) => (artworkBytes = bytes)).catch(() => {});
  });

  let downloads = $derived(library.cachedIds.map((id) => library.tracksIndex[id]).filter((t): t is Track => Boolean(t)));
  let totalBytes = $derived(library.cacheBytes + artworkBytes);
  let downloadShare = $derived(totalBytes > 0 ? (library.cacheBytes / totalBytes) * 100 : 0);
  let artworkLimit = $derived(settings.artworkCacheMb >= 1024 ? `${settings.artworkCacheMb / 1024} GB` : `${settings.artworkCacheMb} MB`);

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
    <div class="bar" role="img" aria-label={`Downloads ${formatMb(library.cacheBytes)}, artwork ${formatMb(artworkBytes)}`}>
      {#if totalBytes > 0}
        <span class="seg downloads" style:width={`${downloadShare}%`}></span>
        <span class="seg artwork" style:width={`${100 - downloadShare}%`}></span>
      {/if}
    </div>
    <div class="legend">
      <span><i class="dot downloads"></i>Downloads {formatMb(library.cacheBytes)}</span>
      <span><i class="dot artwork"></i>Artwork {formatMb(artworkBytes)}</span>
    </div>
  </div>

  <div class="item">
    <span class="badge downloads"><Icon name="download" size={18} /></span>
    <div class="text">
      <div class="title">Downloads</div>
      <div class="sub">
        {formatCount(downloads.length, "song")} • {formatMb(library.cacheBytes)}. No size limit: downloads stay until you remove
        them.
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
        <button class="btn-ghost danger" disabled={downloads.length === 0} onclick={() => (confirmingRemoveAll = true)}>
          Remove all
        </button>
      {/if}
    </div>
  </div>

  {#if downloads.length > 0}
    <ul class="downloads">
      {#each downloads.slice(0, LISTED) as track (track.id)}
        {@const art = resolveArtworkUrl(track.artworkUrl, proxyPort)}
        <li>
          <span class="thumb">{#if art}<img src={art} alt="" loading="lazy" />{:else}<Icon name="music" size={14} />{/if}</span>
          <span class="song">
            <span class="song-title">{track.title}</span>
            <span class="song-sub">{track.artist}</span>
          </span>
          <button class="btn-icon sm" aria-label={`Remove download of ${track.title}`} title="Remove download" onclick={() => onDeleteCached(track.id)}>
            <Icon name="x" size={14} />
          </button>
        </li>
      {/each}
    </ul>
    {#if downloads.length > LISTED}
      <button class="more" onclick={onOpenDownloads}>Show all {downloads.length} downloads</button>
    {/if}
  {/if}

  <div class="item">
    <span class="badge artwork"><Icon name="sparkles" size={18} /></span>
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
  <div class="text">
    <div class="title">Reset settings</div>
    <div class="sub">Puts Pandora's settings and equalizer back to defaults. Your library, playlists and downloads stay.</div>
  </div>
  <div class="actions">
    {#if confirmingReset}
      <button
        class="btn-danger"
        onclick={() => {
          confirmingReset = false;
          onResetSettings();
        }}>Reset</button
      >
      <button class="btn-ghost" onclick={() => (confirmingReset = false)}>Cancel</button>
    {:else}
      <button class="btn-ghost" onclick={() => (confirmingReset = true)}>Reset…</button>
    {/if}
  </div>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 4px;
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
    gap: 2px;
    height: 10px;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--p-text) 8%, transparent);
  }

  .seg {
    min-width: 0;
    height: 100%;
  }

  .seg.downloads,
  .dot.downloads,
  .badge.downloads {
    background: var(--p-accent);
  }

  .seg.artwork,
  .dot.artwork,
  .badge.artwork {
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

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .item,
  .reset {
    display: flex;
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

  .badge.downloads,
  .badge.artwork {
    color: var(--p-on-accent);
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
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
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

  .downloads {
    list-style: none;
    margin: -6px 0 6px 54px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .downloads li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 4px 4px 6px;
    border-radius: 8px;
  }

  .downloads li:hover {
    background: var(--p-fill);
  }

  .thumb {
    width: 34px;
    height: 34px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 5px;
    background: var(--p-fill);
    color: var(--p-text-dim);
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .song {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .song-title,
  .song-sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .song-title {
    font-size: 13px;
    font-weight: 600;
  }

  .song-sub {
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .more {
    align-self: flex-start;
    margin: 0 0 8px 60px;
    font-size: 12.5px;
    font-weight: 650;
    color: var(--p-text-muted);
  }

  .more:hover {
    color: var(--p-text);
    text-decoration: underline;
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
