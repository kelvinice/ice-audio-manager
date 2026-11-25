<script lang="ts">
  // Experimental wavy progress line (owner, 2026-11-26): the played part is a gently travelling wave in the song's
  // colour, the rest a flat line; both share one thickness and round caps, so nothing looks clipped and the wave runs
  // straight into the flat line. The crests move under fixed ends, so the path is rebuilt as it moves: a bounded loop,
  // at most 30 Hz, only while playing (or still flattening after a pause) and visible; none with `instant`.
  import { PROGRESS_WAVE, taperedWavePath } from '../../lib/notch/waves';

  interface Props {
    /** Played share, 0–1. */
    progress: number;
    playing: boolean;
    visible: boolean;
    instant: boolean;
    size: 'regular' | 'large';
  }

  let { progress, playing, visible, instant, size }: Props = $props();

  const FRAME_MS = 1000 / 30;
  /** How quickly the wave swells on play / flattens on pause (time constant, s). */
  const SWELL_S = 0.14;

  let width = $state(0);
  let phase = $state(0);
  let ampScale = $state(0);

  const wave = $derived(PROGRESS_WAVE[size]);
  const height = $derived(Math.ceil(wave.amp * 2 + wave.stroke + 2));
  const mid = $derived(height / 2);
  const cap = $derived(wave.stroke / 2);
  const headX = $derived(cap + Math.max(0, Math.min(1, progress)) * Math.max(0, width - wave.stroke));
  const playedD = $derived(
    taperedWavePath(cap, headX, mid, { wavelength: wave.wavelength, amp: wave.amp, phase, ampScale }),
  );

  $effect(() => {
    const target = playing ? 1 : 0;
    if (instant) {
      ampScale = target;
      return;
    }
    if (!visible) return;
    let raf = 0;
    let last = performance.now();
    let lastPaint = 0;
    const tick = (now: number): void => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      if (playing) phase = (phase + wave.speed * dt) % wave.wavelength;
      const next = ampScale + (target - ampScale) * (1 - Math.exp(-dt / SWELL_S));
      const settled = !playing && Math.abs(target - next) < 0.01;
      if (settled) {
        ampScale = target;
        raf = 0;
        return;
      }
      if (now - lastPaint >= FRAME_MS) {
        lastPaint = now;
        ampScale = next;
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });
</script>

<div class="wavy" bind:clientWidth={width} style:height={`${height}px`}>
  {#if width > 0}
    <svg {width} {height} viewBox={`0 0 ${width} ${height}`} aria-hidden="true">
      {#if headX < width - cap}
        <line
          class="rest"
          x1={headX}
          y1={mid}
          x2={width - cap}
          y2={mid}
          stroke-width={wave.stroke}
          stroke-linecap="round"
        />
      {/if}
      <path class="played" d={playedD} fill="none" stroke-width={wave.stroke} stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  {/if}
</div>

<style>
  .wavy {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
  }

  svg {
    display: block;
    overflow: visible;
  }

  .rest {
    stroke: var(--ice-track);
  }

  .played {
    stroke: var(--ice-active-accent, var(--ice-accent));
  }
</style>
