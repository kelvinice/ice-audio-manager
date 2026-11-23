<script lang="ts">
  import { modifierParts, recordKeys } from '../../lib/settings/model';
  import KeyCaps from './KeyCaps.svelte';

  interface Props {
    value: string | null;
    label: string;
    disabled?: boolean;
    onchange: (next: string | null) => void;
  }

  let { value, label, disabled = false, onchange }: Props = $props();

  let recording = $state(false);
  let held = $state<string[]>([]);
  let hint = $state<string | null>(null);
  let field = $state<HTMLDivElement | null>(null);

  function start(): void {
    recording = true;
    held = [];
    hint = null;
    queueMicrotask(() => field?.focus());
  }

  function stop(): void {
    recording = false;
    held = [];
  }

  // Keys typed into this focused window only: no hook is involved (code-practices §1). A chord another app (or this
  // app) already registered never reaches the page; the engine then reports the conflict.
  function onKeyDown(event: KeyboardEvent): void {
    event.preventDefault();
    event.stopPropagation();
    const plain = !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey;
    if (plain && event.key === 'Escape') return stop();
    if (plain && (event.key === 'Backspace' || event.key === 'Delete')) {
      stop();
      if (value) onchange(null);
      return;
    }
    const result = recordKeys(event);
    if (result.kind === 'chord') {
      stop();
      if (result.chord !== value) onchange(result.chord);
    } else {
      held = result.parts;
      hint = result.kind === 'invalid' ? result.reason : null;
    }
  }

  function onKeyUp(event: KeyboardEvent): void {
    held = modifierParts(event);
    if (!held.length) hint = null;
  }
</script>

<div class="wrap">
  <div class="hotkey">
    {#if recording}
      <div
        bind:this={field}
        class="field recording"
        role="textbox"
        tabindex="0"
        aria-label={`${label}: press the new shortcut`}
        onkeydown={onKeyDown}
        onkeyup={onKeyUp}
        onblur={stop}
      >
        {#if held.length}
          <KeyCaps chord={held.join('+')} />
        {:else}
          <span class="prompt">Press a shortcut</span>
        {/if}
      </div>
    {:else}
      <div class="field"><KeyCaps chord={value} /></div>
    {/if}
    <!-- mousedown is cancelled while recording so the field's blur doesn't stop it before the click lands. -->
    <button
      type="button"
      class="btn"
      {disabled}
      onmousedown={(e) => recording && e.preventDefault()}
      onclick={recording ? stop : start}
    >
      {recording ? 'Cancel' : value ? 'Change' : 'Set'}
    </button>
  </div>
  {#if recording}
    <p class="help" class:warn={hint}>{hint ?? 'Esc cancels · Backspace removes the shortcut'}</p>
  {/if}
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .hotkey {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .field {
    min-width: 168px;
    height: 32px;
    padding: 0 10px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    border-radius: var(--ice-set-radius-sm);
  }

  .recording {
    justify-content: center;
    border: 1px solid var(--ice-set-control-border);
    border-bottom: 2px solid var(--ice-set-accent);
    background: var(--ice-set-control);
    outline: none;
  }

  .prompt {
    font-size: 12px;
    color: var(--ice-set-fg-muted);
  }

  .help {
    max-width: 260px;
    margin: 6px 0 0;
    font-size: 12px;
    line-height: 16px;
    text-align: right;
    color: var(--ice-set-fg-muted);
  }

  .help.warn {
    color: var(--ice-set-warn);
  }
</style>
