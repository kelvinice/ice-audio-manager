<script lang="ts">
  import type { IdleStyle, NotchStyle, ThemeKind } from '../../lib/ipc/engine';
  import DeviceIcon from '../../notch/activities/DeviceIcon.svelte';

  interface Props {
    theme: ThemeKind;
    /** `volume` = the notch mid-activity (theme cards); the rest are the idle styles. */
    mode: IdleStyle | 'volume';
    notchStyle?: NotchStyle;
  }

  let { theme, mode, notchStyle = 'floating' }: Props = $props();
  const isNook = $derived(notchStyle === 'nook');
</script>

{#snippet nookEars()}
  {#if isNook}
    <svg class="ear left" width="7" height="7" viewBox="0 0 7 7" aria-hidden="true">
      <path d="M0 0 H7 V7 A7 7 0 0 0 0 0 Z" fill="var(--ice-bg)" />
      <path d="M7 7 A7 7 0 0 0 0 0" fill="none" stroke="var(--ice-border)" stroke-width="1" />
    </svg>
    <svg class="ear right" width="7" height="7" viewBox="0 0 7 7" aria-hidden="true">
      <path d="M0 0 H7 A7 7 0 0 0 0 7 Z" fill="var(--ice-bg)" />
      <path d="M0 7 A7 7 0 0 0 7 0" fill="none" stroke="var(--ice-border)" stroke-width="1" />
    </svg>
  {/if}
{/snippet}

<!-- A slice of desktop with the notch drawn from the real `--ice-*` tokens of `theme` (base.css). -->
<div class="desk" data-theme={theme} aria-hidden="true">
  {#if mode === 'volume'}
    <div class="pill wide" class:nook={isNook}>
      {@render nookEars()}
      <DeviceIcon kind="headphones" size={11} />
      <span class="track"><span class="fill"></span></span>
      <span class="num">42</span>
    </div>
  {:else if mode === 'mini_pill'}
    <div class="pill mini" class:nook={isNook}>
      {@render nookEars()}
      <DeviceIcon kind="headphones" size={9} />
      <span class="num small">42%</span>
    </div>
  {:else if mode === 'dot'}
    <div class="pill dot" class:nook={isNook}>
      {@render nookEars()}
      <span class="led"></span>
    </div>
  {:else}
    <div class="ghost" class:nook={isNook}></div>
  {/if}
</div>

<style>
  .desk {
    position: relative;
    height: 100%;
    background: var(--ice-set-desktop);
    overflow: hidden;
  }

  .pill {
    position: absolute;
    top: 10px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--ice-fg);
    background: var(--ice-bg);
    border: 1px solid var(--ice-border);
    box-shadow: 0 0 0 0.5px var(--ice-rim) inset, var(--ice-drop-shadow), 0 0 14px var(--ice-accent-glow);
  }

  .pill.nook {
    top: 0;
    border-top: none;
    box-shadow: var(--ice-drop-shadow), 0 4px 14px var(--ice-accent-glow);
  }

  .ear {
    position: absolute;
    top: 0;
    pointer-events: none;
  }

  .ear.left {
    left: -6.5px;
  }

  .ear.right {
    right: -6.5px;
  }

  .wide {
    width: 118px;
    height: 28px;
    padding: 0 10px;
    border-radius: 14px;
  }

  .wide.nook {
    border-radius: 0 0 var(--ice-radius-md) var(--ice-radius-md);
  }

  .mini {
    width: 64px;
    height: 18px;
    border-radius: 9px;
  }

  .mini.nook {
    border-radius: 0 0 var(--ice-radius-sm) var(--ice-radius-sm);
  }

  .dot {
    width: 26px;
    height: 13px;
    border-radius: 7px;
  }

  .dot.nook {
    border-radius: 0 0 var(--ice-radius-sm) var(--ice-radius-sm);
  }

  .led {
    width: 5px;
    height: 5px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-accent);
  }

  .track {
    flex: 1;
    height: 4px;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-track);
    overflow: hidden;
  }

  .fill {
    display: block;
    width: 42%;
    height: 100%;
    border-radius: var(--ice-radius-pill);
    background: var(--ice-accent);
  }

  .num {
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .num.small {
    font-size: 9px;
  }

  .ghost {
    position: absolute;
    top: 10px;
    left: 50%;
    width: 64px;
    height: 18px;
    transform: translateX(-50%);
    border-radius: 9px;
    border: 1px dashed var(--ice-fg-muted);
  }

  .ghost.nook {
    top: 0;
    border-top: none;
    border-radius: 0 0 var(--ice-radius-sm) var(--ice-radius-sm);
  }
</style>
