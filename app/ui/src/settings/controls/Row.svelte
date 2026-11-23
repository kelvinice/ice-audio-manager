<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    description?: string;
    children: Snippet;
  }

  let { label, description, children }: Props = $props();
</script>

<div class="row">
  <div class="text">
    <span class="label">{label}</span>
    {#if description}<span class="desc">{description}</span>{/if}
  </div>
  <div class="control">{@render children()}</div>
</div>

<style>
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
    min-height: 56px;
    padding: 10px 0;
  }

  .row + :global(.row) {
    border-top: 1px solid var(--ice-set-divider);
  }

  .text {
    flex: 1 1 200px;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .desc {
    font-size: 12px;
    line-height: 16px;
    color: var(--ice-set-fg-muted);
  }

  .control {
    flex: none;
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
