<script lang="ts">
  import { chordParts } from '../../lib/settings/model';

  interface Props {
    chord: string | null;
    /** Shown when there is no chord. */
    empty?: string;
  }

  let { chord, empty = 'Not set' }: Props = $props();
  const parts = $derived(chordParts(chord));
</script>

{#if parts.length}
  <span class="caps" aria-label={parts.join(' plus ')}>
    {#each parts as part, i (i)}
      {#if i > 0}<span class="plus" aria-hidden="true">+</span>{/if}
      <kbd>{part}</kbd>
    {/each}
  </span>
{:else}
  <span class="empty">{empty}</span>
{/if}

<style>
  .caps {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  kbd {
    font-family: var(--ice-font);
    font-size: 12px;
    line-height: 20px;
    min-width: 24px;
    padding: 1px 7px;
    text-align: center;
    border-radius: var(--ice-set-radius-sm);
    border: 1px solid var(--ice-set-keycap-border);
    border-bottom-width: 2px;
    background: var(--ice-set-keycap);
  }

  .plus {
    font-size: 11px;
    color: var(--ice-set-fg-subtle);
  }

  .empty {
    font-size: 12px;
    color: var(--ice-set-fg-subtle);
  }
</style>
