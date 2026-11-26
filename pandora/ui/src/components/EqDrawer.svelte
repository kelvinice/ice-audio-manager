<script lang="ts">
  import { audioEngine, EQ_FREQUENCIES, EQ_PRESETS, playerStore } from "../lib/audio";

  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open, onClose }: Props = $props();

  function applyPreset(name: string) {
    const bands = EQ_PRESETS[name] ?? EQ_PRESETS.Flat;
    audioEngine.updateEq({ ...$playerStore.eq, preset: name, bands: [...bands] });
  }

  function setBand(index: number, db: number) {
    const bands = [...$playerStore.eq.bands];
    bands[index] = db;
    audioEngine.updateEq({ ...$playerStore.eq, preset: "Custom", bands });
  }

  function freqLabel(hz: number): string {
    return hz >= 1000 ? `${hz / 1000}k` : `${hz}`;
  }
</script>

{#if open}
  <div class="backdrop" role="presentation" onclick={onClose}>
    <div
      class="drawer card-glass"
      role="dialog"
      aria-modal="true"
      aria-label="Audio DSP and Equalizer"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => { if (e.key === "Escape") onClose(); }}
    >
      <div class="head">
        <div>
          <h3>Audio DSP &amp; Equalizer</h3>
          <p>Web Audio 10-band EQ • Normalization • Skip Silence</p>
        </div>
        <button class="btn-ghost" onclick={onClose}>Done</button>
      </div>

      <div class="toggles">
        <label class="toggle-row">
          <span>Enable Equalizer</span>
          <input
            type="checkbox"
            checked={$playerStore.eq.enabled}
            onchange={(e) =>
              audioEngine.updateEq({
                ...$playerStore.eq,
                enabled: (e.target as HTMLInputElement).checked,
              })}
          />
        </label>
        <label class="toggle-row">
          <span>Audio Normalization (Compressor)</span>
          <input
            type="checkbox"
            checked={$playerStore.eq.normalize}
            onchange={(e) =>
              audioEngine.updateEq({
                ...$playerStore.eq,
                normalize: (e.target as HTMLInputElement).checked,
              })}
          />
        </label>
        <label class="toggle-row">
          <span>Skip Silence (RMS gate)</span>
          <input
            type="checkbox"
            checked={$playerStore.eq.skipSilence}
            onchange={(e) =>
              audioEngine.updateEq({
                ...$playerStore.eq,
                skipSilence: (e.target as HTMLInputElement).checked,
              })}
          />
        </label>
      </div>

      <div class="section-label">Presets</div>
      <div class="presets">
        {#each Object.keys(EQ_PRESETS) as preset}
          <button
            class="preset-chip"
            class:active={$playerStore.eq.preset === preset}
            onclick={() => applyPreset(preset)}
          >
            {preset}
          </button>
        {/each}
      </div>

      <div class="section-label">10-Band Graphic EQ (-12 dB to +12 dB)</div>
      <div class="bands">
        {#each EQ_FREQUENCIES as hz, idx}
          <div class="band">
            <span class="db">{$playerStore.eq.bands[idx] > 0 ? `+${$playerStore.eq.bands[idx]}` : $playerStore.eq.bands[idx]}</span>
            <input
              type="range"
              min="-12"
              max="12"
              step="0.5"
              value={$playerStore.eq.bands[idx]}
              oninput={(e) => setBand(idx, Number((e.target as HTMLInputElement).value))}
            />
            <span class="hz">{freqLabel(hz)}</span>
          </div>
        {/each}
      </div>

      <div class="section-label">Playback Speed ({$playerStore.eq.playbackRate.toFixed(2)}×)</div>
      <input
        class="speed-slider"
        type="range"
        min="0.75"
        max="1.5"
        step="0.05"
        value={$playerStore.eq.playbackRate}
        oninput={(e) =>
          audioEngine.updateEq({
            ...$playerStore.eq,
            playbackRate: Number((e.target as HTMLInputElement).value),
          })}
      />

      <div class="section-label">
        Sleep Timer
        {#if $playerStore.sleepRemainingSec !== null}
          • {Math.ceil($playerStore.sleepRemainingSec / 60)} min left
        {/if}
      </div>
      <div class="presets">
        <button class="preset-chip" onclick={() => audioEngine.setSleepTimer(null)}>Off</button>
        <button class="preset-chip" onclick={() => audioEngine.setSleepTimer(15)}>15m</button>
        <button class="preset-chip" onclick={() => audioEngine.setSleepTimer(30)}>30m</button>
        <button class="preset-chip" onclick={() => audioEngine.setSleepTimer(60)}>60m</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    justify-content: flex-end;
  }

  .drawer {
    width: 410px;
    max-width: 92vw;
    height: 100%;
    border-radius: 20px 0 0 20px;
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow-y: auto;
    background: rgba(14, 18, 30, 0.96);
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .head h3 {
    margin: 0;
    font-size: 18px;
  }

  .head p {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--p-text-muted);
  }

  .toggles {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.05);
    font-size: 13px;
    cursor: pointer;
  }

  .toggle-row input {
    accent-color: var(--p-accent);
    width: 16px;
    height: 16px;
  }

  .section-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--p-text-dim);
    margin-top: 4px;
  }

  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .preset-chip {
    padding: 6px 11px;
    border-radius: 999px;
    font-size: 12px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--p-border);
  }

  .preset-chip.active {
    background: var(--p-accent-soft);
    color: var(--p-accent-bright);
    border-color: var(--p-accent);
  }

  .bands {
    display: grid;
    grid-template-columns: repeat(10, 1fr);
    gap: 4px;
    padding: 12px 8px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.04);
  }

  .band {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .band input[type="range"] {
    writing-mode: vertical-lr;
    direction: rtl;
    height: 110px;
    width: 22px;
    accent-color: var(--p-accent);
  }

  .db,
  .hz {
    font-size: 10px;
    color: var(--p-text-dim);
  }

  .speed-slider {
    width: 100%;
    accent-color: var(--p-accent);
  }
</style>
