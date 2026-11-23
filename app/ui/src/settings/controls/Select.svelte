<script lang="ts" generics="T extends string">
  interface Option {
    value: T;
    label: string;
  }

  interface Props {
    value: T;
    options: Option[];
    label: string;
    disabled?: boolean;
    onchange: (next: T) => void;
  }

  let { value, options, label, disabled = false, onchange }: Props = $props();
</script>

<!-- A native select: keyboard, screen readers and the drop-down list come from Windows for free. -->
<div class="select" class:disabled>
  <select
    aria-label={label}
    {disabled}
    {value}
    onchange={(e) => onchange((e.currentTarget as HTMLSelectElement).value as T)}
  >
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
  <span class="chev" aria-hidden="true"></span>
</div>

<style>
  .select {
    position: relative;
    display: inline-flex;
  }

  select {
    appearance: none;
    width: 240px;
    height: 32px;
    padding: 0 34px 0 11px;
    border-radius: var(--ice-set-radius-sm);
    border: 1px solid var(--ice-set-control-border);
    background: var(--ice-set-control);
    cursor: pointer;
    text-overflow: ellipsis;
    transition: background-color 120ms var(--ice-set-ease);
  }

  select:hover {
    background: var(--ice-set-control-hover);
  }

  select option {
    background: var(--ice-set-solid-bg);
    color: var(--ice-set-fg);
  }

  .chev {
    position: absolute;
    right: 12px;
    top: 50%;
    width: 7px;
    height: 7px;
    border-right: 1.5px solid var(--ice-set-fg-muted);
    border-bottom: 1.5px solid var(--ice-set-fg-muted);
    transform: translateY(-70%) rotate(45deg);
    pointer-events: none;
  }

  .disabled {
    opacity: 0.45;
  }
</style>
