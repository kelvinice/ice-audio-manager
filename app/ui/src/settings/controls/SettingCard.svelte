<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { IconName } from '../icons';
  import Icon from './Icon.svelte';

  interface Props {
    title: string;
    description?: string;
    icon?: IconName;
    /** A custom leading visual (device icon), used instead of `icon`. */
    leading?: Snippet;
    /** The control on the right (toggle, select, button). */
    control?: Snippet;
    /** Extra rows shown under the header; with them the header becomes an expander. */
    details?: Snippet;
    expanded?: boolean;
    disabled?: boolean;
  }

  let {
    title,
    description,
    icon,
    leading,
    control,
    details,
    expanded = $bindable(false),
    disabled = false,
  }: Props = $props();

  const detailsId = `card-${Math.random().toString(36).slice(2, 9)}`;
</script>

<div class="card" class:disabled class:open={expanded && details}>
  <div class="head">
    {#if details}
      <button
        type="button"
        class="expander"
        aria-expanded={expanded}
        aria-controls={detailsId}
        onclick={() => (expanded = !expanded)}
      >
        {@render body()}
      </button>
    {:else}
      <div class="static">{@render body()}</div>
    {/if}
    {#if control}
      <div class="control">{@render control()}</div>
    {/if}
    {#if details}
      <button
        type="button"
        class="chevron"
        tabindex="-1"
        aria-hidden="true"
        onclick={() => (expanded = !expanded)}
      >
        <Icon name="chevronDown" size={12} />
      </button>
    {/if}
  </div>
  {#if details && expanded}
    <div class="details" id={detailsId}>{@render details()}</div>
  {/if}
</div>

{#snippet body()}
  {#if leading}
    <span class="lead">{@render leading()}</span>
  {:else if icon}
    <span class="lead"><Icon name={icon} size={20} /></span>
  {/if}
  <span class="text">
    <span class="title">{title}</span>
    {#if description}<span class="desc">{description}</span>{/if}
  </span>
{/snippet}

<style>
  .card {
    background: var(--ice-set-card);
    border: 1px solid var(--ice-set-card-border);
    border-radius: var(--ice-set-radius);
    transition: background-color 120ms var(--ice-set-ease);
  }

  .card:has(> .head > .expander:hover),
  .card:has(> .head > .chevron:hover) {
    background: var(--ice-set-card-hover);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 68px;
    padding: 0 16px;
  }

  .expander,
  .static {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 0;
    text-align: left;
    background: none;
    border: 0;
  }

  .expander {
    cursor: pointer;
  }

  .lead {
    width: 24px;
    display: inline-flex;
    justify-content: center;
    color: var(--ice-set-fg);
    flex: none;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .title {
    font-size: 14px;
    line-height: 20px;
  }

  .desc {
    font-size: 12px;
    line-height: 16px;
    color: var(--ice-set-fg-muted);
  }

  .control {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .chevron {
    flex: none;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--ice-set-radius-sm);
    background: none;
    color: var(--ice-set-fg-muted);
    cursor: pointer;
    transition: transform 160ms var(--ice-set-ease);
  }

  .chevron:hover {
    background: var(--ice-set-control-hover);
  }

  .open .chevron {
    transform: rotate(180deg);
  }

  .details {
    border-top: 1px solid var(--ice-set-divider);
    padding: 4px 16px 4px 56px;
  }

  .disabled .text,
  .disabled .lead {
    opacity: 0.5;
  }
</style>
