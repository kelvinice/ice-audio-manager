import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { EngineSnapshot, Envelope, MediaAction } from '../ipc/engine';
import type { NotchDelta, NotchSnapshot } from '../ipc/types';
import { SpringGroup } from '../motion/spring.svelte';
import {
  computeArtBackdrop,
  resolveAmbientMotionState,
  resolveMusicServices,
  type MusicServiceId,
} from './backdrop';
import {
  computeClipPath,
  computeHitRect,
  deriveBenchOverride,
  effectivePreset,
  resolveRimStroke,
  targetGeometry,
} from './geometry';
import {
  DEFAULT_IDLE_MODE,
  formatRgbCss,
  formatRgbGlowCss,
  formatSpatialBadge,
  isMediaActive,
  NotchActivityScheduler,
  nowPlayingMode,
  shortDeviceName,
  type ActivityView,
} from './scheduler';
import { resolveDisplayedView } from './nowPlaying';

export class NotchController {
  private readonly scheduler = new NotchActivityScheduler(Date.now());
  private readonly springs: SpringGroup<'w' | 'h' | 'r' | 'vol'>;

  notchSnapshot = $state<NotchSnapshot | null>(null);
  engineState = $state(this.scheduler.state);
  private benchOverride = $state<ActivityView | null>(null);
  private hoverExpanded = $state(false);
  private pointerInside = $state(false);
  pillSize = $state({ w: 116, h: 28, r: 14, vol: 40 });

  private hoverTimer: number | null = null;
  private leaveTimer: number | null = null;
  private activityTimer: number | null = null;
  private lastReportedSeq = 0;
  private resnapshotting = false;

  readonly idleMode = $derived(this.notchSnapshot?.idleStyle ?? DEFAULT_IDLE_MODE);
  readonly isExpanded = $derived((this.notchSnapshot?.expanded ?? false) || this.hoverExpanded);
  readonly isInstant = $derived(
    (this.notchSnapshot?.systemReduceMotion ?? false) || (this.notchSnapshot?.preset ?? 'smooth') === 'off',
  );
  readonly isSuspended = $derived(this.notchSnapshot?.suspended ?? false);
  /** Twin: `Placement::suspends_on_idle` in app/src-tauri/src/placement.rs. */
  private get suspendsOnIdle(): boolean {
    return this.engineState.settings?.performanceProfile === 'balanced';
  }
  readonly isNotchVisible = $derived(!(this.notchSnapshot?.hiddenForBench ?? false) && !this.isSuspended);
  readonly activeView = $derived<ActivityView>(
    this.benchOverride ??
      resolveDisplayedView(
        this.engineState.activity,
        this.engineState.media,
        nowPlayingMode(this.engineState.settings),
        this.scheduler.lastTrackSeq,
      ),
  );

  /** The engine's activity as shown, i.e. with the `appearance.nowPlaying` setting applied (non-reactive read). */
  private displayedEngineView(): ActivityView {
    return resolveDisplayedView(
      this.engineState.activity,
      this.engineState.media,
      nowPlayingMode(this.engineState.settings),
      this.scheduler.lastTrackSeq,
    );
  }

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
  readonly hasBackdropArt = $derived(
    isMediaActive(this.displayedMedia)
      && Boolean(this.displayedMedia.art)
      && (this.isExpanded || this.activeView.kind === 'track'),
  );
  readonly artBackdrop = $derived(
    computeArtBackdrop(this.hasBackdropArt ? this.displayedMedia?.art : null),
  );
  readonly movingBackdropEnabled = $derived(
    this.engineState.settings?.appearance.movingBackdrop ?? true,
  );
  readonly ambientMotionState = $derived(
    resolveAmbientMotionState({
      hasArt: this.hasBackdropArt,
      isExpanded: this.isExpanded,
      isVisible: this.isNotchVisible,
      isInstant: this.isInstant,
      isPlaying: this.displayedMedia?.status === 'playing',
      movingBackdrop: this.movingBackdropEnabled,
    }),
  );
  readonly musicServices = $derived(
    resolveMusicServices(this.engineState.settings, this.engineState.media),
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
      },
    );
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
        // Instant keeps the WebView notch awake; only Balanced hands over to the native pill (shell checks too).
        // A "while playing" now-playing pill isn't idle either; pausing re-arms the linger (`applyEngineEnvelope`).
        if (this.suspendsOnIdle && this.displayedEngineView().kind === 'idle') void invoke('request_idle_suspend');
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

    const view = this.benchOverride ?? this.displayedEngineView();
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
    const prevProfile = this.engineState.settings?.performanceProfile;
    this.scheduler.applySnapshot(snapshot, Date.now());
    this.engineState = { ...this.scheduler.state };
    this.rearmLingerOnProfileChange(prevProfile);
    this.syncGeometry(true);
    this.scheduleNextTick();
  }

  /** A profile switch (e.g. Instant → Balanced) restarts the idle linger so the new policy applies without an event. */
  private rearmLingerOnProfileChange(prevProfile: string | undefined): void {
    const next = this.engineState.settings?.performanceProfile;
    if (prevProfile === undefined || next === undefined || prevProfile === next) return;
    this.scheduler.markInteraction(Date.now());
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

    const prevProfile = this.engineState.settings?.performanceProfile;
    this.engineState = { ...this.scheduler.state };
    if (prevProfile !== this.engineState.settings?.performanceProfile) {
      this.rearmLingerOnProfileChange(prevProfile);
      this.scheduleNextTick();
    }
    const type = delta.event.type;
    if (type === 'mediaChanged' && nowPlayingMode(this.engineState.settings) === 'while_playing') {
      // Play/pause flips the "while playing" pill; restart the idle linger so Balanced suspends only once it's idle.
      this.scheduler.markInteraction(Date.now());
    }
    // `settingsChanged`: the now-playing mode (or idle style) can change what the idle notch shows.
    if (res.wokeActivity || type === 'devicesChanged' || type === 'mediaChanged' || type === 'settingsChanged') {
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

  sendMediaAction(action: MediaAction): void {
    this.scheduler.markInteraction(Date.now());
    this.scheduleNextTick();
    void invoke('media_command', { action }).catch(() => {});
  }

  toggleExpand = (): void => void invoke('toggle_expand');
  toggleMute = (): void => void invoke('toggle_mute');
  setVolume = (percent: number): void => {
    this.scheduler.markInteraction(Date.now());
    void invoke('set_volume', { percent: Math.max(0, Math.min(100, Math.round(percent))) });
  };
  switchDevice = (deviceId: string): void => void invoke('switch_to_device', { deviceId });
  toggleMicMute = (): void => void invoke('toggle_mic_mute');
  openSettings = (): void => void invoke('open_settings');
  openMusicService = (service: MusicServiceId): void => {
    this.scheduler.markInteraction(Date.now());
    this.scheduleNextTick();
    void invoke('open_music_service', { service }).catch(() => {});
  };
}

export function createNotchController(): NotchController {
  return new NotchController();
}
