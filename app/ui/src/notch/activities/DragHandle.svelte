<script lang="ts">
  // Owner, 2026-11-27: a small grabber on the expanded card so people feel safe dragging it. Purely a cue: it takes no
  // pointer events, so the drag area stays exactly as before (anywhere that is not a control, `notch/drag.ts`).
  interface Props {
    /** The card edge away from the dock: `bottom` for a top-anchored notch, `top` for a bottom-anchored one. */
    edge: 'top' | 'bottom';
    /** Experimental `appearance.experiments.dragHandle`, and off when `notchPlacement` is `fixed` (nothing to drag). */
    visible: boolean;
  }

  let { edge, visible }: Props = $props();
</script>

{#if visible}
  <span class="drag-handle" class:top={edge === 'top'} aria-hidden="true"></span>
{/if}

<style>
  .drag-handle {
    position: absolute;
    left: 50%;
    bottom: 3px;
    width: 32px;
    height: 4px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-fg-muted);
    opacity: 0.45;
    transform: translateX(-50%);
    pointer-events: none;
    transition: opacity 120ms ease;
  }

  .drag-handle.top {
    top: 3px;
    bottom: auto;
  }

  /* Hovering the card brightens the cue a little; the press/lift feedback itself is unchanged. */
  :global(:hover) > .drag-handle {
    opacity: 0.7;
  }
</style>
