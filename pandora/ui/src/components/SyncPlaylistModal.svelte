<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { applyPlaylistSync, cancelPlaylistImport, onPlaylistImportProgress, previewPlaylistSync } from "../lib/ipc";
  import type { LibraryState, Playlist, PlaylistImportProgress, PlaylistSyncPlan, Track } from "../lib/types";
  import Icon from "./Icon.svelte";

  /**
   * Sync a playlist with a Spotify / YouTube Music link: read the link (the saved one or a new one), review what
   * changed, choose whether songs that left the link stay, then apply. Nothing is saved before Apply.
   */
  interface Props {
    playlist: Playlist;
    onClose: () => void;
    onSynced: (library: LibraryState, message: string) => void;
  }

  let { playlist, onClose, onSynced }: Props = $props();

  let urlInput = $state("");
  let busy = $state(false);
  let err = $state<string | null>(null);
  let progress = $state<PlaylistImportProgress | null>(null);
  let plan = $state<PlaylistSyncPlan | null>(null);
  let removeMissing = $state(false);
  let showUnmatched = $state(false);
  let unlisten: (() => void) | null = null;

  const SERVICE: Record<string, string> = { spotify: "Spotify", ytmusic: "YouTube Music" };

  onMount(async () => {
    unlisten = await onPlaylistImportProgress((p) => {
      if (busy) progress = p;
    });
  });

  onDestroy(() => {
    unlisten?.();
    if (busy) void cancelPlaylistImport().catch(() => {});
  });

  function close() {
    if (!busy) onClose();
  }

  function ago(ms: number): string {
    const mins = Math.round((Date.now() - ms) / 60_000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins} min ago`;
    const hours = Math.round(mins / 60);
    if (hours < 48) return `${hours} h ago`;
    return new Date(ms).toLocaleDateString();
  }

  /** `input` null = the saved link. */
  async function check(input: string | null) {
    if (input !== null && !input.trim()) {
      err = "Paste a Spotify or YouTube Music playlist link";
      return;
    }
    busy = true;
    err = null;
    progress = null;
    try {
      plan = await previewPlaylistSync(playlist.id, input?.trim() ?? null);
      removeMissing = false;
      showUnmatched = false;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (!msg.toLowerCase().includes("cancel")) err = msg;
    } finally {
      busy = false;
    }
  }

  async function apply() {
    if (!plan) return;
    busy = true;
    err = null;
    try {
      const removing = removeMissing && plan.complete ? plan.missing.length : 0;
      const library = await applyPlaylistSync(plan, removing > 0);
      const parts = [`${plan.added.length} added`];
      if (removing > 0) parts.push(`${removing} removed`);
      onSynced(library, `Synced "${playlist.name}": ${parts.join(", ")}`);
      onClose();
    } catch (e) {
      err = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function label(t: Track): string {
    return t.artist ? `${t.artist} — ${t.title}` : t.title;
  }
</script>

{#snippet songList(tracks: Track[])}
  <ul class="songs">
    {#each tracks as t (t.id)}
      <li>{label(t)}</li>
    {/each}
  </ul>
{/snippet}

<div class="backdrop" role="presentation" onclick={close} onkeydown={(e) => e.key === "Escape" && close()}>
  <div
    class="modal card-glass"
    role="dialog"
    aria-modal="true"
    aria-label="Sync playlist"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.key === "Escape" && close()}
  >
    <div class="head">
      <div class="title-wrap">
        <Icon name="refresh" size={18} />
        <h3>Sync “{playlist.name}”</h3>
      </div>
      <button class="btn-icon" aria-label="Close" disabled={busy} onclick={close}><Icon name="x" size={16} /></button>
    </div>

    {#if !plan}
      {#if playlist.link}
        <div class="saved">
          <div class="saved-top">
            <Icon name="link" size={14} />
            <span>Last link · {SERVICE[playlist.link.service] ?? playlist.link.service} · synced {ago(playlist.link.syncedAtMs)}</span>
          </div>
          <div class="url" title={playlist.link.url}>{playlist.link.url}</div>
          <button type="button" class="btn-primary" disabled={busy} onclick={() => void check(null)}>
            Sync from last link
          </button>
        </div>
      {/if}
      <form class="field" onsubmit={(e) => { e.preventDefault(); void check(urlInput); }}>
        <span>{playlist.link ? "Or use a different link" : "Spotify or YouTube Music playlist link"}</span>
        <div class="row">
          <input
            type="url"
            placeholder="https://open.spotify.com/playlist/… or https://music.youtube.com/playlist?list=…"
            bind:value={urlInput}
            disabled={busy}
          />
          <button type="submit" class="btn-ghost" disabled={busy}>Check</button>
        </div>
      </form>
      <p class="hint">Public playlists and albums. Pandora keeps the full link, so a sync works later even when a short share link has expired.</p>
    {:else}
      <div class="summary">
        <strong>{plan.sourceName}</strong>
        <div class="chips">
          <span class="chip add">+{plan.added.length} new</span>
          <span class="chip">{plan.missing.length} gone from the link</span>
          <span class="chip">{plan.unchanged} unchanged</span>
          {#if plan.unmatchedTracks.length > 0}<span class="chip">{plan.unmatchedTracks.length} not found</span>{/if}
        </div>
      </div>
      {#if plan.note}<div class="note">{plan.note}</div>{/if}

      {#if plan.added.length > 0}
        <div class="group"><span>Will be added</span>{@render songList(plan.added)}</div>
      {/if}

      {#if plan.missing.length > 0}
        <div class="group">
          <span>No longer in the link</span>
          {@render songList(plan.missing)}
          <label class="choice"><input type="radio" bind:group={removeMissing} value={false} disabled={busy} /> Keep them in this playlist</label>
          <label class="choice" class:off={!plan.complete}>
            <input type="radio" bind:group={removeMissing} value={true} disabled={busy || !plan.complete} /> Remove them
          </label>
          {#if !plan.complete}
            <div class="note">Pandora couldn’t read the whole list, so it won’t remove songs.</div>
          {:else}
            <div class="note">Kept songs become yours and aren’t asked about again.</div>
          {/if}
        </div>
      {/if}

      {#if plan.unmatchedTracks.length > 0}
        <button type="button" class="link-btn" onclick={() => (showUnmatched = !showUnmatched)}>
          {showUnmatched ? "Hide" : "Show"} {plan.unmatchedTracks.length} songs not found on YouTube Music
        </button>
        {#if showUnmatched}
          <ul class="songs">{#each plan.unmatchedTracks as item, i (i)}<li>{item}</li>{/each}</ul>
        {/if}
      {/if}
    {/if}

    {#if busy && !plan}
      <div class="status">
        <Icon name="loader" size={13} spin />
        <span>
          {#if progress?.stage === "matching"}
            Matching {progress.completed} / {progress.total} ({progress.matched} found)
          {:else}
            Reading the link…
          {/if}
        </span>
        <button type="button" class="link-btn" onclick={() => void cancelPlaylistImport()}>Cancel</button>
      </div>
    {/if}

    {#if err}<div class="err">{err}</div>{/if}

    {#if plan}
      <div class="foot">
        <button type="button" class="btn-ghost" disabled={busy} onclick={() => (plan = null)}>Back</button>
        <button type="button" class="btn-primary" disabled={busy} onclick={() => void apply()}>
          {busy ? "Saving…" : "Apply sync"}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 62;
    background: rgba(0, 0, 0, 0.62); /* same scrim as the other modals */
    display: grid;
    place-items: center;
    padding: 20px;
  }

  .modal {
    width: 500px;
    max-width: 94vw;
    max-height: 86vh;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: var(--p-surface-solid);
  }

  .head,
  .title-wrap,
  .row,
  .status,
  .saved-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .head {
    justify-content: space-between;
  }

  .title-wrap {
    color: var(--p-accent-bright);
    min-width: 0;
  }

  .head h3 {
    margin: 0;
    font-size: 18px;
    color: var(--p-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .saved,
  .group {
    padding: 11px 12px;
    border-radius: 12px;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .saved-top {
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .url {
    font-size: 12px;
    color: var(--p-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .saved .btn-primary {
    align-self: flex-start;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field span,
  .group > span {
    font-size: 11px;
    font-weight: 600;
    color: var(--p-text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .field input {
    flex: 1;
    min-width: 0;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    border-radius: 12px;
    padding: 10px 12px;
    font-size: 13px;
    color: var(--p-text);
    outline: none;
  }

  .field input:focus {
    border-color: var(--p-accent);
  }

  .hint,
  .note {
    margin: 0;
    font-size: 11px;
    line-height: 1.4;
    color: var(--p-text-muted);
  }

  .summary {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 14px;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    font-size: 11px;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
  }

  .chip.add {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
  }

  .songs {
    margin: 0;
    padding-left: 16px;
    max-height: 120px;
    overflow-y: auto;
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .choice {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }

  .choice.off {
    opacity: 0.5;
    cursor: default;
  }

  .status {
    font-size: 12px;
    font-weight: 600;
  }

  .status span {
    flex: 1;
  }

  .link-btn {
    align-self: flex-start;
    font-size: 11px;
    font-weight: 600;
    color: var(--p-accent-bright);
    text-decoration: underline;
  }

  .err {
    padding: 9px 12px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--p-danger) 14%, transparent);
    font-size: 12px;
    color: var(--p-danger);
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
