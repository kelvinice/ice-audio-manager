<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    description?: string;
    children: Snippet;
  }

  let { title, description, children }: Props = $props();
</script>

<section>
  <h2>{title}</h2>
  {#if description}<p>{description}</p>{/if}
  <div class="stack">{@render children()}</div>
</section>

<style>
  section {
    margin-top: 28px;
  }

  section:first-child {
    margin-top: 0;
  }

  h2 {
    margin: 0 0 4px;
    font-size: 14px;
    line-height: 20px;
    font-weight: 600;
  }

  p {
    margin: 0 0 8px;
    font-size: 12px;
    line-height: 16px;
    color: var(--ice-set-fg-muted);
  }

  .stack {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 8px;
  }
</style>
