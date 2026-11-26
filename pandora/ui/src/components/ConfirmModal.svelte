<script lang="ts">
  import Icon from "./Icon.svelte";

  interface Props {
    open: boolean;
    title: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    danger?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let {
    open,
    title,
    message,
    confirmLabel = "Delete",
    cancelLabel = "Cancel",
    danger = true,
    onConfirm,
    onCancel,
  }: Props = $props();
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={onCancel}
    onkeydown={(e) => {
      if (e.key === "Escape") onCancel();
    }}
  >
    <div
      class="modal card-glass"
      role="alertdialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === "Escape") onCancel();
      }}
    >
      <div class="head">
        <div class="icon-badge" class:danger>
          <Icon name={danger ? "trash" : "sparkles"} size={18} />
        </div>
        <div class="text-col">
          <h3>{title}</h3>
          <p>{message}</p>
        </div>
      </div>

      <div class="foot">
        <button type="button" class="btn-ghost" onclick={onCancel}>{cancelLabel}</button>
        <button
          type="button"
          class={danger ? "btn-danger" : "btn-primary"}
          onclick={onConfirm}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 65;
    background: rgba(0, 0, 0, 0.62);
    display: grid;
    place-items: center;
    padding: 20px;
  }

  .modal {
    width: 400px;
    max-width: 94vw;
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 18px;
    background: var(--p-surface-solid);
  }

  .head {
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }

  .icon-badge {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .icon-badge.danger {
    background: rgba(239, 68, 68, 0.16);
    border: 1px solid rgba(239, 68, 68, 0.32);
    color: var(--p-danger);
  }

  .text-col {
    min-width: 0;
  }

  h3 {
    margin: 0 0 6px;
    font-size: 16px;
    color: var(--p-text);
  }

  p {
    margin: 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--p-text-muted);
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn-danger {
    padding: 8px 16px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 600;
    background: rgba(239, 68, 68, 0.2);
    border: 1px solid rgba(239, 68, 68, 0.42);
    color: var(--p-danger);
    transition: background 0.16s ease, transform 0.12s ease;
  }

  .btn-danger:hover {
    background: rgba(239, 68, 68, 0.38);
  }

  .btn-danger:active {
    transform: scale(0.96);
  }
</style>
