<script lang="ts">
  import { resolveArtworkUrl } from "../lib/theme";
  import type { Playlist, Track } from "../lib/types";
  import Icon from "./Icon.svelte";

  interface Props {
    open: boolean;
    track: Track | null;
    playlists: Playlist[];
    proxyPort: number;
    onClose: () => void;
    onCreatePlaylist: (name: string, initialTrack: Track | null) => Promise<void>;
    onToggleTrackInPlaylist: (playlistId: string, track: Track, inPlaylist: boolean) => Promise<void>;
  }

  let {
    open,
    track,
    playlists,
    proxyPort,
    onClose,
    onCreatePlaylist,
    onToggleTrackInPlaylist,
  }: Props = $props();

  let newName = $state("");
  let busy = $state(false);
  let err = $state<string | null>(null);

  async function handleCreate(e: Event) {
    e.preventDefault();
    const clean = newName.trim();
    if (!clean) {
      err = "Enter a playlist name";
      return;
    }
    busy = true;
    err = null;
    try {
      await onCreatePlaylist(clean, track);
      newName = "";
      onClose();
    } catch (e) {
      err = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function toggleMembership(pl: Playlist) {
    if (!track) return;
    const inPl = pl.trackIds.includes(track.id);
    busy = true;
    err = null;
    try {
      await onToggleTrackInPlaylist(pl.id, track, inPl);
    } catch (e) {
      err = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={onClose}
    onkeydown={(e) => { if (e.key === "Escape") onClose(); }}
  >
    <div
      class="modal card-glass"
      role="dialog"
      aria-modal="true"
      aria-label={track ? "Add to Playlist" : "Create Playlist"}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => { if (e.key === "Escape") onClose(); }}
    >
      <div class="head">
        <h3>{track ? "Add to Playlist" : "New Playlist"}</h3>
        <button class="close-btn" aria-label="Close" onclick={onClose}><Icon name="x" size={16} /></button>
      </div>

      {#if track}
        <div class="track-preview">
          <div class="thumb">
            {#if resolveArtworkUrl(track.artworkUrl, proxyPort)}
              <img src={resolveArtworkUrl(track.artworkUrl, proxyPort)} alt={track.title} />
            {:else}
              <Icon name="music" size={18} />
            {/if}
          </div>
          <div class="meta">
            <div class="title">{track.title}</div>
            <div class="sub">{track.artist}</div>
          </div>
        </div>
      {/if}

      <form class="create-row" onsubmit={handleCreate}>
        <input
          type="text"
          placeholder={track ? "Create new playlist & add track..." : "Playlist name (e.g. Late Night)..."}
          bind:value={newName}
        />
        <button type="submit" class="btn-primary" disabled={busy}>
          {track ? "Create & Add" : "Create"}
        </button>
      </form>

      {#if err}
        <div class="err">{err}</div>
      {/if}

      {#if track}
        <div class="list-head">Existing Playlists ({playlists.length})</div>
        <div class="pl-list">
          {#if playlists.length === 0}
            <div class="empty">No playlists yet — create your first one above.</div>
          {:else}
            {#each playlists as pl (pl.id)}
              {@const checked = pl.trackIds.includes(track.id)}
              <button
                class="pl-row"
                class:checked
                disabled={busy}
                onclick={() => void toggleMembership(pl)}
              >
                <span class="box">
                  {#if checked}<Icon name="check" size={12} strokeWidth={2.5} />{/if}
                </span>
                <span class="name">{pl.name}</span>
                <span class="count">{pl.trackIds.length} tracks</span>
              </button>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgba(0, 0, 0, 0.58);
    display: grid;
    place-items: center;
    padding: 20px;
  }

  .modal {
    width: 440px;
    max-width: 94vw;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background: rgba(15, 19, 33, 0.97);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .head h3 {
    margin: 0;
    font-size: 18px;
  }

  .close-btn {
    padding: 4px 8px;
    color: var(--p-text-muted);
    font-size: 15px;
  }

  .track-preview {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.05);
  }

  .thumb {
    width: 42px;
    height: 42px;
    border-radius: 8px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.08);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .meta {
    min-width: 0;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub {
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .create-row {
    display: flex;
    gap: 8px;
  }

  .create-row input {
    flex: 1;
    min-width: 0;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--p-border);
    border-radius: 999px;
    padding: 9px 14px;
    font-size: 13px;
    outline: none;
  }

  .create-row input:focus {
    border-color: var(--p-accent);
  }

  .err {
    font-size: 12px;
    color: #fca5a5;
  }

  .list-head {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--p-text-dim);
  }

  .pl-list {
    max-height: 240px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .empty {
    padding: 18px 8px;
    text-align: center;
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .pl-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.04);
    text-align: left;
  }

  .pl-row:hover {
    background: rgba(255, 255, 255, 0.09);
  }

  .pl-row.checked {
    background: var(--p-accent-soft);
  }

  .box {
    width: 18px;
    height: 18px;
    border-radius: 5px;
    border: 1px solid var(--p-border-strong);
    display: grid;
    place-items: center;
    font-size: 11px;
    font-weight: 700;
    color: var(--p-accent-bright);
  }

  .pl-row.checked .box {
    background: var(--p-accent);
    color: #080b12;
    border-color: var(--p-accent);
  }

  .name {
    flex: 1;
    font-size: 13px;
    font-weight: 600;
  }

  .count {
    font-size: 11px;
    color: var(--p-text-dim);
  }
</style>
