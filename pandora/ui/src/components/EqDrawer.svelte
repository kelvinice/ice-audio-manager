<script lang="ts">
  import { audioEngine, EQ_FREQUENCIES, EQ_PRESETS, playerStore } from "../lib/audio";
  import { parseParametricEqText } from "../lib/parametricEq";
  import type { EqState } from "../lib/types";
  import EqBand from "./EqBand.svelte";
  import Icon from "./Icon.svelte";

  /** Right-hand sheet: the 10-band EQ over its response curve, sound options, speed, sleep timer and AutoEq import. */
  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open, onClose }: Props = $props();

  const SPEEDS = [0.75, 0.9, 1, 1.1, 1.25, 1.5];
  const SLEEP_MINUTES = [15, 30, 60];
  /** Must match `.track` height in EqBand and the space above it (value label + gap). */
  const TRACK_H = 150;
  const TRACK_TOP = 22;
  const GAIN_RANGE = 12;

  let showPaste = $state(false);
  let pasteText = $state("");
  let parseErr = $state<string | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);
  let graphWidth = $state(0);

  let eq = $derived($playerStore.eq);

  function update(patch: Partial<EqState>) {
    audioEngine.updateEq({ ...$playerStore.eq, ...patch });
  }

  function setBand(index: number, db: number) {
    const bands = [...$playerStore.eq.bands];
    bands[index] = db;
    update({ preset: "Custom", bands });
  }

  function applyPreset(name: string) {
    update({ preset: name, bands: [...(EQ_PRESETS[name] ?? EQ_PRESETS.Flat)] });
  }

  function applyParametricText(text: string, name: string) {
    parseErr = null;
    try {
      update({ enabled: true, parametric: parseParametricEqText(text, name) });
      showPaste = false;
      pasteText = "";
    } catch (e) {
      parseErr = e instanceof Error ? e.message : String(e);
    }
  }

  async function handleFilePicked(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    applyParametricText(await file.text(), file.name.replace(/\.txt$/i, ""));
    input.value = "";
  }

  function freqLabel(hz: number): string {
    return hz >= 1000 ? `${hz / 1000}k` : `${hz}`;
  }

  /** Smooth curve through the band handles (Catmull-Rom as cubic Béziers), plus the area down to 0 dB. */
  let curve = $derived.by(() => {
    if (graphWidth <= 0) return { line: "", area: "" };
    const n = eq.bands.length;
    const pts = eq.bands.map((db, i) => [((i + 0.5) / n) * graphWidth, TRACK_H * (0.5 - db / (2 * GAIN_RANGE))]);
    let line = `M ${pts[0][0].toFixed(1)} ${pts[0][1].toFixed(1)}`;
    for (let i = 0; i < pts.length - 1; i++) {
      const [p0, p1, p2, p3] = [pts[i - 1] ?? pts[i], pts[i], pts[i + 1], pts[i + 2] ?? pts[i + 1]];
      const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
      const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
      line += ` C ${c1[0].toFixed(1)} ${c1[1].toFixed(1)}, ${c2[0].toFixed(1)} ${c2[1].toFixed(1)}, ${p2[0].toFixed(1)} ${p2[1].toFixed(1)}`;
    }
    const mid = TRACK_H / 2;
    const area = `${line} L ${pts[n - 1][0].toFixed(1)} ${mid} L ${pts[0][0].toFixed(1)} ${mid} Z`;
    return { line, area };
  });

  let sleepLabel = $derived(
    $playerStore.sleepEndOfTrack
      ? "Stops after this song"
      : $playerStore.sleepRemainingSec !== null
        ? `${Math.ceil($playerStore.sleepRemainingSec / 60)} min left`
        : "Off",
  );
</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && onClose()} />

{#if open}
  <div class="scrim" role="presentation" onclick={onClose}></div>
  <div class="drawer" role="dialog" aria-modal="true" aria-label="Equalizer and sound" tabindex="-1">
    <header class="head">
      <h2>Equalizer</h2>
      <label class="master" title={eq.enabled ? "Turn the equalizer off" : "Turn the equalizer on"}>
        <span>{eq.enabled ? "On" : "Off"}</span>
        <input type="checkbox" checked={eq.enabled} onchange={(e) => update({ enabled: (e.target as HTMLInputElement).checked })} />
      </label>
      <button class="btn-icon" onclick={onClose} aria-label="Close equalizer" title="Close (Esc)">
        <Icon name="x" size={18} />
      </button>
    </header>

    <div class="scroll">
      <section class="card">
        <div class="graph" class:off={!eq.enabled} bind:clientWidth={graphWidth}>
          {#if curve.line}
            <svg class="curve" width={graphWidth} height={TRACK_H} style:top={`${TRACK_TOP}px`} aria-hidden="true">
              <path class="area" d={curve.area} />
              <path class="line" d={curve.line} />
            </svg>
          {/if}
          {#each EQ_FREQUENCIES as hz, idx (hz)}
            <EqBand value={eq.bands[idx]} label={freqLabel(hz)} disabled={!eq.enabled} onChange={(db) => setBand(idx, db)} />
          {/each}
        </div>
        <div class="chips" role="group" aria-label="Presets">
          {#each Object.keys(EQ_PRESETS) as preset (preset)}
            <button class="chip" class:active={eq.preset === preset} onclick={() => applyPreset(preset)}>{preset}</button>
          {/each}
          {#if eq.preset === "Custom"}<span class="chip active static">Custom</span>{/if}
        </div>
      </section>

      <section class="card">
        <h3>Sound</h3>
        <label class="row">
          <span class="text">
            <span class="title">Volume normalization</span>
            <span class="sub">Evens out loud and quiet songs.</span>
          </span>
          <input type="checkbox" checked={eq.normalize} onchange={(e) => update({ normalize: (e.target as HTMLInputElement).checked })} />
        </label>
        <label class="row">
          <span class="text">
            <span class="title">Skip silence</span>
            <span class="sub">Jumps over long quiet gaps.</span>
          </span>
          <input type="checkbox" checked={eq.skipSilence} onchange={(e) => update({ skipSilence: (e.target as HTMLInputElement).checked })} />
        </label>
      </section>

      <section class="card">
        <h3>Playback speed <span class="value">{eq.playbackRate.toFixed(2)}×</span></h3>
        <div class="chips" role="group" aria-label="Playback speed">
          {#each SPEEDS as speed (speed)}
            <button class="chip" class:active={Math.abs(eq.playbackRate - speed) < 0.001} onclick={() => update({ playbackRate: speed })}>
              {speed}×
            </button>
          {/each}
        </div>
      </section>

      <section class="card">
        <h3>Sleep timer <span class="value">{sleepLabel}</span></h3>
        <div class="chips" role="group" aria-label="Sleep timer">
          <button
            class="chip"
            class:active={!$playerStore.sleepEndOfTrack && $playerStore.sleepRemainingSec === null}
            onclick={() => audioEngine.setSleepTimer(null)}>Off</button
          >
          {#each SLEEP_MINUTES as minutes (minutes)}
            <button class="chip" onclick={() => audioEngine.setSleepTimer(minutes)}>{minutes} min</button>
          {/each}
          <button
            class="chip"
            class:active={$playerStore.sleepEndOfTrack}
            onclick={() => audioEngine.setSleepEndOfTrack(!$playerStore.sleepEndOfTrack)}>End of song</button
          >
        </div>
      </section>

      <section class="card">
        <h3>AutoEq profile</h3>
        <p class="sub">Load a ParametricEQ.txt from AutoEq or Equalizer APO to tune for your headphones.</p>
        {#if eq.parametric}
          <div class="profile">
            <span class="text">
              <span class="title">{eq.parametric.name}</span>
              <span class="sub">{eq.parametric.bands.length} filters • preamp {eq.parametric.preamp} dB</span>
            </span>
            <button class="btn-ghost" onclick={() => update({ parametric: null })}>Remove</button>
          </div>
        {/if}
        <div class="chips">
          <input bind:this={fileInput} type="file" accept=".txt" hidden onchange={(e) => void handleFilePicked(e)} />
          <button class="btn-ghost" onclick={() => fileInput?.click()}>
            <Icon name="folder" size={14} />
            <span>Import file</span>
          </button>
          <button class="btn-ghost" onclick={() => (showPaste = !showPaste)}>{showPaste ? "Cancel" : "Paste text"}</button>
        </div>
        {#if showPaste}
          <textarea rows="5" placeholder="Paste the contents of ParametricEQ.txt" bind:value={pasteText}></textarea>
          <button class="btn-primary apply" disabled={!pasteText.trim()} onclick={() => applyParametricText(pasteText, "Pasted AutoEq")}>
            Apply profile
          </button>
        {/if}
        {#if parseErr}<p class="err">{parseErr}</p>{/if}
      </section>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: rgb(0 0 0 / 0.4);
    animation: fade 0.18s ease both;
  }

  .drawer {
    position: fixed;
    z-index: 51;
    top: 8px;
    right: 8px;
    bottom: 8px;
    width: min(420px, calc(100vw - 16px));
    display: flex;
    flex-direction: column;
    border-radius: 14px;
    background: var(--p-surface-solid);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.5);
    overflow: hidden;
    animation: slide-in 0.22s cubic-bezier(0.22, 1, 0.36, 1) both;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 12px 12px 20px;
  }

  h2 {
    flex: 1;
    margin: 0;
    font-size: 20px;
    font-weight: 800;
  }

  .master {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    font-weight: 650;
    color: var(--p-text-muted);
    cursor: pointer;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 0 12px 16px;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    border-radius: 12px;
    background: var(--p-fill);
  }

  h3 {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin: 0;
    font-size: 14px;
    font-weight: 750;
  }

  .value {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--p-text-muted);
    font-variant-numeric: tabular-nums;
  }

  .graph {
    position: relative;
    display: grid;
    grid-template-columns: repeat(10, minmax(0, 1fr));
  }

  .curve {
    position: absolute;
    left: 0;
    pointer-events: none;
    overflow: visible;
  }

  .area {
    fill: color-mix(in srgb, var(--p-accent) 18%, transparent);
  }

  .line {
    fill: none;
    stroke: var(--p-accent);
    stroke-width: 2;
    stroke-linecap: round;
  }

  .graph.off .curve {
    opacity: 0.35;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    height: 30px;
    padding: 0 12px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--p-text) 8%, transparent);
    font-size: 12.5px;
    font-weight: 600;
    color: var(--p-text-muted);
    transition: background 0.14s ease, color 0.14s ease;
  }

  .chip:hover {
    color: var(--p-text);
  }

  .chip.active {
    background: var(--p-text);
    color: var(--p-bg);
  }

  .chip.static {
    display: inline-flex;
    align-items: center;
  }

  .row,
  .profile {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .row {
    cursor: pointer;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    font-size: 13.5px;
    font-weight: 600;
  }

  .sub {
    margin: 0;
    font-size: 12px;
    color: var(--p-text-dim);
  }

  .profile {
    padding: 10px 12px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--p-accent) 14%, transparent);
  }

  .profile .title {
    color: var(--p-accent-bright);
  }

  textarea {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--p-border);
    border-radius: 10px;
    background: var(--p-surface-solid);
    font-family: ui-monospace, Consolas, monospace;
    font-size: 11.5px;
    resize: vertical;
  }

  .apply {
    align-self: flex-start;
  }

  .err {
    margin: 0;
    font-size: 12px;
    color: var(--p-danger);
  }

  @keyframes slide-in {
    from {
      opacity: 0;
      translate: 24px 0;
    }
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }
</style>
