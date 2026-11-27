<script lang="ts">
  import type { ComponentProps } from "svelte";
  import Icon from "./Icon.svelte";

  interface MenuItem {
    label: string;
    icon: ComponentProps<typeof Icon>["name"];
    danger?: boolean;
    disabled?: boolean;
    onSelect: () => void;
  }

  /** "…" button with a small popover of actions; closes on outside click, Escape, or a choice. */
  interface Props {
    items: MenuItem[];
    label?: string;
  }

  let { items, label = "More options" }: Props = $props();
  let open = $state(false);
  let root: HTMLDivElement | undefined = $state();

  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === "Escape" && (open = false)} />

<div class="more" bind:this={root}>
  <button class="btn-icon bare" aria-label={label} title={label} aria-expanded={open} onclick={() => (open = !open)}>
    <Icon name="more" size={20} />
  </button>
  {#if open}
    <div class="menu" role="menu">
      {#each items as item (item.label)}
        <button
          role="menuitem"
          class:danger={item.danger}
          disabled={item.disabled}
          onclick={() => {
            open = false;
            item.onSelect();
          }}
        >
          <Icon name={item.icon} size={15} />
          <span>{item.label}</span>
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
    padding: 6px;
    border-radius: 10px;
    background: var(--p-surface-elevated);
    border: 1px solid var(--p-border);
    box-shadow: 0 16px 36px rgb(0 0 0 / 0.4);
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
</style>
