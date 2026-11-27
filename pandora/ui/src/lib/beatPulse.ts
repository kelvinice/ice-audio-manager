import { audioEngine } from "./audio";
import { BassBreath } from "./breath";

/**
 * Experimental beat pulse: a Svelte action that scales an element up a little on each bass kick and lets it settle
 * (the same kick detection as the Lively backdrop, `lib/breath.ts`). Samples the analyser at most 30 Hz, only while
 * `active` (the setting is on and the song plays); never with reduced motion. Uses the `scale` property, so it
 * combines with an element's own `transform` animation (the spinning cover).
 */
const PULSE_SCALE = 0.045;
const BANDS = new Float32Array(16);

export function beatPulse(node: HTMLElement, active: boolean) {
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const breath = new BassBreath();
  let timer: number | null = null;
  let last = 0;
  let shown = -1;

  function stop() {
    if (timer !== null) window.clearInterval(timer);
    timer = null;
    breath.reset();
    shown = -1;
    node.style.scale = "";
  }

  function tick() {
    const now = performance.now();
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    // 16 bands over ~8 octaves: the first two cover ~55–110 Hz, where the kick drum sits.
    const bass = audioEngine.getSpectrumDb(BANDS.length, BANDS) ? Math.max(BANDS[0], BANDS[1]) : Number.NaN;
    const value = breath.update(bass, dt);
    if (value === shown) return;
    shown = value;
    node.style.scale = value > 0 ? (1 + PULSE_SCALE * value).toFixed(4) : "";
  }

  function start(on: boolean) {
    stop();
    if (!on || reduceMotion) return;
    last = performance.now();
    timer = window.setInterval(tick, 1000 / 30);
  }

  start(active);
  return {
    update: start,
    destroy: stop,
  };
}
