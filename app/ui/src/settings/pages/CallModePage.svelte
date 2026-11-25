<script lang="ts">
  // D-30 call mode (owner, 2026-11-25): lower the music while people talk on a call, bring it back when it's quiet.
  import type { CallModeConfig, CallSensitivity } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import Icon from '../controls/Icon.svelte';
  import InfoBar from '../controls/InfoBar.svelte';
  import Section from '../controls/Section.svelte';
  import Select from '../controls/Select.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();

  const settings = $derived(client.engine?.settings ?? null);
  const call = $derived(settings?.callMode ?? null);
  const off = $derived(!call?.enabled);

  let draftApp = $state('');

  const levelOptions = [0, 10, 20, 30, 40, 50, 70].map((p) => ({
    value: String(p),
    label: p === 0 ? 'Silent' : `${p} %${p === 30 ? ' (Default)' : ''}`,
  }));
  const holdOptions = [1, 2, 3, 5, 8, 10, 15].map((s) => ({
    value: String(s),
    label: `${s} ${s === 1 ? 'second' : 'seconds'}${s === 3 ? ' (Default)' : ''}`,
  }));
  const sensitivityOptions: { value: CallSensitivity; label: string }[] = [
    { value: 'low', label: 'Low: clear speech only' },
    { value: 'normal', label: 'Normal (Default)' },
    { value: 'high', label: 'High: soft voices too' },
  ];

  function patch(change: Partial<CallModeConfig>): void {
    void client.apply({
      type: 'setCallMode',
      enabled: change.enabled ?? null,
      apps: change.apps ?? null,
      duckPercent: change.duckPercent ?? null,
      holdSecs: change.holdSecs ?? null,
      sensitivity: change.sensitivity ?? null,
    });
  }

  function addApp(): void {
    const name = draftApp.trim();
    if (!call || name.length === 0) return;
    patch({ apps: [...call.apps, name] });
    draftApp = '';
  }

  function removeApp(name: string): void {
    if (call) patch({ apps: call.apps.filter((a) => a !== name) });
  }
</script>

{#if settings && call}
  <Section
    title="Call mode"
    description="Turns the music down while people talk on a call, then brings it back after a few quiet seconds."
  >
    <SettingCard
      icon="call"
      title="Lower music during calls"
      description="Listens to how loud your call app is (never what is said) and lowers the app playing music, like Spotify. Your music volume comes back when you change it by hand or when the call ends."
    >
      {#snippet control()}
        <Toggle label="Lower music during calls" checked={call.enabled} onchange={(enabled) => patch({ enabled })} />
      {/snippet}
    </SettingCard>
    {#if call.enabled}
      <InfoBar
        tone="info"
        title="Using Discord's Attenuation too?"
        text="Discord can also lower other apps (User Settings → Voice & Video → Attenuation). Turn one of them off so the music isn't lowered twice."
      />
    {/if}
  </Section>

  <Section title="How it sounds" description="How far the music drops and how soon it comes back.">
    <SettingCard
      icon="volume"
      title="Music level while talking"
      description="A share of the music's own volume."
      disabled={off}
    >
      {#snippet control()}
        <Select
          label="Music level while talking"
          value={String(call.duckPercent)}
          options={levelOptions}
          onchange={(v) => patch({ duckPercent: Number.parseInt(v, 10) })}
        />
      {/snippet}
    </SettingCard>
    <SettingCard
      icon="music"
      title="Back to full volume after"
      description="How long it must stay quiet before the music fades back in."
      disabled={off}
    >
      {#snippet control()}
        <Select
          label="Back to full volume after"
          value={String(call.holdSecs)}
          options={holdOptions}
          onchange={(v) => patch({ holdSecs: Number.parseInt(v, 10) })}
        />
      {/snippet}
    </SettingCard>
    <SettingCard
      icon="mic"
      title="Sensitivity"
      description="How loud the call must be to count as talking. Raise it if soft voices don't lower the music."
      disabled={off}
    >
      {#snippet control()}
        <Select
          label="Sensitivity"
          value={call.sensitivity}
          options={sensitivityOptions}
          onchange={(sensitivity) => patch({ sensitivity })}
        />
      {/snippet}
    </SettingCard>
  </Section>

  <Section
    title="Call apps"
    description="Sound from these apps counts as a call. Use the app's file name, as Task Manager's Details tab shows it."
  >
    <div class="apps" class:off>
      {#each call.apps as app (app)}
        <div class="app">
          <Icon name="call" size={14} />
          <span class="name">{app}</span>
          <button class="icon-btn" type="button" aria-label={`Remove ${app}`} onclick={() => removeApp(app)}>
            <Icon name="close" size={12} />
          </button>
        </div>
      {:else}
        <p class="empty">No call apps: nothing will lower the music.</p>
      {/each}
      <form
        class="add"
        onsubmit={(e) => {
          e.preventDefault();
          addApp();
        }}
      >
        <input
          class="text"
          type="text"
          maxlength="64"
          placeholder="e.g. Zoom.exe or ms-teams.exe"
          bind:value={draftApp}
          aria-label="Call app file name"
        />
        <button class="btn" type="submit" disabled={draftApp.trim().length === 0}>
          <Icon name="add" size={12} /> Add
        </button>
      </form>
    </div>
    <InfoBar
      tone="info"
      title="Which music gets lowered"
      text="The app the notch shows as playing. For YouTube Music in a browser, that browser's whole sound is lowered, so a call in the same browser would be lowered too: use the Discord app for calls."
    />
  </Section>
{/if}

<style>
  .apps {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
    border-radius: var(--ice-set-radius);
    background: var(--ice-set-card);
    border: 1px solid var(--ice-set-card-border);
  }

  .apps.off {
    opacity: 0.6;
  }

  .app {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 36px;
    padding: 0 4px 0 10px;
    border-radius: var(--ice-set-radius-sm);
    color: var(--ice-set-fg-muted);
  }

  .app:hover {
    background: var(--ice-set-control);
  }

  .name {
    flex: 1;
    min-width: 0;
    font-size: 13.5px;
    color: var(--ice-set-fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    margin: 0;
    padding: 8px 10px;
    font-size: 12.5px;
    color: var(--ice-set-fg-muted);
  }

  .add {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 4px 2px;
  }

  .text {
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--ice-set-radius-sm);
    border: 1px solid var(--ice-set-control-border);
    border-bottom-color: var(--ice-set-control-strong);
    background: var(--ice-set-control);
    outline: none;
  }

  .text:focus {
    border-bottom: 2px solid var(--ice-set-accent);
  }

  .text::placeholder {
    color: var(--ice-set-fg-subtle);
  }
</style>
