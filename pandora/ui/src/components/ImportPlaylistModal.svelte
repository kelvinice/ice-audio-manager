<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    cancelPlaylistImport,
    importPlaylist,
    onPlaylistImportProgress,
  } from "../lib/ipc";
  import type { PlaylistImportProgress, PlaylistImportResult } from "../lib/types";
  import Icon from "./Icon.svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
    onImported: (result: PlaylistImportResult) => void;
    onOpenPlaylist: (playlistId: string) => void;
  }

  let { open, onClose, onImported, onOpenPlaylist }: Props = $props();

  let mode = $state<"url" | "text">("url");
  let urlInput = $state("");
  let textInput = $state("");
  let customName = $state("");
  let busy = $state(false);
  let err = $state<string | null>(null);
  let progress = $state<PlaylistImportProgress | null>(null);
  let result = $state<PlaylistImportResult | null>(null);
  let showUnmatched = $state(false);
  let fileInputEl = $state<HTMLInputElement | null>(null);
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    unlisten = await onPlaylistImportProgress((p) => {
      progress = p;
    });
  });

  onDestroy(() => {
    unlisten?.();
    if (busy) {
      void cancelPlaylistImport().catch(() => {});
    }
  });

  function resetTransient() {
    err = null;
    result = null;
    progress = null;
    showUnmatched = false;
  }

  function handleSafeClose() {
    if (busy) return;
    resetTransient();
    onClose();
  }

  async function handleFileSelect(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    try {
      textInput = await file.text();
      if (!customName.trim()) {
        customName = file.name.replace(/\.[^.]+$/, "");
      }
      err = null;
    } catch (readErr) {
      err = readErr instanceof Error ? readErr.message : String(readErr);
    } finally {
      input.value = "";
    }
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    const payload = (mode === "url" ? urlInput : textInput).trim();
    if (!payload) {
      err =
        mode === "url"
          ? "Paste a public Spotify or YouTube Music playlist URL"
          : "Paste Exportify CSV or Artist - Title lines";
      return;
    }

    busy = true;
    err = null;
    result = null;
    showUnmatched = false;
    progress = {
      stage: "fetching",
      source: mode === "url" ? "auto" : "text",
      completed: 0,
      total: 0,
      matched: 0,
      currentTrack: null,
    };

    try {
      const res = await importPlaylist(payload, customName.trim() || null);
      result = res;
      onImported(res);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (!msg.toLowerCase().includes("cancel")) {
        err = msg;
      }
    } finally {
      busy = false;
    }
  }

  let pct = $derived(
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.completed / progress.total) * 100))
      : 12,
  );
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={handleSafeClose}
    onkeydown={(e) => {
      if (e.key === "Escape") handleSafeClose();
    }}
  >
    <div
      class="modal card-glass"
      role="dialog"
      aria-modal="true"
      aria-label="Import Playlist"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === "Escape") handleSafeClose();
      }}
    >
      <div class="head">
        <div class="title-wrap">
          <Icon name="link" size={18} />
          <h3>Import Playlist</h3>
        </div>
        <button
          class="close-btn"
          aria-label="Close"
          disabled={busy}
          onclick={handleSafeClose}
        >
          <Icon name="x" size={16} />
        </button>
      </div>

      <div class="tabs">
        <button
          type="button"
          class="tab-btn"
          class:active={mode === "url"}
          disabled={busy}
          onclick={() => {
            mode = "url";
            err = null;
          }}
        >
          Spotify / YouTube Music URL
        </button>
        <button
          type="button"
          class="tab-btn"
          class:active={mode === "text"}
          disabled={busy}
          onclick={() => {
            mode = "text";
            err = null;
          }}
        >
          Paste / CSV
        </button>
      </div>

      <form class="form" onsubmit={handleSubmit}>
        {#if mode === "url"}
          <label class="field">
            <span>Public Playlist or Album Link</span>
            <input
              type="url"
              placeholder="https://open.spotify.com/playlist/... or https://music.youtube.com/playlist?list=..."
              bind:value={urlInput}
              disabled={busy}
            />
          </label>
        {:else}
          <div class="field">
            <div class="field-top">
              <span>Tracklist (Artist - Title), M3U, or Exportify CSV</span>
              <button
                type="button"
                class="mini-btn"
                disabled={busy}
                onclick={() => fileInputEl?.click()}
              >
                <Icon name="folder" size={12} />
                <span>Load File</span>
              </button>
              <input
                bind:this={fileInputEl}
                type="file"
                accept=".csv,.txt,.m3u,.m3u8"
                hidden
                onchange={handleFileSelect}
              />
            </div>
            <textarea
              rows="5"
              placeholder={"Laufey - Silver Lining\nGracie Abrams - I Love You, I'm Sorry\nSabrina Carpenter - Espresso"}
              bind:value={textInput}
              disabled={busy}
            ></textarea>
          </div>
        {/if}

        <label class="field">
          <span>Playlist Name (optional)</span>
          <input
            type="text"
            placeholder="Auto-detect playlist name"
            bind:value={customName}
            disabled={busy}
          />
        </label>

        {#if busy && progress}
          <div class="progress-box">
            <div class="progress-head">
              <span class="prog-status">
                <Icon name="loader" size={13} spin />
                {#if progress.stage === "fetching"}
                  Fetching playlist metadata...
                {:else}
                  Matching {progress.completed} / {progress.total} ({progress.matched} found)
                {/if}
              </span>
              <button
                type="button"
                class="mini-btn danger"
                onclick={() => void cancelPlaylistImport()}
              >
                Cancel
              </button>
            </div>
            <div class="bar-track">
              <div class="bar-fill" style="width: {pct}%"></div>
            </div>
            {#if progress.currentTrack}
              <div class="curr-track">{progress.currentTrack}</div>
            {/if}
          </div>
        {/if}

        {#if err}
          <div class="err">{err}</div>
        {/if}

        {#if result}
          <div class="result-box">
            <div class="result-head">
              <Icon name="check" size={15} />
              <strong>{result.playlistName}</strong>
              <span class="pill-count">
                {result.importedTracks} / {result.totalTracks} tracks
              </span>
            </div>
            {#if result.note}
              <div class="note">{result.note}</div>
            {/if}
            {#if result.unmatchedTracks.length > 0}
              <button
                type="button"
                class="toggle-unmatched"
                onclick={() => (showUnmatched = !showUnmatched)}
              >
                {showUnmatched ? "Hide" : "Show"} {result.unmatchedTracks.length} unmatched tracks
              </button>
              {#if showUnmatched}
                <ul class="unmatched-list">
                  {#each result.unmatchedTracks as item (item)}
                    <li>{item}</li>
                  {/each}
                </ul>
              {/if}
            {/if}
          </div>
        {/if}

        <div class="foot">
          {#if result && !busy}
            <button type="button" class="btn-ghost" onclick={handleSafeClose}>Done</button>
            <button
              type="button"
              class="btn-primary"
              onclick={() => {
                const id = result?.playlistId;
                resetTransient();
                if (id) onOpenPlaylist(id);
              }}
            >
              Open Playlist
            </button>
          {:else}
            <button type="button" class="btn-ghost" disabled={busy} onclick={handleSafeClose}>
              Cancel
            </button>
            <button type="submit" class="btn-primary" disabled={busy}>
              {busy ? "Importing..." : "Import Playlist"}
            </button>
          {/if}
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 62;
    background: rgba(0, 0, 0, 0.62);
    display: grid;
    place-items: center;
    padding: 20px;
  }

  .modal {
    width: 500px;
    max-width: 94vw;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background: var(--p-surface-solid);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .title-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--p-accent-bright);
  }

  .head h3 {
    margin: 0;
    font-size: 18px;
    color: var(--p-text);
  }

  .close-btn {
    padding: 4px 8px;
    color: var(--p-text-muted);
  }

  .close-btn:disabled {
    opacity: 0.4;
  }

  .tabs {
    display: flex;
    gap: 6px;
    background: var(--p-fill);
    padding: 4px;
    border-radius: 12px;
    border: 1px solid var(--p-border);
  }

  .tab-btn {
    flex: 1;
    padding: 7px 10px;
    border-radius: 9px;
    font-size: 12px;
    font-weight: 600;
    color: var(--p-text-muted);
    transition: background 0.16s ease, color 0.16s ease;
  }

  .tab-btn.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .field span {
    font-size: 11px;
    font-weight: 600;
    color: var(--p-text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .field input,
  .field textarea {
    width: 100%;
    box-sizing: border-box;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    border-radius: 12px;
    padding: 10px 12px;
    font-size: 13px;
    color: var(--p-text);
    outline: none;
    resize: vertical;
  }

  .field input:focus,
  .field textarea:focus {
    border-color: var(--p-accent);
  }

  .mini-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    color: var(--p-text);
  }

  .mini-btn:hover:not(:disabled) {
    border-color: var(--p-border-strong);
  }

  .mini-btn.danger:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.22);
    color: var(--p-danger);
  }

  .progress-box {
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .progress-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
  }

  .prog-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }

  .bar-track {
    height: 6px;
    border-radius: 999px;
    background: var(--p-border);
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    border-radius: 999px;
    background: linear-gradient(90deg, var(--p-accent), var(--p-accent-bright));
    transition: width 0.2s ease;
  }

  .curr-track {
    font-size: 11px;
    color: var(--p-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .err {
    padding: 9px 12px;
    border-radius: 10px;
    background: rgba(239, 68, 68, 0.14);
    border: 1px solid rgba(239, 68, 68, 0.35);
    font-size: 12px;
    color: var(--p-danger);
  }

  .result-box {
    padding: 11px 12px;
    border-radius: 12px;
    background: var(--p-accent-soft);
    border: 1px solid var(--p-border-strong);
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
  }

  .result-head {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--p-accent-bright);
  }

  .result-head strong {
    color: var(--p-text);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pill-count {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--p-fill);
    border: 1px solid var(--p-border);
    color: var(--p-text);
  }

  .note {
    font-size: 11px;
    color: var(--p-text-muted);
    line-height: 1.4;
  }

  .toggle-unmatched {
    align-self: flex-start;
    font-size: 11px;
    font-weight: 600;
    color: var(--p-accent-bright);
    text-decoration: underline;
  }

  .unmatched-list {
    margin: 2px 0 0;
    padding-left: 16px;
    max-height: 110px;
    overflow-y: auto;
    font-size: 11px;
    color: var(--p-text-muted);
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 2px;
  }
</style>
