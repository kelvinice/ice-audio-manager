<script lang="ts">
  import type { DevicePrefsView, SavedDevice } from '../../lib/ipc/engine';
  import { MAX_VISIBLE_DEVICE_CHIPS } from '../../lib/notch/devices';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import { hiddenDevices, moveItem, savedLabel } from '../../lib/settings/model';
  import DeviceIcon from '../../notch/activities/DeviceIcon.svelte';
  import Icon from '../controls/Icon.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import DeviceCard from './DeviceCard.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();

  const engine = $derived(client.engine);
  const settings = $derived(engine?.settings ?? null);
  const favourites = $derived(settings?.favouritesPlayback ?? []);
  const hidden = $derived([...hiddenDevices(settings, 'render'), ...hiddenDevices(settings, 'capture')]);

  const live = (saved: SavedDevice) => engine?.playback.find((d) => d.id === saved.deviceId) ?? null;

  const favLabel = (saved: SavedDevice) => savedLabel(saved, settings);

  function move(index: number, delta: -1 | 1): void {
    const next = moveItem(favourites, index, delta);
    if (next) client.apply({ type: 'setFavourites', flow: 'render', deviceIds: next.map((f) => f.savedId) });
  }

  function remove(index: number): void {
    const deviceIds = favourites.filter((_, i) => i !== index).map((f) => f.savedId);
    client.apply({ type: 'setFavourites', flow: 'render', deviceIds });
  }

  function unhide(pref: DevicePrefsView): void {
    if (!pref.device.deviceId) return;
    client.apply({ type: 'setDevicePrefs', deviceId: pref.device.deviceId, hidden: false, alias: null, spatial: null });
  }
</script>

<Section
  title="Quick switch in the notch"
  description={`The expanded notch shows these as one-click switches, in this order. The first ${MAX_VISIBLE_DEVICE_CHIPS} fit. Star a device below to add it.`}
>
  {#if favourites.length === 0}
    <div class="empty">
      <Icon name="star" size={20} />
      <span>No devices yet. Star a playback device below, and the notch shows it next to the current one.</span>
    </div>
  {:else}
    <ol class="favs">
      {#each favourites as fav, index (fav.savedId)}
        {@const device = live(fav)}
        <li class:overflow={index >= MAX_VISIBLE_DEVICE_CHIPS}>
          <span class="pos">{index + 1}</span>
          <span class="ico"><DeviceIcon kind="auto" formFactor={device?.formFactor ?? null} size={18} /></span>
          <span class="name">
            {favLabel(fav)}
            {#if !device}<span class="muted">Not connected</span>
            {:else if index >= MAX_VISIBLE_DEVICE_CHIPS}<span class="muted">Under “More…”</span>{/if}
          </span>
          <span class="actions">
            <button type="button" class="icon-btn" aria-label={`Move ${favLabel(fav)} up`} disabled={index === 0} onclick={() => move(index, -1)}>
              <Icon name="up" size={12} />
            </button>
            <button
              type="button"
              class="icon-btn"
              aria-label={`Move ${favLabel(fav)} down`}
              disabled={index === favourites.length - 1}
              onclick={() => move(index, 1)}
            >
              <Icon name="down" size={12} />
            </button>
            <button type="button" class="icon-btn" aria-label={`Remove ${favLabel(fav)} from the notch`} onclick={() => remove(index)}>
              <Icon name="close" size={12} />
            </button>
          </span>
        </li>
      {/each}
    </ol>
  {/if}
</Section>

{#if engine}
  <Section title="Playback devices">
    {#each engine.playback as device (device.id)}
      <DeviceCard {client} {device} />
    {:else}
      <p class="none">No playback devices are connected.</p>
    {/each}
  </Section>

  <Section title="Recording devices">
    {#each engine.recording as device (device.id)}
      <DeviceCard {client} {device} />
    {:else}
      <p class="none">No microphones are connected.</p>
    {/each}
  </Section>
{/if}

{#if hidden.length}
  <Section title="Hidden devices" description="Left out of the tray and the notch.">
    {#each hidden as pref (pref.device.savedId)}
      <SettingCard
        icon={pref.device.flow === 'render' ? 'speakers' : 'mic'}
        title={pref.alias || pref.device.name}
        description={pref.device.deviceId ? undefined : 'Not connected. Connect it to show it again.'}
      >
        {#snippet control()}
          <button type="button" class="btn" disabled={!pref.device.deviceId} onclick={() => unhide(pref)}>Show</button>
        {/snippet}
      </SettingCard>
    {/each}
  </Section>
{/if}

<style>
  .empty {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 18px 16px;
    border-radius: var(--ice-set-radius);
    border: 1px dashed var(--ice-set-control-border);
    color: var(--ice-set-fg-muted);
  }

  .favs {
    list-style: none;
    margin: 0;
    padding: 0;
    border-radius: var(--ice-set-radius);
    border: 1px solid var(--ice-set-card-border);
    background: var(--ice-set-card);
  }

  li {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    padding: 0 12px 0 16px;
  }

  li + li {
    border-top: 1px solid var(--ice-set-divider);
  }

  li.overflow .pos,
  li.overflow .ico {
    opacity: 0.5;
  }

  .pos {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    font-size: 12px;
    font-weight: 600;
    color: var(--ice-set-accent);
    background: var(--ice-set-accent-soft);
  }

  .ico {
    display: inline-flex;
  }

  .name {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .muted {
    font-size: 12px;
    line-height: 16px;
    color: var(--ice-set-fg-muted);
  }

  .actions {
    display: flex;
    gap: 2px;
  }


  .none {
    margin: 0;
    padding: 8px 0;
    color: var(--ice-set-fg-muted);
  }

</style>
