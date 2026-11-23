import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Command, EngineSnapshot, Envelope, MediaAction } from '../ipc/engine';
import type { NotchDelta, NotchSnapshot, SpringPreset, ThemeKind } from '../ipc/types';
import { SpringGroup } from '../motion/spring.svelte';
import {
  computeClipPath,
  computeHitRect,
  deriveBenchOverride,
  effectivePreset,
  resolveRimStroke,
  targetGeometry,
} from './geometry';
import {
  createMediaCommand,
  DEFAULT_IDLE_MODE,
  formatRgbCss,
  formatRgbGlowCss,
  formatSpatialBadge,
  isMediaActive,
  NotchActivityScheduler,
  shortDeviceName,
  type ActivityView,
  type IdleMode,
} from './scheduler';

export class NotchController {
  private readonly scheduler = new NotchActivityScheduler(Date.now());
  private readonly springs: SpringGroup<'w' | 'h' | 'r' | 'vol'>;

  notchSnapshot = $state<NotchSnapshot | null>(null);
  engineState = $state(this.scheduler.state);
  private benchOverride = $state<ActivityView | null>(null);
  private hoverExpanded = $state(false);
  private pointerInside = $state(false);
  pillSize = $state({ w: 116, h: 28, r: 14, vol: 40 });
  frameRate = $state(0);

  private hoverTimer: number | null = null;
  private leaveTimer: number | null = null;
  private activityTimer: number | null = null;
  private lastReportedSeq = 0;
  private lastFrameTs = 0;
  private frameCount = 0;
  private fpsWindowStart = 0;
  private resnapshotting = false;

  readonly idleMode = $derived(this.notchSnapshot?.idleStyle ?? DEFAULT_IDLE_MODE);
  readonly isExpanded = $derived((this.notchSnapshot?.expanded ?? false) || this.hoverExpanded);
  readonly isInstant = $derived(
    (this.notchSnapshot?.systemReduceMotion ?? false) || (this.notchSnapshot?.preset ?? 'smooth') === 'off',
  );
  readonly isSuspended = $derived(this.notchSnapshot?.suspended ?? false);
  readonly isNotchVisible = $derived(!(this.notchSnapshot?.hiddenForBench ?? false) && !this.isSuspended);
  readonly activeView = $derived<ActivityView>(this.benchOverride ?? this.engineState.activity);

  readonly effectiveMicMuted = $derived(this.engineState.micMuted ?? this.notchSnapshot?.micMuted ?? false);
  readonly effectiveVolumePercent = $derived(
    this.activeView.kind === 'volume'
      ? this.activeView.percent
      : (this.engineState.volume?.percent ?? this.notchSnapshot?.volumePercent ?? 0),
  );
  readonly effectiveVolumeMuted = $derived(
    this.activeView.kind === 'volume'
      ? this.activeView.muted
      : (this.engineState.volume?.muted ?? this.notchSnapshot?.muted ?? false),
  );
  readonly effectiveDeviceName = $derived(
    this.engineState.activeDevice?.name ?? (this.notchSnapshot?.activeDevice || 'No playback device'),
  );
  readonly effectiveDeviceShort = $derived(shortDeviceName(this.effectiveDeviceName));
  readonly effectiveSpatialBadge = $derived(
    this.engineState.activeDevice
      ? formatSpatialBadge(this.engineState.activeDevice.spatial)
      : (this.notchSnapshot?.spatialBadge ?? null),
  );
  readonly displayedMedia = $derived(
    this.activeView.kind === 'track' ? this.activeView.media : this.engineState.media,
  );
  readonly activeAccentCss = $derived(
    isMediaActive(this.displayedMedia)
      ? (formatRgbCss(this.displayedMedia.art?.accent) ?? 'var(--ice-accent)')
      : 'var(--ice-accent)',
  );
  readonly activeGlowCss = $derived(
    isMediaActive(this.displayedMedia)
      ? (formatRgbGlowCss(this.displayedMedia.art?.accent, 0.24) ?? 'var(--ice-accent-glow)')
      : 'var(--ice-accent-glow)',
  );
  readonly isIdleHidden = $derived(!this.isExpanded && this.activeView.kind === 'idle' && this.idleMode === 'hidden');
  private readonly clipMetrics = $derived(computeClipPath(this.pillSize));
  readonly insetLeft = $derived(this.clipMetrics.insetLeft);
  readonly clipPath = $derived(this.clipMetrics.clipPath);
  readonly targetBox = $derived(
    targetGeometry(
      this.activeView,
      this.isExpanded,
      this.effectiveMicMuted,
      this.idleMode,
      isMediaActive(this.engineState.media),
      this.engineState.volume?.percent ?? this.notchSnapshot?.volumePercent ?? 40,
    ),
  );
  readonly rimStroke = $derived(resolveRimStroke(this.activeView, this.isExpanded, this.effectiveMicMuted));

  constructor() {
    this.springs = new SpringGroup<'w' | 'h' | 'r' | 'vol'>(
      { w: 116, h: 28, r: 14, vol: 40 },
      'smooth',
      (vals) => {
        this.pillSize = { w: vals.w, h: vals.h, r: vals.r, vol: vals.vol };
        this.recordFrameRate();
      },
    );
  }

  private recordFrameRate(): void {
    const now = performance.now();
    if (this.lastFrameTs > 0) {
      this.frameCount += 1;
      if (now - this.fpsWindowStart >= 350) {
        this.frameRate = Math.round((this.frameCount * 1000) / (now - this.fpsWindowStart));
        this.frameCount = 0;
        this.fpsWindowStart = now;
      }
    } else {
      this.fpsWindowStart = now;
    }
    this.lastFrameTs = now;
  }

  private clearActivityTimer(): void {
    if (this.activityTimer !== null) {
      clearTimeout(this.activityTimer);
      this.activityTimer = null;
    }
  }

  private scheduleNextTick(): void {
    this.clearActivityTimer();
    const snap = this.notchSnapshot;
    if (!snap || snap.hiddenForBench || snap.suspended || this.benchOverride !== null) return;
    const holdOpen = snap.expanded || this.hoverExpanded || this.pointerInside;
    const delay = this.scheduler.nextTickDelayMs(Date.now(), holdOpen);
    if (delay === null) return;

    this.activityTimer = window.setTimeout(() => {
      this.activityTimer = null;
      const nowSnap = this.notchSnapshot;
      if (!nowSnap || nowSnap.hiddenForBench || nowSnap.suspended) return;
      const holding = nowSnap.expanded || this.hoverExpanded || this.pointerInside;
      const outcome = this.scheduler.tick(Date.now(), holding);
      this.engineState = { ...this.scheduler.state };
      if (outcome.dismissedToIdle) this.syncGeometry(false);
      if (outcome.shouldRequestSuspend && !nowSnap.hiddenForBench && !nowSnap.suspended) {
        void invoke('request_idle_suspend');
        return;
      }
      this.scheduleNextTick();
    }, delay);
  }

  private syncGeometry(instantMove: boolean): void {
    const snap = this.notchSnapshot;
    const preset = effectivePreset(snap);
    const paused = (snap?.hiddenForBench ?? false) || (snap?.suspended ?? false);
    this.springs.setPaused(paused);
    this.springs.setPreset(preset);

    const view = this.benchOverride ?? this.engineState.activity;
    const expandedNow = (snap?.expanded ?? false) || this.hoverExpanded;
    const micMutedNow = this.engineState.micMuted ?? snap?.micMuted ?? false;
    const fallbackVol = this.engineState.volume?.percent ?? snap?.volumePercent ?? 40;
    const target = targetGeometry(
      view,
      expandedNow,
      micMutedNow,
      this.idleMode,
      isMediaActive(this.engineState.media),
      fallbackVol,
    );

    if (instantMove || preset === 'off' || paused) {
      this.springs.snapAll(target);
    } else {
      this.springs.setTargets(target);
    }
    void invoke('report_hit_rect', { rect: computeHitRect(target) });
  }

  private acknowledgeRenderedSeq(seq: number): void {
    if (seq <= this.lastReportedSeq) return;
    this.lastReportedSeq = seq;
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        void invoke('report_event_rendered', { eventSeq: seq });
      });
    });
  }

  private applyNotchSnapshot(next: NotchSnapshot, instant = false): void {
    const prevActivity = this.notchSnapshot?.activity;
    const prevSeq = this.notchSnapshot?.eventSeq ?? 0;
    const wasSuspended = this.notchSnapshot?.suspended ?? false;
    this.notchSnapshot = next;
    document.documentElement.dataset.theme = next.theme;

    this.benchOverride = deriveBenchOverride(
      prevActivity,
      prevSeq,
      next,
      this.benchOverride,
      this.engineState.activity.kind === 'idle',
      Date.now(),
    );
    this.syncGeometry(instant || wasSuspended !== next.suspended);
    this.scheduleNextTick();

    if (next.eventSeq > this.lastReportedSeq && !next.suspended) {
      this.acknowledgeRenderedSeq(next.eventSeq);
    }
  }

  private applyEngineSnapshot(snapshot: EngineSnapshot): void {
    if (snapshot.revision < this.scheduler.state.revision) return;
    this.scheduler.applySnapshot(snapshot, Date.now());
    this.engineState = { ...this.scheduler.state };
    this.syncGeometry(true);
    this.scheduleNextTick();
  }

  private async resnapshotEngine(): Promise<void> {
    if (this.resnapshotting) return;
    this.resnapshotting = true;
    try {
      const fresh = await invoke<EngineSnapshot>('get_engine_snapshot');
      this.applyEngineSnapshot(fresh);
    } finally {
      this.resnapshotting = false;
    }
  }

  private applyEngineEnvelope(delta: Envelope): void {
    const res = this.scheduler.applyEnvelope(delta, Date.now());
    if (res.status === 'needs_resnapshot') {
      void this.resnapshotEngine();
      return;
    }
    if (res.status === 'ignored_stale') return;

    this.engineState = { ...this.scheduler.state };
    if (res.wokeActivity || delta.event.type === 'devicesChanged' || delta.event.type === 'mediaChanged') {
      this.benchOverride = null;
      this.syncGeometry(false);
      this.scheduleNextTick();
    }
    const snap = this.notchSnapshot;
    if (res.wokeActivity && snap && !snap.suspended && snap.eventSeq > this.lastReportedSeq) {
      this.acknowledgeRenderedSeq(snap.eventSeq);
    }
  }

  mount(): () => void {
    let disposed = false;
    const bufferedNotch: NotchDelta[] = [];
    const bufferedEngine: Envelope[] = [];
    let notchInitialized = false;
    let engineInitialized = false;
    let bufferedEngineSnapshot: EngineSnapshot | null = null;

    const unlistenNotch = listen<NotchDelta>('notch://delta/v1', (ev) => {
      if (disposed) return;
      if (!notchInitialized) {
        bufferedNotch.push(ev.payload);
        return;
      }
      const next = ev.payload.snapshot;
      if (!this.notchSnapshot || next.revision > this.notchSnapshot.revision) {
        this.applyNotchSnapshot(next);
      }
    });

    const unlistenEngine = listen<Envelope>('engine://delta/v1', (ev) => {
      if (disposed) return;
      if (!engineInitialized) {
        bufferedEngine.push(ev.payload);
        return;
      }
      this.applyEngineEnvelope(ev.payload);
    });

    const unlistenEngineSnap = listen<EngineSnapshot>('engine://snapshot/v1', ({ payload }) => {
      if (disposed) return;
      if (!engineInitialized) {
        bufferedEngineSnapshot = payload;
        return;
      }
      this.applyEngineSnapshot(payload);
    });

    void this.hydrateSnapshots(
      () => disposed,
      [unlistenNotch, unlistenEngine, unlistenEngineSnap],
      () => {
        engineInitialized = true;
        return { bufferedEngineSnapshot, bufferedEngine };
      },
      () => {
        notchInitialized = true;
        return bufferedNotch;
      },
    );

    return () => {
      disposed = true;
      this.springs.dispose();
      this.clearTimers();
      this.clearActivityTimer();
      void unlistenNotch.then((fn) => fn());
      void unlistenEngine.then((fn) => fn());
      void unlistenEngineSnap.then((fn) => fn());
    };
  }

  private async hydrateSnapshots(
    isDisposed: () => boolean,
    listeners: Promise<unknown>[],
    claimEngineBuffer: () => { bufferedEngineSnapshot: EngineSnapshot | null; bufferedEngine: Envelope[] },
    claimNotchBuffer: () => NotchDelta[],
  ): Promise<void> {
    await Promise.all(listeners);
    if (isDisposed()) return;
    const [initialNotch, initialEngine] = await Promise.all([
      invoke<NotchSnapshot>('get_notch_snapshot'),
      invoke<EngineSnapshot>('get_engine_snapshot'),
    ]);
    if (isDisposed()) return;

    const { bufferedEngineSnapshot, bufferedEngine } = claimEngineBuffer();
    const latestEngine =
      bufferedEngineSnapshot && bufferedEngineSnapshot.revision > initialEngine.revision
        ? bufferedEngineSnapshot
        : initialEngine;
    this.applyEngineSnapshot(latestEngine);
    for (const delta of bufferedEngine) {
      if (delta.revision > this.scheduler.state.revision) this.applyEngineEnvelope(delta);
    }

    const bufferedNotch = claimNotchBuffer();
    let latestNotch = initialNotch;
    for (const delta of bufferedNotch) {
      if (delta.snapshot.revision > latestNotch.revision) latestNotch = delta.snapshot;
    }
    this.applyNotchSnapshot(latestNotch, true);

    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        void invoke('report_first_paint');
      });
    });
  }

  private clearTimers(): void {
    if (this.hoverTimer !== null) {
      clearTimeout(this.hoverTimer);
      this.hoverTimer = null;
    }
    if (this.leaveTimer !== null) {
      clearTimeout(this.leaveTimer);
      this.leaveTimer = null;
    }
  }

  onPointerEnter = (): void => {
    this.clearTimers();
    this.pointerInside = true;
    this.scheduler.markInteraction(Date.now());
    this.scheduleNextTick();
    if (this.hoverExpanded || this.notchSnapshot?.expanded) return;
    this.hoverTimer = window.setTimeout(() => {
      this.hoverExpanded = true;
      this.syncGeometry(false);
      this.scheduleNextTick();
    }, 120);
  };

  onPointerLeave = (): void => {
    this.clearTimers();
    this.pointerInside = false;
    this.scheduler.markInteraction(Date.now());
    if (!this.hoverExpanded) {
      this.scheduleNextTick();
      return;
    }
    this.leaveTimer = window.setTimeout(() => {
      this.hoverExpanded = false;
      this.syncGeometry(false);
      this.scheduleNextTick();
    }, 250);
  };

  onWheel = (e: WheelEvent): void => {
    e.preventDefault();
    this.scheduler.markInteraction(Date.now());
    if (e.shiftKey) {
      void invoke('trigger_switch');
    } else {
      const step = e.deltaY < 0 ? 4 : -4;
      void invoke('adjust_volume', { delta: step });
    }
  };

  onAuxClick = (e: MouseEvent): void => {
    if (e.button === 1) {
      e.preventDefault();
      this.scheduler.markInteraction(Date.now());
      void invoke('toggle_mute');
    }
  };

  async sendMediaAction(action: MediaAction): Promise<void> {
    this.scheduler.markInteraction(Date.now());
    this.scheduleNextTick();
    const command: Command = createMediaCommand(action);
    try {
      await invoke('send_command', { command });
      return;
    } catch {
      // Fall back to direct media_command if registered under that name.
    }
    try {
      await invoke('media_command', { action });
    } catch {
      // No-op when running in standalone preview before backend media IPC lands.
    }
  }

  setIdleMode = (mode: IdleMode): void => {
    this.scheduler.markInteraction(Date.now());
    this.syncGeometry(false);
    this.scheduleNextTick();
    void invoke('set_idle_style', { style: mode }).catch(() => {});
  };

  toggleExpand = (): void => void invoke('toggle_expand');
  toggleMute = (): void => void invoke('toggle_mute');
  adjustVolume = (delta: number): void => void invoke('adjust_volume', { delta });
  switchDevice = (deviceId: string): void => void invoke('switch_to_device', { deviceId });
  toggleMicMute = (): void => void invoke('toggle_mic_mute');
  cycleMonitor = (): void => void invoke('cycle_monitor');
  openSettings = (): void => void invoke('open_settings');
  setPreset = (preset: SpringPreset): void => void invoke('set_preset', { preset });
  setTheme = (theme: ThemeKind): void => void invoke('set_theme', { theme });
}

export function createNotchController(): NotchController {
  return new NotchController();
}
