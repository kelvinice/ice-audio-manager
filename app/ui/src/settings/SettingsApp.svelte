<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import type { NotchDelta, NotchSnapshot } from '../lib/ipc/types';

  let snapshot = $state<NotchSnapshot | null>(null);

  onMount(() => {
    let closed = false;
    const applySnapshot = (next: NotchSnapshot) => {
      if (!closed && (!snapshot || next.revision > snapshot.revision)) snapshot = next;
    };
    const listener = listen<NotchDelta>('notch://delta/v1', ({ payload }) => applySnapshot(payload.snapshot));
    void listener.then(async () => {
      if (closed) return;
      applySnapshot(await invoke<NotchSnapshot>('get_notch_snapshot'));
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          if (!closed) void invoke('report_settings_ready');
        });
      });
    });
    return () => {
      closed = true;
      void listener.then((unlisten) => unlisten());
    };
  });
</script>

<main class="settings-shell">
  <header class="hdr">
    <h1>Ice Audio Manager — Settings Spike</h1>
    <span class="tag">Win11 Mica · Destroyed on Close</span>
  </header>

  {#if snapshot}
    <section class="card">
      <div class="row">
        <span class="k">Active device</span>
        <span class="v">{snapshot.activeDevice}</span>
      </div>
      <div class="row">
        <span class="k">Target toggle</span>
        <span class="v">{snapshot.targetDevice}</span>
      </div>
      <div class="row">
        <span class="k">Spring preset</span>
        <span class="v">{snapshot.preset}</span>
      </div>
      <div class="row">
        <span class="k">Monitors</span>
        <span class="v">{snapshot.monitors.map((m) => `${m.width}×${m.height}@${m.refreshHz}Hz`).join(' · ')}</span>
      </div>
    </section>
  {/if}
</main>

<style>
  .settings-shell {
    min-height: 100vh;
    padding: 20px;
    background: rgba(14, 17, 22, 0.45);
    color: var(--ice-fg);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .hdr {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  h1 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .tag {
    font-size: 11px;
    color: var(--ice-accent-strong);
  }

  .card {
    background: rgba(23, 29, 39, 0.72);
    border: 1px solid var(--ice-border);
    border-radius: 12px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
  }

  .k {
    color: var(--ice-fg-muted);
  }
</style>
