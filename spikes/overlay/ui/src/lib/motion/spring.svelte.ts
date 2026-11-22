import { SPRING_PRESETS, Spring1D, type SpringParams, type SpringPresetId } from './spring.ts';

export class SpringGroup<K extends string> {
  private readonly springs: Record<K, Spring1D>;
  private readonly values: Record<K, number>;
  private rafId: number | null = null;
  private lastTs = 0;
  private paused = false;
  private readonly onUpdate: (values: Record<K, number>) => void;

  constructor(
    initial: Record<K, number>,
    preset: SpringPresetId,
    onUpdate: (values: Record<K, number>) => void,
  ) {
    const params = SPRING_PRESETS[preset];
    this.springs = {} as Record<K, Spring1D>;
    this.values = {} as Record<K, number>;
    for (const key of Object.keys(initial) as K[]) {
      this.springs[key] = new Spring1D(initial[key], params);
      this.values[key] = initial[key];
    }
    this.onUpdate = onUpdate;
  }

  setPreset(preset: SpringPresetId, custom?: SpringParams): void {
    const params = custom ?? SPRING_PRESETS[preset];
    for (const key of Object.keys(this.springs) as K[]) {
      this.springs[key].setParams(params);
      this.values[key] = this.springs[key].value;
    }
    this.onUpdate({ ...this.values });
    if (params.instant) {
      this.stopLoop();
    } else {
      this.ensureLoop();
    }
  }

  setTargets(targets: Partial<Record<K, number>>): void {
    let anyMoving = false;
    for (const key of Object.keys(targets) as K[]) {
      const t = targets[key];
      if (t !== undefined) {
        this.springs[key].setTarget(t);
        this.values[key] = this.springs[key].value;
        if (!this.springs[key].isSettled()) anyMoving = true;
      }
    }
    this.onUpdate({ ...this.values });
    if (anyMoving && !this.paused) {
      this.ensureLoop();
    }
  }

  snapAll(targets: Partial<Record<K, number>>): void {
    for (const key of Object.keys(targets) as K[]) {
      const t = targets[key];
      if (t !== undefined) {
        this.springs[key].snap(t);
        this.values[key] = t;
      }
    }
    this.onUpdate({ ...this.values });
    this.stopLoop();
  }

  setPaused(paused: boolean): void {
    this.paused = paused;
    if (paused) {
      this.stopLoop();
    } else {
      this.ensureLoop();
    }
  }

  isActive(): boolean {
    return this.rafId !== null;
  }

  dispose(): void {
    this.stopLoop();
  }

  private ensureLoop(): void {
    if (this.paused || this.rafId !== null) return;
    const anyMoving = (Object.values(this.springs) as Spring1D[]).some((s) => !s.isSettled());
    if (!anyMoving) return;
    this.lastTs = performance.now();
    this.rafId = requestAnimationFrame(this.tick);
  }

  private stopLoop(): void {
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
  }

  private tick = (now: number): void => {
    this.rafId = null;
    if (this.paused) return;
    const dt = (now - this.lastTs) / 1000;
    this.lastTs = now;

    let anyMoving = false;
    for (const key of Object.keys(this.springs) as K[]) {
      if (this.springs[key].step(dt)) anyMoving = true;
      this.values[key] = this.springs[key].value;
    }
    this.onUpdate({ ...this.values });
    if (anyMoving) {
      this.rafId = requestAnimationFrame(this.tick);
    }
  };
}
