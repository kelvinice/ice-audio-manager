import type { MeterFrame } from '../ipc/engine.ts';
import { SPRING_PRESETS, Spring1D, type SpringParams, type SpringPresetId } from '../motion/spring.ts';

export const DEFAULT_BAR_COUNT = 5;
export const MIN_BAR_COUNT = 4;
export const MAX_BAR_COUNT = 6;
export const SILENT_THRESHOLD = 0.001;
export const MIN_BAR_SCALE = 0.18;
export const BAR_SPRING_PRECISION = 0.004;
export const MOCK_METER_INTERVAL_MS = 33;

const PERCEPTUAL_GAMMA = 0.42;
const STEREO_CENTER_PEAK_BLEND = 0.35;
const CONTOUR_BASE = 0.82;
const CONTOUR_ARCH = 0.18;

// Multipliers applied to config SPRING_PRESETS for audio meter dynamics (fast attack, slow decay).
const ATTACK_STIFFNESS_MULT = 4.5;
const ATTACK_DAMPING_MULT = 1.18;
const ATTACK_MASS_MULT = 0.35;
const DECAY_STIFFNESS_MULT = 0.65;
const DECAY_DAMPING_RATIO = 1.04;

export function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(1, value));
}

export function perceptualLevel(linearAmplitude: number): number {
  const clamped = clamp01(linearAmplitude);
  if (clamped < SILENT_THRESHOLD) return 0;
  const normalized = (clamped - SILENT_THRESHOLD) / (1 - SILENT_THRESHOLD);
  return Math.pow(normalized, PERCEPTUAL_GAMMA);
}

export function barScaleY(level: number): number {
  return MIN_BAR_SCALE + (1 - MIN_BAR_SCALE) * clamp01(level);
}

function resolveEffectiveChannels(frame: MeterFrame): number[] {
  const peak = clamp01(frame.peak);
  const raw = frame.channels.map(clamp01);
  if (raw.length === 0 || raw.every((c) => c < SILENT_THRESHOLD)) {
    return peak >= SILENT_THRESHOLD ? [peak] : [];
  }
  // Surround endpoints (5.1/7.1) playing stereo often leave FC/LFE/rear silent; fold to active L/R pair.
  if (raw.length > 2 && raw.slice(2).every((c) => c < SILENT_THRESHOLD)) {
    return [raw[0], raw[1]];
  }
  return raw;
}

export function mapFrameToBars(
  frame: MeterFrame | null | undefined,
  barCount = DEFAULT_BAR_COUNT,
): number[] {
  const count = Math.max(MIN_BAR_COUNT, Math.min(MAX_BAR_COUNT, Math.round(barCount)));
  if (!frame) return new Array<number>(count).fill(0);

  const peak = clamp01(frame.peak);
  const channels = resolveEffectiveChannels(frame);
  if (peak < SILENT_THRESHOLD && channels.length === 0) {
    return new Array<number>(count).fill(0);
  }

  const source = channels.length > 0 ? channels : [peak];
  const centerIndex = (count - 1) / 2;
  const out = new Array<number>(count);

  for (let i = 0; i < count; i += 1) {
    const t = count > 1 ? i / (count - 1) : 0;
    const pos = t * (source.length - 1);
    const lo = Math.floor(pos);
    const hi = Math.min(source.length - 1, lo + 1);
    const frac = pos - lo;
    let level = source[lo] * (1 - frac) + source[hi] * frac;

    if (
      source.length >= 2 &&
      Math.abs(i - centerIndex) <= 0.5 &&
      source[lo] >= SILENT_THRESHOLD &&
      source[hi] >= SILENT_THRESHOLD
    ) {
      level = level * (1 - STEREO_CENTER_PEAK_BLEND) + peak * STEREO_CENTER_PEAK_BLEND;
    }

    const contour = CONTOUR_BASE + CONTOUR_ARCH * Math.sin(((i + 0.5) / count) * Math.PI);
    out[i] = clamp01(perceptualLevel(level) * contour);
  }
  return out;
}

export function deriveVisualizerSpringParams(
  preset: SpringPresetId,
  phase: 'attack' | 'decay',
): SpringParams {
  const base = SPRING_PRESETS[preset];
  if (base.instant) {
    return { ...base, instant: true, precision: BAR_SPRING_PRECISION };
  }
  if (phase === 'attack') {
    return {
      stiffness: base.stiffness * ATTACK_STIFFNESS_MULT,
      damping: base.damping * ATTACK_DAMPING_MULT,
      mass: base.mass * ATTACK_MASS_MULT,
      precision: BAR_SPRING_PRECISION,
    };
  }
  const stiffness = base.stiffness * DECAY_STIFFNESS_MULT;
  const mass = base.mass;
  return {
    stiffness,
    damping: 2 * Math.sqrt(stiffness * mass) * DECAY_DAMPING_RATIO,
    mass,
    precision: BAR_SPRING_PRECISION,
  };
}

export class VisualizerBank {
  private readonly springs: Spring1D[];
  private effectivePreset: SpringPresetId;

  constructor(barCount = DEFAULT_BAR_COUNT, preset: SpringPresetId = 'smooth') {
    const count = Math.max(MIN_BAR_COUNT, Math.min(MAX_BAR_COUNT, Math.round(barCount)));
    this.effectivePreset = preset;
    const initialParams = deriveVisualizerSpringParams(preset, 'decay');
    this.springs = Array.from({ length: count }, () => new Spring1D(0, initialParams));
  }

  setPreset(preset: SpringPresetId, instantOverride = false): void {
    const next: SpringPresetId = instantOverride ? 'off' : preset;
    if (next === this.effectivePreset) return;
    this.effectivePreset = next;
    for (const spring of this.springs) {
      const phase = spring.target > spring.value ? 'attack' : 'decay';
      spring.setParams(deriveVisualizerSpringParams(next, phase));
    }
  }

  setFrame(frame: MeterFrame | null | undefined): boolean {
    const targets = mapFrameToBars(frame, this.springs.length);
    for (let i = 0; i < this.springs.length; i += 1) {
      const spring = this.springs[i];
      const nextTarget = targets[i];
      const changed =
        Math.abs(nextTarget - spring.target) >= BAR_SPRING_PRECISION ||
        (nextTarget === 0 && spring.target !== 0);
      if (!changed) continue;
      const phase = nextTarget > spring.value ? 'attack' : 'decay';
      spring.setParams(deriveVisualizerSpringParams(this.effectivePreset, phase));
      spring.setTarget(nextTarget);
    }
    return !this.isSettled();
  }

  step(dtSec: number): boolean {
    let anyMoving = false;
    for (const spring of this.springs) {
      if (spring.step(dtSec)) {
        anyMoving = true;
      }
    }
    return anyMoving;
  }

  snapSilence(): void {
    for (const spring of this.springs) {
      spring.snap(0);
    }
  }

  isSettled(): boolean {
    return this.springs.every((s) => s.isSettled());
  }

  values(out?: number[]): number[] {
    const dest = out ?? new Array<number>(this.springs.length);
    for (let i = 0; i < this.springs.length; i += 1) {
      dest[i] = clamp01(this.springs[i].value);
    }
    return dest;
  }
}

export function syntheticMeterFrame(tick: number): MeterFrame {
  const x = tick % 65_536;
  const left = 0.45 + 0.35 * Math.sin(x * 0.37) * Math.abs(Math.sin(x * 0.05));
  const right = 0.4 + 0.35 * Math.sin(x * 0.29 + 1.0) * Math.abs(Math.sin(x * 0.07));
  return { peak: Math.max(left, right), channels: [left, right] };
}

export interface VisualizerSubscriptionOptions {
  active: boolean;
  preset: SpringPresetId;
  instant: boolean;
}

export interface VisualizerHandle {
  update: (options: VisualizerSubscriptionOptions) => void;
  dispose: () => void;
}

type LevelListener = (levels: number[]) => void;

class VisualizerController {
  private readonly bank = new VisualizerBank(DEFAULT_BAR_COUNT);
  private readonly levelsBuffer = new Array<number>(DEFAULT_BAR_COUNT).fill(0);
  private readonly listeners = new Set<LevelListener>();
  private activeCount = 0;
  private rafId: number | null = null;
  private lastTs = 0;
  private mockTimerId: number | null = null;
  private mockTick = 0;
  private seenRealFrame = false;
  private backendRequested = false;
  private requestInFlight = false;
  private streamGen = 0;
  private syncQueued = false;
  private listenerReady: Promise<void> | null = null;
  private visibilityBound = false;

  subscribe(onLevels: LevelListener, initial: VisualizerSubscriptionOptions): VisualizerHandle {
    this.bindVisibility();
    this.listeners.add(onLevels);
    let isActive = false;

    const apply = (opts: VisualizerSubscriptionOptions) => {
      this.bank.setPreset(opts.preset, opts.instant);
      if (opts.active !== isActive) {
        isActive = opts.active;
        this.activeCount += isActive ? 1 : -1;
        this.scheduleSync();
      } else if (isActive && opts.instant) {
        this.stopRaf();
        this.emitValues();
      }
    };

    apply(initial);
    onLevels(this.bank.values(this.levelsBuffer));

    return {
      update: apply,
      dispose: () => {
        this.listeners.delete(onLevels);
        if (isActive) {
          isActive = false;
          this.activeCount = Math.max(0, this.activeCount - 1);
          this.scheduleSync();
        }
      },
    };
  }

  private bindVisibility(): void {
    if (this.visibilityBound || typeof document === 'undefined') return;
    this.visibilityBound = true;
    document.addEventListener('visibilitychange', () => {
      this.scheduleSync();
    });
  }

  private isVisibleAndActive(): boolean {
    const docHidden = typeof document !== 'undefined' && document.hidden;
    return this.activeCount > 0 && !docHidden;
  }

  private scheduleSync(): void {
    if (this.syncQueued) return;
    this.syncQueued = true;
    queueMicrotask(() => {
      this.syncQueued = false;
      this.syncNow();
    });
  }

  private syncNow(): void {
    if (!this.isVisibleAndActive()) {
      this.streamGen += 1;
      this.requestInFlight = false;
      this.stopRaf();
      this.stopMock();
      this.bank.snapSilence();
      this.emitValues();
      void this.sendVisualizerHint(false);
      return;
    }

    this.ensureListener();
    if (this.backendRequested || this.requestInFlight) return;
    const gen = ++this.streamGen;
    void this.requestBackendStream(gen);
  }

  private async requestBackendStream(gen: number): Promise<void> {
    if (typeof window === 'undefined') return;
    this.requestInFlight = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_visualizer', { active: true });
      if (gen === this.streamGen) {
        this.requestInFlight = false;
      }
      if (gen !== this.streamGen || !this.isVisibleAndActive()) {
        this.backendRequested = true;
        await this.sendVisualizerHint(false);
        return;
      }
      this.backendRequested = true;
      this.stopMock();
    } catch {
      if (gen === this.streamGen) {
        this.requestInFlight = false;
      }
      // Until phase-3/meter-shell merges (or in Vite browser preview), fall back to bounded 30 Hz fake levels.
      if (gen === this.streamGen && this.isVisibleAndActive() && !this.seenRealFrame) {
        this.startMock();
      }
    }
  }

  private async sendVisualizerHint(active: boolean): Promise<void> {
    if (!this.backendRequested && !active) return;
    if (!active) this.backendRequested = false;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_visualizer', { active });
    } catch {
      // Ignore when shell command is unavailable (pre-merge or standalone browser preview).
    }
  }

  private ensureListener(): void {
    if (this.listenerReady || typeof window === 'undefined') return;
    this.listenerReady = (async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        await listen<MeterFrame>('audio://meter/v1', (ev) => {
          this.seenRealFrame = true;
          this.stopMock();
          if (!this.isVisibleAndActive()) return;
          this.pushFrame(ev.payload);
        });
      } catch {
        this.listenerReady = null;
      }
    })();
  }

  private startMock(): void {
    if (this.mockTimerId !== null || typeof window === 'undefined') return;
    this.pushFrame(syntheticMeterFrame(this.mockTick++));
    this.mockTimerId = window.setInterval(() => {
      if (!this.isVisibleAndActive() || this.seenRealFrame) {
        this.stopMock();
        return;
      }
      this.pushFrame(syntheticMeterFrame(this.mockTick++));
    }, MOCK_METER_INTERVAL_MS);
  }

  private stopMock(): void {
    if (this.mockTimerId !== null && typeof window !== 'undefined') {
      window.clearInterval(this.mockTimerId);
      this.mockTimerId = null;
    }
  }

  private pushFrame(frame: MeterFrame): void {
    const moving = this.bank.setFrame(frame);
    this.emitValues();
    if (moving && this.isVisibleAndActive()) {
      this.ensureRaf();
    } else if (!moving) {
      this.stopRaf();
    }
  }

  private ensureRaf(): void {
    if (this.rafId !== null || typeof requestAnimationFrame === 'undefined') return;
    this.lastTs = performance.now();
    this.rafId = requestAnimationFrame(this.tick);
  }

  private stopRaf(): void {
    if (this.rafId !== null && typeof cancelAnimationFrame !== 'undefined') {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
  }

  private emitValues(): void {
    const snapshot = this.bank.values(this.levelsBuffer);
    for (const listener of this.listeners) {
      listener(snapshot);
    }
  }

  private tick = (now: number): void => {
    this.rafId = null;
    if (!this.isVisibleAndActive()) return;
    const dt = (now - this.lastTs) / 1000;
    this.lastTs = now;
    const moving = this.bank.step(dt);
    this.emitValues();
    if (moving) {
      this.rafId = requestAnimationFrame(this.tick);
    }
  };
}

const sharedController = new VisualizerController();

export function subscribeVisualizer(
  onLevels: LevelListener,
  initial: VisualizerSubscriptionOptions,
): VisualizerHandle {
  return sharedController.subscribe(onLevels, initial);
}
