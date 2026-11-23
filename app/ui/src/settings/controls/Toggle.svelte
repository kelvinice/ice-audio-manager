<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    onchange: (next: boolean) => void;
  }

  let { checked, label, disabled = false, onchange }: Props = $props();
</script>

<div class="toggle" class:disabled>
  <span class="state" aria-hidden="true">{checked ? 'On' : 'Off'}</span>
  <button
    type="button"
    role="switch"
    aria-checked={checked}
    aria-label={label}
    {disabled}
    class:on={checked}
    onclick={() => onchange(!checked)}
  >
    <span class="knob"></span>
  </button>
</div>

<style>
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 12px;
  }

  .state {
    min-width: 22px;
    text-align: right;
    color: var(--ice-set-fg);
  }

  button {
    position: relative;
    width: 40px;
    height: 20px;
    padding: 0;
    border-radius: 10px;
    border: 1px solid var(--ice-set-control-strong);
    background: var(--ice-set-control);
    cursor: pointer;
    transition: background-color 120ms var(--ice-set-ease), border-color 120ms var(--ice-set-ease);
  }

  button:hover {
    background: var(--ice-set-control-hover);
  }

  .knob {
    position: absolute;
    top: 50%;
    left: 4px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--ice-set-control-strong);
    transform: translateY(-50%);
    transition: transform 160ms var(--ice-set-ease), width 120ms var(--ice-set-ease), height 120ms var(--ice-set-ease);
  }

  button:hover .knob {
    width: 14px;
    height: 14px;
    left: 3px;
  }

  button.on {
    background: var(--ice-set-accent);
    border-color: var(--ice-set-accent);
  }

  button.on:hover {
    background: var(--ice-set-accent-hover);
  }

  button.on .knob {
    background: var(--ice-set-accent-fg);
    transform: translate(20px, -50%);
  }

  button.on:hover .knob {
    transform: translate(19px, -50%);
  }

  .disabled {
    opacity: 0.45;
  }

  .disabled button {
    cursor: default;
  }
</style>
