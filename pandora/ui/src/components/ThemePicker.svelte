<script lang="ts">
  import type { PandoraTheme } from "../lib/types";

  let { theme, onSelect }: { theme: PandoraTheme; onSelect: (value: PandoraTheme) => void } = $props();
  const choices = [
    { id: "midnight", name: "Midnight" },
    { id: "ice", name: "Ice" },
    { id: "daylight", name: "Daylight" },
    { id: "ember", name: "Ember" },
    { id: "system", name: "Use Windows" },
  ] as const;
</script>

<fieldset class="picker">
  <legend>Theme</legend>
  <div class="choices">
    {#each choices as choice}
      <label class="option">
        <input type="radio" name="pandora-theme" value={choice.id}
          checked={theme === choice.id} onchange={() => onSelect(choice.id)} />
        <span class="theme-preview" data-theme={choice.id} aria-hidden="true">
          <span class="rail"></span><span class="record"></span><span class="lines"></span>
        </span>
        <span class="caption">{choice.name}<span class="indicator" aria-hidden="true"></span></span>
      </label>
    {/each}
  </div>
</fieldset>

<style>
  .picker { margin: 0; padding: 0; border: 0; min-width: 0; }
  legend { padding: 0; margin-bottom: 12px; font-size: 17px; font-weight: 650; }
  .choices { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 12px; }
  .option { position: relative; display: grid; overflow: hidden; border: 1px solid var(--p-border);
    border-radius: 12px; background: var(--p-surface); cursor: pointer; }
  .option:has(:checked) { outline: 2px solid var(--p-accent); outline-offset: -2px; }
  .option:has(:focus-visible) { outline: 3px solid var(--p-accent); outline-offset: 3px; }
  input { position: absolute; width: 1px; height: 1px; opacity: 0; }
  .theme-preview { --art-primary: initial; --art-secondary: initial; --art-tertiary: initial;
    margin: 6px 6px 0; position: relative; height: 60px; overflow: hidden; border-radius: 7px;
    background: var(--p-bg); border: 1px solid var(--p-border); }
  .rail { position: absolute; inset: 0 auto 0 0; width: 16%; background: var(--p-rail); border-right: 1px solid var(--p-border); }
  .record { position: absolute; top: 18%; left: 23%; height: 64%; aspect-ratio: 1; border-radius: 50%;
    background: radial-gradient(circle, var(--p-rail) 8%, var(--p-accent) 9% 24%, transparent 25%),
      repeating-radial-gradient(circle, var(--p-surface-elevated) 0 3px, var(--p-border-strong) 4px 5px); }
  .lines { position: absolute; top: 38%; left: calc(23% + 46px); right: 9%; height: 4px;
    background: var(--p-accent); box-shadow: 0 9px var(--p-border-strong); }
  .caption { display: flex; justify-content: space-between; align-items: center; gap: 4px; padding: 10px 12px; font-size: 13px; font-weight: 650; }
  .indicator { width: 12px; height: 12px; border: 1px solid var(--p-border-strong); border-radius: 50%; }
  :checked ~ .caption .indicator { background: var(--p-accent); box-shadow: inset 0 0 0 2px var(--p-surface); border-color: var(--p-accent); }
  @media (max-width: 960px) { .choices { gap: 8px; } }
</style>
