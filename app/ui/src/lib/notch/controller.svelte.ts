import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { EngineSnapshot, Envelope, MediaAction, NotchAnchor, NotchStyle } from '../ipc/engine';
import type { HitRect, NotchDelta, NotchSnapshot } from '../ipc/types';
import { SpringGroup } from '../motion/spring.svelte';
import {
  adaptAccentForSurface,
  computeArtBackdrop,
  resolveAmbientMotionState,
  resolveMusicServices,
  type MusicServiceId,
} from './backdrop';
import { isDragBlockedTarget, shouldStartDrag } from './drag';
import {
  CARD_LYRICS_H,
  deriveBenchOverride,
  effectivePreset,
  isBottomAnchor as isBottomAnchorValue,
  PILL_LYRICS_H,
  PILL_LYRICS_SUB_H,
  resolveRimStroke,
} from './geometry';
import { getNotchStyleStrategy, NO_HIT_RECT, plainGeometryContext, type GeometryContext } from './notchStyle';
import { hasSubtitleLines, shouldShowCardLyrics, shouldShowPillLyrics } from './lyrics';
import {
  DEFAULT_IDLE_MODE,
  formatRgbCss,
  formatRgbGlowCss,
  formatSpatialBadge,
  isMediaActive,
  NotchActivityScheduler,
  nowPlayingMode,
  shortDeviceName,
  toMsNumber,
  type ActivityView,
} from './scheduler';
import {
  applyOptimisticSeek,
  clampSeekPositionMs,
  mediaTrackKey,
  OPTIMISTIC_SEEK_HOLD_MS,
  shouldClearOptimisticSeek,
  type OptimisticSeek,
} from './scrubber';
import { resolveDisplayedView } from './nowPlaying';

/** D-27 hibernate shows only the idle shape. */
const IDLE_VIEW: ActivityView = Object.freeze({ kind: 'idle' });
/**
 * D-27: hibernating in Balanced hands over to the native pill (same plain notch, or nothing) once the plain shape has
 * settled, without the idle linger: nothing can pop up, and music playing would otherwise keep re-arming the linger.
 */
const HIBERNATE_HANDOVER_MS = 1000;
/** After asking the shell to drag, how long the notch stays lifted waiting for `NotchSnapshot.dragging`. */
const HELD_DRAG_START_GRACE_MS = 600;

export class NotchController {
  private readonly scheduler = new NotchActivityScheduler(Date.now());
  private readonly springs: SpringGroup<'w' | 'h' | 'r' | 'vol'>;

  notchSnapshot = $state<NotchSnapshot | null>(null);
  engineState = $state(this.scheduler.state);
  private benchOverride = $state<ActivityView | null>(null);
  private hoverExpanded = $state(false);
  private pointerInside = $state(false);
  lyricsCardOpen = $state(true);
  pillSize = $state({ w: 116, h: 30, r: 13, vol: 40 });
  /** 0 = resting on its edge, 1 = held / carried (full capsule, lifted). Spring-driven; may overshoot slightly. */
  lift = $state(0);
  /** The pointer is pressed on the notch (a possible drag); cleared on release or when the shell's drag ends. */
  private held = $state(false);
  private heldFallbackTimer: number | null = null;
  private readonly liftSpring: SpringGroup<'lift'>;
  placedFlash = $state(false);
  placedFlashKey = $state(0);

  private hoverTimer: number | null = null;
  private leaveTimer: number | null = null;
  private activityTimer: number | null = null;
  private placedFlashTimer: number | null = null;
  private optimisticSeek = $state<OptimisticSeek | null>(null);
  private optimisticSeekTimer: number | null = null;
  private dragCleanup: (() => void) | null = null;
  private lastHitRect: HitRect | null = null;
  private lastReportedSeq = 0;
  private lastPlacedSeq: number | null = null;
  private resnapshotting = false;

  readonly idleMode = $derived(this.notchSnapshot?.idleStyle ?? DEFAULT_IDLE_MODE);
  readonly notchStyle = $derived<NotchStyle>(
    this.notchSnapshot?.notchStyle ?? this.engineState.settings?.appearance.notchStyle ?? 'nook',
  );
  readonly styleStrategy = $derived(getNotchStyleStrategy(this.notchStyle));
  readonly effectiveAnchor = $derived<NotchAnchor>(
    this.notchSnapshot?.anchor ?? this.engineState.settings?.notchAnchor ?? 'top_center',
  );
  readonly isBottomAnchor = $derived(isBottomAnchorValue(this.effectiveAnchor));
  /** D-27 hibernate: a plain, click-through silhouette; no activities, no expansion, no media. */
  readonly isHibernating = $derived(this.engineState.settings?.appearance.hibernate ?? false);
  /** How much later lyrics show for the playing output (Bluetooth delay; `DeviceSummary.lyricsDelayMs`). */
  readonly lyricsDelayMs = $derived(this.engineState.activeDevice?.lyricsDelayMs ?? 0);
  readonly isExpanded = $derived(
    !this.isHibernating && ((this.notchSnapshot?.expanded ?? false) || this.hoverExpanded),
  );
  readonly isInstant = $derived(
    (this.notchSnapshot?.systemReduceMotion ?? false) || (this.notchSnapshot?.preset ?? 'bouncy') === 'off',
  );
  readonly isSuspended = $derived(this.notchSnapshot?.suspended ?? false);
  /** Twin: `Placement::suspends_on_idle` in app/src-tauri/src/placement.rs. */
  private get suspendsOnIdle(): boolean {
    return this.engineState.settings?.performanceProfile === 'balanced';
  }
  readonly isNotchVisible = $derived(!(this.notchSnapshot?.hiddenForBench ?? false) && !this.isSuspended);
  readonly activeView = $derived<ActivityView>(
    this.isHibernating
      ? IDLE_VIEW
      : (this.benchOverride ??
      resolveDisplayedView(
        this.engineState.activity,
        this.engineState.media,
        nowPlayingMode(this.engineState.settings),
        this.scheduler.lastTrackSeq,
      )),
  );

  /** The engine's activity as shown, i.e. with the `appearance.nowPlaying` setting applied (non-reactive read). */
  private displayedEngineView(): ActivityView {
    if (this.engineState.settings?.appearance.hibernate) return IDLE_VIEW;
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
  readonly effectiveMedia = $derived(
    applyOptimisticSeek(this.engineState.media, this.optimisticSeek, Date.now()),
  );
  readonly displayedMedia = $derived(
    this.activeView.kind === 'track' ? this.activeView.media : this.effectiveMedia,
  );
  readonly isLightTheme = $derived(
    (this.notchSnapshot?.theme ?? this.engineState.settings?.appearance.theme ?? 'ice') === 'ice_light',
  );
  readonly activeAccentCss = $derived(
    isMediaActive(this.displayedMedia) && this.displayedMedia.art?.accent
      ? (formatRgbCss(adaptAccentForSurface(this.displayedMedia.art.accent, this.isLightTheme)) ?? 'var(--ice-accent)')
      : 'var(--ice-accent)',
  );
  readonly activeGlowCss = $derived(
    isMediaActive(this.displayedMedia) && this.displayedMedia.art?.accent
      ? (formatRgbGlowCss(
          adaptAccentForSurface(this.displayedMedia.art.accent, this.isLightTheme),
          this.isLightTheme ? 0.22 : 0.24,
        ) ?? 'var(--ice-accent-glow)')
      : 'var(--ice-accent-glow)',
  );
  readonly hasBackdropArt = $derived(
    isMediaActive(this.displayedMedia)
      && Boolean(this.displayedMedia.art)
      && (this.isExpanded || this.activeView.kind === 'track'),
  );
  readonly artBackdrop = $derived(
    computeArtBackdrop(this.hasBackdropArt ? this.displayedMedia?.art : null, this.isLightTheme),
  );
  readonly movingBackdropEnabled = $derived(
    this.engineState.settings?.appearance.movingBackdrop ?? true,
  );
  readonly ambientMotionState = $derived(
    resolveAmbientMotionState({
      hasArt: this.hasBackdropArt,
      isExpanded: this.isExpanded,
      isTrackPill: this.activeView.kind === 'track',
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
  /**
   * Owner, 2026-11-26: holding or carrying the notch shows it as a full capsule lifted off its edge (Edge Nook loses
   * its ears and flush corners while held). A press on the open card doesn't lift it; a drag does (the card collapses).
   */
  readonly isLifted = $derived(
    !this.isHibernating && ((this.held && !this.isExpanded) || (this.notchSnapshot?.dragging ?? false)),
  );
  readonly dragEffects = $derived(
    this.engineState.settings?.appearance.dragEffects ?? { landingRipple: false, jellyLanding: false, carryGlow: false },
  );
  readonly surfaceShape = $derived(
    this.styleStrategy.computeSurfaceShape(this.pillSize, this.isBottomAnchor, this.lift),
  );
  readonly insetLeft = $derived(this.surfaceShape.insetLeft);
  readonly clipPath = $derived(this.surfaceShape.clipPath);
  readonly showPillLyrics = $derived(
    !this.isExpanded &&
      this.activeView.kind === 'track' &&
      shouldShowPillLyrics(
        this.engineState.settings?.lyrics,
        this.engineState.lyrics,
        isMediaActive(this.activeView.media),
      ),
  );
  readonly showCardLyrics = $derived(
    this.isExpanded &&
      shouldShowCardLyrics(
        this.engineState.settings?.lyrics,
        this.engineState.lyrics,
        isMediaActive(this.engineState.media),
        this.lyricsCardOpen,
      ),
  );
  private computePillLyricsExtraH(view: ActivityView): number {
    const cfg = this.engineState.settings?.lyrics;
    if (view.kind === 'track' && shouldShowPillLyrics(cfg, this.engineState.lyrics, isMediaActive(view.media))) {
      const hasSub = hasSubtitleLines(this.engineState.lyrics.document, cfg?.romaji ?? true);
      return hasSub ? PILL_LYRICS_SUB_H : PILL_LYRICS_H;
    }
    return 0;
  }
  private computeCardLyricsExtraH(hasMedia: boolean): number {
    const cfg = this.engineState.settings?.lyrics;
    return shouldShowCardLyrics(cfg, this.engineState.lyrics, hasMedia, this.lyricsCardOpen) ? CARD_LYRICS_H : 0;
  }
  readonly targetBox = $derived.by(() => {
    const hasMedia = isMediaActive(this.engineState.media);
    const ctx: GeometryContext = {
      view: this.activeView,
      expanded: this.isExpanded,
      micMuted: this.effectiveMicMuted,
      idleMode: this.idleMode,
      hasMedia,
      fallbackVolume: this.engineState.volume?.percent ?? this.notchSnapshot?.volumePercent ?? 40,
      pillLyricsExtraH: this.computePillLyricsExtraH(this.activeView),
      cardLyricsExtraH: this.computeCardLyricsExtraH(hasMedia),
      showCardLyrics: this.showCardLyrics,
    };
    return this.styleStrategy.targetGeometry(this.isHibernating ? plainGeometryContext(ctx) : ctx);
  });
  readonly rimStroke = $derived(resolveRimStroke(this.activeView, this.isExpanded, this.effectiveMicMuted));

  constructor() {
    this.springs = new SpringGroup<'w' | 'h' | 'r' | 'vol'>(
      { w: 116, h: 28, r: 14, vol: 40 },
      'bouncy',
      (vals) => {
        this.pillSize = { w: vals.w, h: vals.h, r: vals.r, vol: vals.vol };
      },
    );
    // Snappy on purpose: the lift should answer the press at once, whatever the morph preset is.
    this.liftSpring = new SpringGroup<'lift'>({ lift: 0 }, 'snappy', (vals) => {
      this.lift = vals.lift;
    });
  }

  /** Moves the lift spring towards the current held / carried state (instant with reduce motion or preset off). */
  private syncLift(): void {
    const target = this.isLifted ? 1 : 0;
    const snap = this.notchSnapshot;
    if (this.isInstant || (snap?.suspended ?? false) || (snap?.hiddenForBench ?? false)) {
      this.liftSpring.snapAll({ lift: target });
    } else {
      this.liftSpring.setTargets({ lift: target });
    }
  }

  private setHeld(held: boolean): void {
    if (this.heldFallbackTimer !== null) {
      clearTimeout(this.heldFallbackTimer);
      this.heldFallbackTimer = null;
    }
    this.held = held;
    this.syncLift();
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
    if (this.engineState.settings?.appearance.hibernate) {
      if (this.suspendsOnIdle) this.scheduleHibernateHandover();
      return;
    }
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

  /** One-shot (stops on wake, suspend or leaving hibernate): the shell re-checks the profile and the window state. */
  private scheduleHibernateHandover(): void {
    this.activityTimer = window.setTimeout(() => {
      this.activityTimer = null;
      const snap = this.notchSnapshot;
      if (!snap || snap.suspended || !this.engineState.settings?.appearance.hibernate || !this.suspendsOnIdle) return;
      void invoke('request_idle_suspend');
    }, HIBERNATE_HANDOVER_MS);
  }

  private syncGeometry(instantMove: boolean): void {
    const snap = this.notchSnapshot;
    const preset = effectivePreset(snap);
    const paused = (snap?.hiddenForBench ?? false) || (snap?.suspended ?? false);
    this.springs.setPaused(paused);
    this.springs.setPreset(preset);

    const hibernating = this.engineState.settings?.appearance.hibernate ?? false;
    if (hibernating) {
      // The plain notch is click-through, so a pointer that was over it never reports leaving: forget the hover.
      this.clearTimers();
      this.hoverExpanded = false;
      this.pointerInside = false;
    }
    const view = hibernating ? IDLE_VIEW : (this.benchOverride ?? this.displayedEngineView());
    const expandedNow = !hibernating && ((snap?.expanded ?? false) || this.hoverExpanded);
    const micMutedNow = this.engineState.micMuted ?? snap?.micMuted ?? false;
    const fallbackVol = this.engineState.volume?.percent ?? snap?.volumePercent ?? 40;
    const hasMedia = isMediaActive(this.engineState.media);
    const strategy = getNotchStyleStrategy(snap?.notchStyle ?? this.engineState.settings?.appearance.notchStyle);
    const showLyrics = expandedNow
      ? shouldShowCardLyrics(this.engineState.settings?.lyrics, this.engineState.lyrics, hasMedia, this.lyricsCardOpen)
      : false;
    const ctx: GeometryContext = {
      view,
      expanded: expandedNow,
      micMuted: micMutedNow,
      idleMode: this.idleMode,
      hasMedia,
      fallbackVolume: fallbackVol,
      pillLyricsExtraH: this.computePillLyricsExtraH(view),
      cardLyricsExtraH: this.computeCardLyricsExtraH(hasMedia),
      showCardLyrics: showLyrics,
    };
    const target = strategy.targetGeometry(hibernating ? plainGeometryContext(ctx) : ctx);

    if (instantMove || preset === 'off' || paused) {
      this.springs.snapAll(target);
    } else {
      this.springs.setTargets(target);
    }
    const style = snap?.notchStyle ?? this.engineState.settings?.appearance.notchStyle;
    const anchor = snap?.anchor ?? this.engineState.settings?.notchAnchor;
    const isBottom = isBottomAnchorValue(anchor);
    const nextRect = hibernating ? NO_HIT_RECT : strategy.computeHitRect(target, isBottom);
    if (
      !this.lastHitRect ||
      this.lastHitRect.x !== nextRect.x ||
      this.lastHitRect.y !== nextRect.y ||
      this.lastHitRect.width !== nextRect.width ||
      this.lastHitRect.height !== nextRect.height
    ) {
      this.lastHitRect = nextRect;
      // A lost report would leave the shell with an old (e.g. expanded) rect that swallows clicks around the pill,
      // and the dedup above would never resend it: forget it on failure so the next sync retries.
      invoke('report_hit_rect', { rect: nextRect }).catch(() => {
        if (this.lastHitRect === nextRect) this.lastHitRect = null;
      });
    }
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
    const prevAnchor = this.notchSnapshot?.anchor;
    const wasSuspended = this.notchSnapshot?.suspended ?? false;
    const wasDragging = this.notchSnapshot?.dragging ?? false;
    this.notchSnapshot = next;
    // The shell's drag ended (button released or Esc): let go, so the notch settles back onto its edge.
    if (wasDragging && !next.dragging) this.held = false;
    if (next.dragging && this.heldFallbackTimer !== null) {
      clearTimeout(this.heldFallbackTimer);
      this.heldFallbackTimer = null;
    }
    this.syncLift();
    // Waking from TrySuspend or switching anchor: re-send the hit rect even if unchanged.
    if ((wasSuspended && !next.suspended) || (prevAnchor !== undefined && prevAnchor !== next.anchor)) {
      this.lastHitRect = null;
    }
    document.documentElement.dataset.theme = next.theme;

    this.benchOverride = deriveBenchOverride(
      prevActivity,
      prevSeq,
      next,
      this.benchOverride,
      this.engineState.activity.kind === 'idle',
      Date.now(),
    );
    const flippedEdge =
      prevAnchor !== undefined && isBottomAnchorValue(prevAnchor) !== isBottomAnchorValue(next.anchor);
    this.syncGeometry(instant || wasSuspended !== next.suspended || flippedEdge);
    this.scheduleNextTick();

    if (this.lastPlacedSeq === null) {
      this.lastPlacedSeq = next.placedSeq;
    } else if (next.placedSeq !== this.lastPlacedSeq) {
      this.lastPlacedSeq = next.placedSeq;
      if (!this.isInstant && !this.isHibernating && !next.suspended) {
        this.triggerPlacedFlash();
      }
    }

    if (next.eventSeq > this.lastReportedSeq && !next.suspended) {
      this.acknowledgeRenderedSeq(next.eventSeq);
    }
  }

  private triggerPlacedFlash(): void {
    if (this.placedFlashTimer !== null) {
      clearTimeout(this.placedFlashTimer);
      this.placedFlashTimer = null;
    }
    this.placedFlashKey += 1;
    this.placedFlash = true;
    // Long enough for the longest landing (`notch-place-jelly` / `notch-ripple`, 640 ms in NotchApp.svelte).
    const fx = this.dragEffects;
    this.placedFlashTimer = window.setTimeout(
      () => {
        this.placedFlashTimer = null;
        this.placedFlash = false;
      },
      fx.jellyLanding || fx.landingRipple ? 680 : 460,
    );
  }

  private applyEngineSnapshot(snapshot: EngineSnapshot): void {
    if (snapshot.revision < this.scheduler.state.revision) return;
    const prevProfile = this.engineState.settings?.performanceProfile;
    this.scheduler.applySnapshot(snapshot, Date.now());
    this.engineState = { ...this.scheduler.state };
    if (this.optimisticSeek && shouldClearOptimisticSeek(this.optimisticSeek, this.engineState.media, Date.now())) {
      this.clearOptimisticSeek();
    }
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
    const prevMedia = this.engineState.media;
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
    if (type === 'mediaChanged' && this.optimisticSeek) {
      const next = this.engineState.media;
      const trackChanged = !next || mediaTrackKey(next) !== this.optimisticSeek.trackKey;
      const timelineMoved =
        toMsNumber(next?.positionMs) !== toMsNumber(prevMedia?.positionMs) ||
        toMsNumber(next?.positionUpdatedMs) !== toMsNumber(prevMedia?.positionUpdatedMs) ||
        next?.status !== prevMedia?.status;
      if ((trackChanged || timelineMoved) && shouldClearOptimisticSeek(this.optimisticSeek, next, Date.now())) {
        this.clearOptimisticSeek();
      }
    }
    if (type === 'mediaChanged' && nowPlayingMode(this.engineState.settings) === 'while_playing') {
      // Play/pause flips the "while playing" pill; restart the idle linger so Balanced suspends only once it's idle.
      this.scheduler.markInteraction(Date.now());
    }
    // `settingsChanged` / `lyricsChanged`: idle mode or bottom lyrics extension can change the capsule height.
    if (
      res.wokeActivity ||
      type === 'devicesChanged' ||
      type === 'mediaChanged' ||
      type === 'lyricsChanged' ||
      type === 'settingsChanged'
    ) {
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
      this.dragCleanup?.();
      this.springs.dispose();
      this.liftSpring.dispose();
      if (this.heldFallbackTimer !== null) clearTimeout(this.heldFallbackTimer);
      this.clearTimers();
      this.clearActivityTimer();
      this.clearOptimisticSeek();
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
    if (this.placedFlashTimer !== null) {
      clearTimeout(this.placedFlashTimer);
      this.placedFlashTimer = null;
    }
  }

  /** D-27: the notch's moon button. The shell saves the setting and unpins the card; waking is in the tray/Settings. */
  hibernate = (): void => {
    void invoke('set_hibernate', { enabled: true });
  };

  onPointerDown = (e: PointerEvent): void => {
    if (e.button !== 0 || this.isHibernating) return;
    if (this.engineState.settings?.notchPlacement === 'fixed') return;
    if (isDragBlockedTarget(e.target)) return;

    this.dragCleanup?.();
    const startX = e.clientX;
    const startY = e.clientY;
    const pointerId = e.pointerId;

    const cleanup = () => {
      window.removeEventListener('pointermove', onMove, true);
      window.removeEventListener('pointerup', onEnd, true);
      window.removeEventListener('pointercancel', onEnd, true);
      window.removeEventListener('blur', onEnd);
      if (this.dragCleanup === cleanup) this.dragCleanup = null;
    };

    const onMove = (ev: PointerEvent) => {
      if (ev.pointerId !== pointerId) return;
      if (!shouldStartDrag(startX, startY, ev.clientX, ev.clientY)) return;
      cleanup();
      this.clearTimers();
      this.hoverExpanded = false;
      this.pointerInside = false;
      this.syncGeometry(false);
      // Stay lifted until the shell's drag ends (`NotchSnapshot.dragging`); if it never starts, put the notch down.
      this.heldFallbackTimer = window.setTimeout(() => {
        this.heldFallbackTimer = null;
        if (!(this.notchSnapshot?.dragging ?? false)) this.setHeld(false);
      }, HELD_DRAG_START_GRACE_MS);
      void invoke('start_notch_drag').catch(() => this.setHeld(false));
    };

    const onEnd = (ev?: Event) => {
      if (ev instanceof PointerEvent && ev.pointerId !== pointerId) return;
      cleanup();
      this.setHeld(false);
    };

    this.setHeld(true);
    this.dragCleanup = cleanup;
    window.addEventListener('pointermove', onMove, true);
    window.addEventListener('pointerup', onEnd, true);
    window.addEventListener('pointercancel', onEnd, true);
    window.addEventListener('blur', onEnd);
  };

  onPointerEnter = (e?: PointerEvent): void => {
    if (this.isHibernating) return;
    if (e && e.buttons !== 0) return;
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
    if (this.isHibernating) return;
    this.scheduler.markInteraction(Date.now());
    if (e.shiftKey) {
      void invoke('trigger_switch');
    } else {
      // Twin: `WHEEL_VOLUME_STEP` in app/src-tauri/src/show_notch.rs (the native pill's wheel).
      const step = e.deltaY < 0 ? 4 : -4;
      void invoke('adjust_volume', { delta: step });
    }
  };

  onAuxClick = (e: MouseEvent): void => {
    if (e.button === 1 && !this.isHibernating) {
      e.preventDefault();
      this.scheduler.markInteraction(Date.now());
      void invoke('toggle_mute');
    }
  };

  private clearOptimisticSeek(): void {
    this.optimisticSeek = null;
    if (this.optimisticSeekTimer !== null) {
      clearTimeout(this.optimisticSeekTimer);
      this.optimisticSeekTimer = null;
    }
  }

  sendMediaAction(action: MediaAction): void {
    this.scheduler.markInteraction(Date.now());
    this.scheduleNextTick();
    if (typeof action === 'object' && 'seek' in action) {
      const media = this.engineState.media;
      const dur = toMsNumber(media?.durationMs);
      const clamped =
        dur !== null && dur > 0
          ? clampSeekPositionMs(action.seek.positionMs, dur)
          : Math.max(0, Math.round(action.seek.positionMs));
      if (media) {
        const now = Date.now();
        this.clearOptimisticSeek();
        this.optimisticSeek = {
          trackKey: mediaTrackKey(media),
          targetMs: clamped,
          setAtEpochMs: now,
          untilEpochMs: now + OPTIMISTIC_SEEK_HOLD_MS,
        };
        this.optimisticSeekTimer = window.setTimeout(() => {
          this.optimisticSeekTimer = null;
          this.optimisticSeek = null;
        }, OPTIMISTIC_SEEK_HOLD_MS);
      }
      void invoke('media_command', { action: { seek: { positionMs: clamped } } }).catch(() => {});
      return;
    }
    if (action === 'next' || action === 'previous') {
      this.clearOptimisticSeek();
    }
    void invoke('media_command', { action }).catch(() => {});
  }

  toggleExpand = (): void => void invoke('toggle_expand');
  toggleLyricsCard = (): void => {
    this.lyricsCardOpen = !this.lyricsCardOpen;
    this.scheduler.markInteraction(Date.now());
    this.syncGeometry(false);
  };
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
