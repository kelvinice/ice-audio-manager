<script lang="ts">
  import type { ComponentProps } from "svelte";
  import Icon from "./Icon.svelte";

  interface MenuItem {
    label: string;
    icon?: ComponentProps<typeof Icon>["name"];
    danger?: boolean;
    disabled?: boolean;
    /** Set on choice lists: the item is a radio and the chosen one gets a check. */
    checked?: boolean;
    /** A short second line under the label. */
    hint?: string;
    onSelect: () => void;
  }

  /** "…" button with a small popover of actions; closes on outside click, Escape, or a choice. */
  interface Props {
    items: MenuItem[];
    label?: string;
    icon?: ComponentProps<typeof Icon>["name"];
    /** Trigger classes from app.css (`btn-icon` = the bare "…" look). */
    buttonClass?: string;
    /** Which edge of the trigger the menu lines up with. */
    align?: "left" | "right";
    /** Small title above the items. */
    heading?: string;
    /** Marks the trigger as switched on (accent colour). */
    active?: boolean;
  }

  let { items, label = "More options", icon = "more", buttonClass = "btn-icon", align = "left", heading, active = false }: Props =
    $props();
  let open = $state(false);
  let root: HTMLDivElement | undefined = $state();

  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === "Escape" && (open = false)} />

<div class="more" bind:this={root}>
  <button
    class={buttonClass}
    class:on={active}
    aria-label={label}
    title={label}
    aria-expanded={open}
    onclick={() => (open = !open)}
  >
    <Icon name={icon} size={icon === "more" ? 20 : 17} />
  </button>
  {#if open}
    <div class="menu" class:right={align === "right"} role="menu" aria-label={heading ?? label}>
      {#if heading}<div class="heading">{heading}</div>{/if}
      {#each items as item (item.label)}
        <button
          role={item.checked === undefined ? "menuitem" : "menuitemradio"}
          aria-checked={item.checked}
          class:danger={item.danger}
          class:checked={item.checked}
          disabled={item.disabled}
          onclick={() => {
            open = false;
            item.onSelect();
          }}
        >
          {#if item.icon}<Icon name={item.icon} size={15} />{/if}
          <span class="text">
            <span>{item.label}</span>
            {#if item.hint}<span class="hint">{item.hint}</span>{/if}
          </span>
          {#if item.checked}<Icon name="check" size={15} />{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .more {
    position: relative;
  }

  .menu {
    position: absolute;
    z-index: 30;
    top: calc(100% + 6px);
    left: 0;
    min-width: 200px;
    max-width: 260px;
    padding: 6px;
    border-radius: 10px;
    background: var(--p-surface-elevated);
    border: 1px solid var(--p-border);
    box-shadow: 0 16px 36px rgb(0 0 0 / 0.4);
  }

  .menu.right {
    left: auto;
    right: 0;
  }

  .menu button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 10px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 550;
    text-align: left;
    color: var(--p-text);
  }

  .menu button:hover {
    background: var(--p-fill);
  }

  .menu button:disabled {
    opacity: 0.45;
    pointer-events: none;
  }

  .menu button.danger {
    color: var(--p-danger);
  }

  .menu button.checked {
    color: var(--p-accent-bright);
  }

  .heading {
    padding: 6px 10px 8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--p-text-dim);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .hint {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--p-text-dim);
  }
</style>
