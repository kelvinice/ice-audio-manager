<script lang="ts" generics="T extends string">
  import type { Snippet } from 'svelte';

  interface Choice {
    value: T;
    label: string;
    description?: string;
    badge?: string;
  }

  interface Props {
    value: T;
    choices: Choice[];
    label: string;
    /** Picture above each choice's text (theme, idle style, animation). */
    preview?: Snippet<[Choice, boolean]>;
    columns?: number;
    onchange: (next: T) => void;
  }

  let { value, choices, label, preview, columns = choices.length, onchange }: Props = $props();

  const buttons: HTMLButtonElement[] = [];

  // Radio-group keyboard pattern: arrows move and select, Tab leaves the group.
  function onKey(event: KeyboardEvent, index: number): void {
    const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
    if (!step) return;
    event.preventDefault();
    const next = (index + step + choices.length) % choices.length;
    const choice = choices[next];
    if (!choice) return;
    onchange(choice.value);
    buttons[next]?.focus();
  }
</script>

<div class="group" role="radiogroup" aria-label={label} style:--cols={columns}>
  {#each choices as choice, index (choice.value)}
    {@const selected = choice.value === value}
    <button
      bind:this={buttons[index]}
      type="button"
      role="radio"
      aria-checked={selected}
      tabindex={selected ? 0 : -1}
      class:selected
      onclick={() => onchange(choice.value)}
      onkeydown={(e) => onKey(e, index)}
    >
      {#if preview}
        <span class="preview">{@render preview(choice, selected)}</span>
      {/if}
      <span class="body">
        <span class="radio" aria-hidden="true"></span>
        <span class="text">
          <span class="label">
            {choice.label}
            {#if choice.badge}<span class="badge">{choice.badge}</span>{/if}
          </span>
          {#if choice.description}<span class="desc">{choice.description}</span>{/if}
        </span>
      </span>
    </button>
  {/each}
</div>

<style>
  .group {
    display: grid;
    grid-template-columns: repeat(var(--cols), minmax(0, 1fr));
    gap: 8px;
  }

  @media (max-width: 860px) {
    .group {
      grid-template-columns: repeat(min(var(--cols), 2), minmax(0, 1fr));
    }
  }

  button {
    display: flex;
    flex-direction: column;
    padding: 0;
    text-align: left;
    border-radius: var(--ice-set-radius);
    border: 1px solid var(--ice-set-card-border);
    background: var(--ice-set-card);
    cursor: pointer;
    overflow: hidden;
    transition: background-color 120ms var(--ice-set-ease), border-color 120ms var(--ice-set-ease),
      box-shadow 120ms var(--ice-set-ease);
  }

  button:hover {
    background: var(--ice-set-card-hover);
  }

  button.selected {
    border-color: var(--ice-set-accent);
    box-shadow: inset 0 0 0 1px var(--ice-set-accent);
  }

  .preview {
    display: block;
    height: 92px;
    border-bottom: 1px solid var(--ice-set-card-border);
  }

  .body {
    display: flex;
    gap: 10px;
    padding: 12px 14px 14px;
  }

  .radio {
    flex: none;
    width: 18px;
    height: 18px;
    margin-top: 1px;
    border-radius: 50%;
    border: 1px solid var(--ice-set-control-strong);
    background: var(--ice-set-control);
    transition: border-width 120ms var(--ice-set-ease), border-color 120ms var(--ice-set-ease);
  }

  .selected .radio {
    border: 5px solid var(--ice-set-accent);
    background: var(--ice-set-accent-fg);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .label {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 8px;
    font-weight: 600;
  }

  .badge {
    font-size: 11px;
    line-height: 16px;
    font-weight: 600;
    padding: 0 6px;
    border-radius: 999px;
    color: var(--ice-set-accent);
    background: var(--ice-set-accent-soft);
  }

  .desc {
    font-size: 12px;
    line-height: 16px;
    color: var(--ice-set-fg-muted);
  }
</style>
