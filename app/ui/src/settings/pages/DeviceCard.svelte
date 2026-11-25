<script lang="ts">
  import type { DeviceSummary, SpatialMode } from '../../lib/ipc/engine';
  import { isDeviceFavourite, toggleFavouriteDeviceIds } from '../../lib/notch/devices';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import { deviceLabel, lyricsDelayChoice, lyricsDelayFromChoice, prefsFor } from '../../lib/settings/model';
  import DeviceIcon from '../../notch/activities/DeviceIcon.svelte';
  import Icon from '../controls/Icon.svelte';
  import Row from '../controls/Row.svelte';
  import Select from '../controls/Select.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';

  interface Props {
    client: SettingsClient;
    device: DeviceSummary;
  }

  let { client, device }: Props = $props();

  const settings = $derived(client.engine?.settings ?? null);
  const prefs = $derived(prefsFor(settings, device.id));
  const playback = $derived(device.flow === 'render');
  const inNotch = $derived(isDeviceFavourite(settings?.favouritesPlayback, device.id));
  const label = $derived(deviceLabel(device, settings));

  const facts = $derived(
    [
      device.isDefault && 'Default',
      device.isDefaultCommunications && 'Communications',
      device.isVirtual && 'Virtual',
      label !== device.name && device.name,
    ]
      .filter(Boolean)
      .join(' · '),
  );

  // Twin: app/src-tauri/src/tray.rs `SPATIAL_OPTIONS` (the same four choices; a value saved by hand stays visible).
  const SPATIAL: { value: SpatialMode; label: string }[] = [
    { value: 'keep', label: 'Keep Windows’ setting' },
    { value: 'off', label: 'Off' },
    { value: 'windowsSonic', label: 'Windows Sonic' },
    { value: 'dolbyAtmosHeadphones', label: 'Dolby Atmos for Headphones' },
  ];
  const spatialOptions = $derived(
    prefs && !SPATIAL.some((o) => o.value === prefs.spatial)
      ? [...SPATIAL, { value: prefs.spatial, label: prefs.spatial }]
      : SPATIAL,
  );

  const lyricsDelay = $derived(lyricsDelayChoice(prefs?.lyricsDelayMs, device.lyricsDelayMs));

  let expanded = $state(false);
  let draftAlias = $state('');
  $effect(() => {
    draftAlias = prefs?.alias ?? '';
  });

  function saveAlias(): void {
    const alias = draftAlias.trim();
    if (alias === (prefs?.alias ?? '')) return;
    client.apply({ type: 'setDevicePrefs', deviceId: device.id, alias, spatial: null, hidden: null });
  }

  function toggleNotch(): void {
    client.apply({
      type: 'setFavourites',
      flow: 'render',
      deviceIds: toggleFavouriteDeviceIds(settings?.favouritesPlayback, device.id),
    });
  }
</script>

<SettingCard title={label} description={facts || undefined} bind:expanded>
  {#snippet leading()}
    <DeviceIcon kind="auto" formFactor={device.formFactor} size={20} />
  {/snippet}
  {#snippet control()}
    {#if playback}
      <button
        type="button"
        class="icon-btn star"
        class:on={inNotch}
        aria-pressed={inNotch}
        title={inNotch ? 'Shown in the notch' : 'Show in the notch'}
        aria-label={`Show ${label} in the notch`}
        onclick={toggleNotch}
      >
        <Icon name={inNotch ? 'starFill' : 'star'} size={16} />
      </button>
    {/if}
  {/snippet}
  {#snippet details()}
    <Row label="Name" description="Shown in the tray and the notch instead of Windows’ name.">
      <input
        class="text"
        type="text"
        maxlength="48"
        placeholder={device.name}
        bind:value={draftAlias}
        onblur={saveAlias}
        onkeydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()}
        aria-label={`Name for ${device.name}`}
      />
    </Row>
    {#if playback}
      <Row label="Spatial sound" description="Applied every time you switch to this device.">
        <Select
          label="Spatial sound"
          value={prefs?.spatial ?? 'keep'}
          options={spatialOptions}
          onchange={(spatial) =>
            client.apply({ type: 'setDevicePrefs', deviceId: device.id, spatial, alias: null, hidden: null })}
        />
      </Row>
      <Row
        label="Lyrics delay"
        description="Shows lyrics later so they match what you hear. Bluetooth audio arrives late; Auto uses 200 ms for Bluetooth devices and 0 for others."
      >
        <Select
          label="Lyrics delay"
          value={lyricsDelay.value}
          options={lyricsDelay.options}
          onchange={(choice) =>
            client.apply({ type: 'setLyricsDelay', deviceId: device.id, delayMs: lyricsDelayFromChoice(choice) })}
        />
      </Row>
    {/if}
    <Row label="Hide this device" description="Leaves it out of the tray and the notch. You can show it again below.">
      <Toggle
        label={`Hide ${label}`}
        checked={prefs?.hidden ?? false}
        onchange={(hidden) =>
          client.apply({ type: 'setDevicePrefs', deviceId: device.id, hidden, alias: null, spatial: null })}
      />
    </Row>
  {/snippet}
</SettingCard>

<style>
  .star {
    color: var(--ice-set-fg-muted);
  }

  .star.on {
    color: var(--ice-set-accent);
  }

  .text {
    width: 240px;
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
