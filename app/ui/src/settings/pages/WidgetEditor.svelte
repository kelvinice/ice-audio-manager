<script lang="ts">
  // Experimental notch widget (owner, 2026-11-27): switch it on, then pick and order up to five quick buttons. The
  // notch shows them in the card's side rail (Floating) or the toolbar capsule (Edge Nook).
  import type { WidgetButton } from '../../lib/ipc/engine';
  import type { SettingsClient } from '../../lib/settings/client.svelte';
  import { ALL_WIDGET_BUTTONS, MAX_WIDGET_BUTTONS, WIDGET_BUTTONS, addWidgetButton } from '../../lib/notch/widget';
  import { moveItem } from '../../lib/settings/model';
  import DeviceIcon from '../../notch/activities/DeviceIcon.svelte';
  import Icon from '../controls/Icon.svelte';
  import InfoBar from '../controls/InfoBar.svelte';
  import Section from '../controls/Section.svelte';
  import SettingCard from '../controls/SettingCard.svelte';
  import Toggle from '../controls/Toggle.svelte';

  interface Props {
    client: SettingsClient;
  }

  let { client }: Props = $props();
  const widget = $derived(client.engine?.settings.widget);
  const buttons = $derived(widget?.buttons ?? []);
  const unused = $derived(ALL_WIDGET_BUTTONS.filter((b) => !buttons.includes(b)));
  const full = $derived(buttons.length >= MAX_WIDGET_BUTTONS);
  const usesPandora = $derived(buttons.some((b) => b === 'pandora_recognize' || b === 'pandora_open'));

  function save(next: WidgetButton[] | null): void {
    if (next && next !== buttons) void client.apply({ type: 'setNotchWidget', enabled: null, buttons: next });
  }

  function setEnabled(on: boolean): void {
    void client.apply({ type: 'setNotchWidget', enabled: on, buttons: null });
  }
</script>

{#if widget}
  <Section
    title="Notch widget"
    description={`Choose up to ${MAX_WIDGET_BUTTONS} quick buttons and their order. They replace the buttons beside the open card (Floating) or in its toolbar (Edge Nook).`}
  >
    <SettingCard
      icon="flask"
      title="Custom notch buttons"
      description="Off: the notch keeps its usual buttons."
    >
      {#snippet control()}
        <Toggle label="Custom notch buttons" checked={widget.enabled} onchange={setEnabled} />
      {/snippet}
    </SettingCard>

    <ol class="order" class:off={!widget.enabled} aria-label="Widget buttons, in order">
      {#each buttons as b, index (b)}
        {@const info = WIDGET_BUTTONS[b]}
        <li>
          <span class="pos">{index + 1}</span>
          <span class="ico"><DeviceIcon kind={info.icon} size={16} /></span>
          <span class="name">
            {info.label}
            <span class="muted">{info.description}</span>
          </span>
          <span class="actions">
            <button type="button" class="icon-btn" aria-label={`Move ${info.label} up`} disabled={index === 0} onclick={() => save(moveItem(buttons, index, -1))}>
              <Icon name="up" size={12} />
            </button>
            <button
              type="button"
              class="icon-btn"
              aria-label={`Move ${info.label} down`}
              disabled={index === buttons.length - 1}
              onclick={() => save(moveItem(buttons, index, 1))}
            >
              <Icon name="down" size={12} />
            </button>
            <button type="button" class="icon-btn" aria-label={`Remove ${info.label}`} onclick={() => save(buttons.filter((x) => x !== b))}>
              <Icon name="close" size={12} />
            </button>
          </span>
        </li>
      {:else}
        <li class="empty">No buttons yet. Add some below.</li>
      {/each}
    </ol>

    {#if unused.length > 0}
      <div class="add" role="group" aria-label="Add a button">
        {#each unused as b (b)}
          {@const info = WIDGET_BUTTONS[b]}
          <button
            type="button"
            class="chip"
            disabled={full}
            title={full ? `The widget holds ${MAX_WIDGET_BUTTONS} buttons. Remove one first.` : info.description}
            onclick={() => save(addWidgetButton(buttons, b))}
          >
            <Icon name="add" size={10} />
            <DeviceIcon kind={info.icon} size={13} />
            <span>{info.label}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if usesPandora}
      <InfoBar
        tone="info"
        title="Pandora buttons"
        text="They start the Pandora player if it isn't running (run it once so Ice can find it). Recognise song listens to what this PC plays, never the microphone, and Pandora looks the song up online. Ice only talks to Pandora on this PC."
      />
    {/if}
  </Section>
{/if}

<style>
  .order {
    list-style: none;
    margin: 0;
    padding: 0;
    border-radius: var(--ice-set-radius);
    border: 1px solid var(--ice-set-card-border);
    background: var(--ice-set-card);
  }

  .order.off {
    opacity: 0.6;
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

  li.empty {
    color: var(--ice-set-fg-muted);
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

  .add {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    appearance: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px 0 10px;
    border: 1px solid var(--ice-set-control-border);
    border-radius: var(--ice-set-radius);
    background: var(--ice-set-control);
    color: var(--ice-set-fg);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .chip:hover:not(:disabled) {
    background: var(--ice-set-control-hover);
  }

  .chip:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
