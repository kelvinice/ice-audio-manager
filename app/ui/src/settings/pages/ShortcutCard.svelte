<script lang="ts">
  import { untrack } from 'svelte';
  import type { BindingView, Command, DeviceSummary, RoleSet, TogglePairView } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import { deviceLabel, nextId, savedLabel } from '../../lib/settings/model';
  import DeviceIcon from '../../notch/activities/DeviceIcon.svelte';
  import HotkeyField from '../controls/HotkeyField.svelte';
  import Icon from '../controls/Icon.svelte';
  import KeyCaps from '../controls/KeyCaps.svelte';
  import Row from '../controls/Row.svelte';
  import Select from '../controls/Select.svelte';
  import SettingCard from '../controls/SettingCard.svelte';

  /** A toggle pair (two devices) or a device shortcut (one). Without `pair`/`binding` it's a new, unsaved draft. */
  interface Props {
    client: SettingsClient;
    kind: 'pair' | 'binding';
    pair?: TogglePairView;
    binding?: BindingView;
    ondone?: () => void;
  }

  let { client, kind, pair, binding, ondone }: Props = $props();

  type Fields = { a: string | null; b: string | null; hotkey: string | null; roles: RoleSet | null };
  type RoleChoice = RoleSet | 'inherit';

  const engine = $derived(client.engine);
  const settings = $derived(engine?.settings ?? null);
  const all = $derived<DeviceSummary[]>([...(engine?.playback ?? []), ...(engine?.recording ?? [])]);
  const isDraft = $derived(!pair && !binding);

  let draft = $state<Fields>({ a: null, b: null, hotkey: null, roles: null });
  // A new draft opens expanded; a saved one starts closed.
  let expanded = $state(untrack(() => !pair && !binding));

  const fields = $derived<Fields>(
    pair
      ? { a: pair.primary.deviceId, b: pair.secondary.deviceId, hotkey: pair.hotkey, roles: pair.roles }
      : binding
        ? { a: binding.device.deviceId, b: null, hotkey: binding.hotkey, roles: binding.roles }
        : draft,
  );

  const savedDevices = $derived(pair ? [pair.primary, pair.secondary] : binding ? [binding.device] : []);
  const missing = $derived(savedDevices.filter((d) => !d.deviceId));
  const flowOfA = $derived(all.find((d) => d.id === fields.a)?.flow ?? savedDevices[0]?.flow ?? null);

  const liveLabel = (id: string | null) => {
    const d = all.find((x) => x.id === id);
    return d ? deviceLabel(d, settings) : null;
  };

  const title = $derived.by(() => {
    if (pair) return `${savedLabel(pair.primary, settings)}  ⇄  ${savedLabel(pair.secondary, settings)}`;
    if (binding) return `Switch to ${savedLabel(binding.device, settings)}`;
    return kind === 'pair' ? 'New toggle pair' : 'New device shortcut';
  });

  const description = $derived(
    missing.length
      ? `${missing.map((d) => savedLabel(d, settings)).join(' and ')} ${missing.length > 1 ? 'aren’t' : 'isn’t'} connected. The shortcut works again once ${missing.length > 1 ? 'they’re' : 'it’s'} back; connect ${missing.length > 1 ? 'them' : 'it'} to edit.`
      : undefined,
  );

  const placeholder = { value: '', label: 'Choose a device' };
  const optionsA = $derived([...(fields.a ? [] : [placeholder]), ...all.map((d) => ({ value: d.id, label: deviceLabel(d, settings) }))]);
  const optionsB = $derived([
    ...(fields.b ? [] : [placeholder]),
    ...all.filter((d) => d.flow === flowOfA && d.id !== fields.a).map((d) => ({ value: d.id, label: deviceLabel(d, settings) })),
  ]);

  const roleOptions: { value: RoleChoice; label: string }[] = [
    { value: 'inherit', label: 'Same as General' },
    { value: 'default', label: 'Default device' },
    { value: 'communications', label: 'Communications device' },
    { value: 'both', label: 'Both' },
  ];

  function complete(f: Fields): f is Fields & { a: string } {
    return Boolean(f.a && (kind === 'binding' || (f.b && f.b !== f.a)));
  }

  function command(id: string, f: Fields & { a: string }): Command {
    return kind === 'pair'
      ? { type: 'setTogglePair', pairId: id, primaryId: f.a, secondaryId: f.b ?? '', hotkey: f.hotkey, roles: f.roles }
      : { type: 'setBinding', bindingId: id, deviceId: f.a, hotkey: f.hotkey, roles: f.roles };
  }

  function update(patch: Partial<Fields>): void {
    const next = { ...fields, ...patch };
    if (patch.a !== undefined && kind === 'pair' && next.b === next.a) next.b = null;
    if (isDraft) {
      draft = next;
      return;
    }
    const id = pair?.id ?? binding?.id;
    if (id && complete(next)) client.apply(command(id, next));
  }

  // The draft stays open if the engine refuses it (e.g. a shortcut already in use), so nothing typed is lost.
  async function create(): Promise<void> {
    if (!complete(draft) || !settings) return;
    const taken = kind === 'pair' ? settings.togglePairs.map((p) => p.id) : settings.bindings.map((b) => b.id);
    if (await client.apply(command(nextId(kind === 'pair' ? 'pair' : 'shortcut', taken), draft))) ondone?.();
  }

  function remove(): void {
    if (pair) client.apply({ type: 'removeTogglePair', pairId: pair.id });
    if (binding) client.apply({ type: 'removeBinding', bindingId: binding.id });
  }
</script>

<SettingCard {title} {description} bind:expanded>
  {#snippet leading()}
    {#if kind === 'pair'}
      <Icon name="switch" size={20} />
    {:else}
      <DeviceIcon kind="auto" formFactor={all.find((d) => d.id === fields.a)?.formFactor ?? null} size={20} />
    {/if}
  {/snippet}
  {#snippet control()}
    {#if !isDraft}<KeyCaps chord={fields.hotkey} empty="No shortcut" />{/if}
  {/snippet}
  {#snippet details()}
    {@const locked = missing.length > 0}
    <Row label={kind === 'pair' ? 'First device' : 'Device'}>
      <Select
        label={kind === 'pair' ? 'First device' : 'Device'}
        value={fields.a ?? ''}
        options={optionsA}
        disabled={locked}
        onchange={(a) => update({ a: a || null })}
      />
    </Row>
    {#if kind === 'pair'}
      <Row label="Second device" description={liveLabel(fields.a) ? 'Same kind as the first: both playback or both microphones.' : undefined}>
        <Select
          label="Second device"
          value={fields.b ?? ''}
          options={optionsB}
          disabled={locked || !fields.a}
          onchange={(b) => update({ b: b || null })}
        />
      </Row>
    {/if}
    <Row label="Shortcut" description="Works everywhere in Windows.">
      <HotkeyField label="Shortcut" value={fields.hotkey} disabled={locked} onchange={(hotkey) => update({ hotkey })} />
    </Row>
    <Row label="Changes" description="Which Windows default this switch sets.">
      <Select
        label="Changes"
        value={fields.roles ?? 'inherit'}
        options={roleOptions}
        disabled={locked}
        onchange={(r) => update({ roles: r === 'inherit' ? null : r })}
      />
    </Row>
    <div class="footer">
      {#if isDraft}
        <button type="button" class="btn" onclick={() => ondone?.()}>Cancel</button>
        <button type="button" class="btn accent" disabled={!complete(draft)} onclick={create}>
          {kind === 'pair' ? 'Create pair' : 'Create shortcut'}
        </button>
      {:else}
        <button type="button" class="btn danger" onclick={remove}>
          <Icon name="delete" size={14} />
          Remove
        </button>
      {/if}
    </div>
  {/snippet}
</SettingCard>

<style>
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 0;
    border-top: 1px solid var(--ice-set-divider);
  }
</style>
