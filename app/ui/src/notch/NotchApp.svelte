<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import type { Command, EngineSnapshot, Envelope, MediaAction } from '../lib/ipc/engine';
  import type { HitRect, NotchDelta, NotchSnapshot, SpringPreset, ThemeKind } from '../lib/ipc/types';
  import { SpringGroup } from '../lib/motion/spring.svelte';
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
  } from '../lib/notch/scheduler';
  import ExpandedNotch from './activities/ExpandedNotch.svelte';
  import IdlePill from './activities/IdlePill.svelte';
  import MicActivity from './activities/MicActivity.svelte';
  import PlugActivity from './activities/PlugActivity.svelte';
  import SwitchActivity from './activities/SwitchActivity.svelte';
  import TrackActivity from './activities/TrackActivity.svelte';
  import VolumeActivity from './activities/VolumeActivity.svelte';

  const CANVAS_W = 420;
  const CANVAS_H = 190;
  const TOP_PAD = 6;

  const scheduler = new NotchActivityScheduler(Date.now());

  let notchSnapshot = $state<NotchSnapshot | null>(null);
  let engineState = $state(scheduler.state);
  let benchOverride = $state<ActivityView | null>(null);
  let hoverExpanded = $state(false);
  let pointerInside = $state(false);
  const idleMode = $derived(notchSnapshot?.idleStyle ?? DEFAULT_IDLE_MODE);
  let pillSize = $state({ w: 116, h: 28, r: 14, vol: 40 });
  let frameRate = $state(0);

  let hoverTimer: number | null = null;
  let leaveTimer: number | null = null;
  let activityTimer: number | null = null;
  let lastReportedSeq = 0;
  let lastFrameTs = 0;
  let frameCount = 0;
  let fpsWindowStart = 0;
  let resnapshotting = false;

  const isExpanded = $derived((notchSnapshot?.expanded ?? false) || hoverExpanded);
  const isInstant = $derived(
    (notchSnapshot?.systemReduceMotion ?? false) || (notchSnapshot?.preset ?? 'smooth') === 'off',
  );
  const isSuspended = $derived(notchSnapshot?.suspended ?? false);
  const isNotchVisible = $derived(!(notchSnapshot?.hiddenForBench ?? false) && !isSuspended);
  const activeView = $derived<ActivityView>(benchOverride ?? engineState.activity);

  const effectiveMicMuted = $derived(engineState.micMuted ?? notchSnapshot?.micMuted ?? false);
  const effectiveVolumePercent = $derived(
    activeView.kind === 'volume'
      ? activeView.percent
      : (engineState.volume?.percent ?? notchSnapshot?.volumePercent ?? 0),
  );
  const effectiveVolumeMuted = $derived(
    activeView.kind === 'volume' ? activeView.muted : (engineState.volume?.muted ?? notchSnapshot?.muted ?? false),
  );
  const effectiveDeviceName = $derived(
    engineState.activeDevice?.name ?? (notchSnapshot?.activeDevice || 'No playback device'),
  );
  const effectiveSpatialBadge = $derived(
    engineState.activeDevice
      ? formatSpatialBadge(engineState.activeDevice.spatial)
      : (notchSnapshot?.spatialBadge ?? null),
  );
  const displayedMedia = $derived(
    activeView.kind === 'track' ? activeView.media : engineState.media,
  );
  const activeAccentCss = $derived(
    isMediaActive(displayedMedia)
      ? (formatRgbCss(displayedMedia.art?.accent) ?? 'var(--ice-accent)')
      : 'var(--ice-accent)',
  );
  const activeGlowCss = $derived(
    isMediaActive(displayedMedia)
      ? (formatRgbGlowCss(displayedMedia.art?.accent, 0.24) ?? 'var(--ice-accent-glow)')
      : 'var(--ice-accent-glow)',
  );
  const isIdleHidden = $derived(
    !isExpanded && activeView.kind === 'idle' && idleMode === 'hidden',
  );

  function targetGeometry(
    view: ActivityView,
    expandedNow: boolean,
    micMutedNow: boolean,
    idleModeNow: IdleMode = idleMode,
  ) {
    const vol =
      view.kind === 'volume'
        ? view.percent
        : (engineState.volume?.percent ?? notchSnapshot?.volumePercent ?? 40);
    const hasMedia = isMediaActive(engineState.media);
    if (expandedNow) {
      return { w: 384, h: hasMedia ? 178 : 170, r: 22, vol };
    }
    switch (view.kind) {
      case 'volume':
        return { w: 236, h: 38, r: 19, vol: view.percent };
      case 'switch':
        return { w: 296, h: 44, r: 22, vol };
      case 'plug':
        return { w: 268, h: 44, r: 22, vol };
      case 'mic':
        return { w: 204, h: 38, r: 19, vol };
      case 'track':
        return { w: 296, h: 48, r: 24, vol };
      case 'idle':
      default:
        if (idleModeNow === 'hidden') {
          return { w: 96, h: 10, r: 5, vol };
        }
        if (idleModeNow === 'dot') {
          if (hasMedia) {
            return { w: micMutedNow ? 58 : 44, h: 18, r: 9, vol };
          }
          return { w: micMutedNow ? 42 : 36, h: 18, r: 9, vol };
        }
        if (hasMedia) {
          return { w: micMutedNow ? 216 : 196, h: 28, r: 14, vol };
        }
        return { w: micMutedNow ? 132 : 116, h: 28, r: 14, vol };
    }
  }

  const springs = new SpringGroup<'w' | 'h' | 'r' | 'vol'>(
    { w: 116, h: 28, r: 14, vol: 40 },
    'smooth',
    (vals) => {
      pillSize = { w: vals.w, h: vals.h, r: vals.r, vol: vals.vol };
      const now = performance.now();
      if (lastFrameTs > 0) {
        frameCount += 1;
        if (now - fpsWindowStart >= 350) {
          frameRate = Math.round((frameCount * 1000) / (now - fpsWindowStart));
          frameCount = 0;
          fpsWindowStart = now;
        }
      } else {
        fpsWindowStart = now;
      }
      lastFrameTs = now;
    },
  );

  function effectivePreset(snap: NotchSnapshot | null): SpringPreset {
    if (!snap) return 'smooth';
    return snap.systemReduceMotion ? 'off' : snap.preset;
  }

  function clearActivityTimer() {
    if (activityTimer !== null) {
      clearTimeout(activityTimer);
      activityTimer = null;
    }
  }

  function scheduleNextTick() {
    clearActivityTimer();
    const snap = notchSnapshot;
    if (!snap || snap.hiddenForBench || snap.suspended || benchOverride !== null) {
      return;
    }
    const holdOpen = snap.expanded || hoverExpanded || pointerInside;
    const delay = scheduler.nextTickDelayMs(Date.now(), holdOpen);
    if (delay === null) {
      return;
    }
    activityTimer = window.setTimeout(() => {
      activityTimer = null;
      const nowSnap = notchSnapshot;
      if (!nowSnap || nowSnap.hiddenForBench || nowSnap.suspended) {
        return;
      }
      const holding = nowSnap.expanded || hoverExpanded || pointerInside;
      const outcome = scheduler.tick(Date.now(), holding);
      engineState = { ...scheduler.state };
      if (outcome.dismissedToIdle) {
        syncGeometry(false);
      }
      if (outcome.shouldRequestSuspend && !nowSnap.hiddenForBench && !nowSnap.suspended) {
        void invoke('request_idle_suspend');
        return;
      }
      scheduleNextTick();
    }, delay);
  }

  function syncGeometry(instantMove: boolean) {
    const snap = notchSnapshot;
    const preset = effectivePreset(snap);
    const paused = (snap?.hiddenForBench ?? false) || (snap?.suspended ?? false);
    springs.setPaused(paused);
    springs.setPreset(preset);

    const view = benchOverride ?? engineState.activity;
    const expandedNow = (snap?.expanded ?? false) || hoverExpanded;
    const micMutedNow = engineState.micMuted ?? snap?.micMuted ?? false;
    const target = targetGeometry(view, expandedNow, micMutedNow, idleMode);

    if (instantMove || preset === 'off' || paused) {
      springs.snapAll(target);
    } else {
      springs.setTargets(target);
    }

    const hitRect: HitRect = {
      x: (CANVAS_W - target.w) / 2,
      y: TOP_PAD,
      width: target.w,
      height: target.h,
    };
    void invoke('report_hit_rect', { rect: hitRect });
  }

  function acknowledgeRenderedSeq(seq: number) {
    if (seq <= lastReportedSeq) return;
    lastReportedSeq = seq;
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        void invoke('report_event_rendered', { eventSeq: seq });
      });
    });
  }

  function applyNotchSnapshot(next: NotchSnapshot, instant = false) {
    const prevActivity = notchSnapshot?.activity;
    const prevSeq = notchSnapshot?.eventSeq ?? 0;
    const wasSuspended = notchSnapshot?.suspended ?? false;
    notchSnapshot = next;
    document.documentElement.dataset.theme = next.theme;

    // Preserve --bench driver compatibility when NotchSnapshot changes outside EngineEvent.
    if (next.hiddenForBench) {
      benchOverride = null;
    } else if (next.eventSeq > prevSeq || (next.activity !== prevActivity && next.activity === 'idle')) {
      if (next.activity === 'idle') {
        benchOverride = null;
      } else if (engineState.activity.kind === 'idle') {
        if (next.activity === 'volume') {
          benchOverride = {
            kind: 'volume',
            channel: 'playback',
            percent: next.volumePercent,
            muted: next.muted,
            deviceName: next.activeDevice,
            formFactor: null,
            entrySeq: next.eventSeq,
            boundBump: 0,
            bumpSeq: 0,
            expiresAtMs: Date.now(),
          };
        } else if (next.activity === 'mic_mute') {
          benchOverride = {
            kind: 'mic',
            muted: next.micMuted,
            deviceName: 'Microphone',
            entranceSeq: next.eventSeq,
            expiresAtMs: Date.now(),
          };
        }
      }
    }

    syncGeometry(instant || wasSuspended !== next.suspended);
    scheduleNextTick();

    if (next.eventSeq > lastReportedSeq && !next.suspended) {
      acknowledgeRenderedSeq(next.eventSeq);
    }
  }

  function applyEngineSnapshot(snapshot: EngineSnapshot) {
    if (snapshot.revision < scheduler.state.revision) return;
    scheduler.applySnapshot(snapshot, Date.now());
    engineState = { ...scheduler.state };
    syncGeometry(true);
    scheduleNextTick();
  }

  async function resnapshotEngine() {
    if (resnapshotting) return;
    resnapshotting = true;
    try {
      const fresh = await invoke<EngineSnapshot>('get_engine_snapshot');
      applyEngineSnapshot(fresh);
    } finally {
      resnapshotting = false;
    }
  }

  function applyEngineEnvelope(delta: Envelope) {
    const res = scheduler.applyEnvelope(delta, Date.now());
    if (res.status === 'needs_resnapshot') {
      void resnapshotEngine();
      return;
    }
    if (res.status === 'ignored_stale') {
      return;
    }
    engineState = { ...scheduler.state };
    if (
      res.wokeActivity ||
      delta.event.type === 'devicesChanged' ||
      delta.event.type === 'mediaChanged'
    ) {
      benchOverride = null;
      syncGeometry(false);
      scheduleNextTick();
    }
    if (res.wokeActivity && notchSnapshot && !notchSnapshot.suspended && notchSnapshot.eventSeq > lastReportedSeq) {
      acknowledgeRenderedSeq(notchSnapshot.eventSeq);
    }
  }

  onMount(() => {
    let disposed = false;
    const bufferedNotch: NotchDelta[] = [];
    const bufferedEngine: Envelope[] = [];
    let notchInitialized = false;
    let engineInitialized = false;
    let bufferedEngineSnapshot: EngineSnapshot | null = null;

    const unlistenNotchPromise = listen<NotchDelta>('notch://delta/v1', (ev) => {
      if (disposed) return;
      if (!notchInitialized) {
        bufferedNotch.push(ev.payload);
        return;
      }
      const next = ev.payload.snapshot;
      if (!notchSnapshot || next.revision > notchSnapshot.revision) {
        applyNotchSnapshot(next);
      }
    });

    const unlistenEnginePromise = listen<Envelope>('engine://delta/v1', (ev) => {
      if (disposed) return;
      if (!engineInitialized) {
        bufferedEngine.push(ev.payload);
        return;
      }
      applyEngineEnvelope(ev.payload);
    });

    const unlistenEngineSnapshotPromise = listen<EngineSnapshot>('engine://snapshot/v1', ({ payload }) => {
      if (disposed) return;
      if (!engineInitialized) {
        bufferedEngineSnapshot = payload;
        return;
      }
      applyEngineSnapshot(payload);
    });

    void (async () => {
      await Promise.all([unlistenNotchPromise, unlistenEnginePromise, unlistenEngineSnapshotPromise]);
      if (disposed) return;
      const [initialNotch, initialEngine] = await Promise.all([
        invoke<NotchSnapshot>('get_notch_snapshot'),
        invoke<EngineSnapshot>('get_engine_snapshot'),
      ]);
      if (disposed) return;

      engineInitialized = true;
      applyEngineSnapshot(bufferedEngineSnapshot && bufferedEngineSnapshot.revision > initialEngine.revision
        ? bufferedEngineSnapshot
        : initialEngine);
      for (const delta of bufferedEngine) {
        if (delta.revision > scheduler.state.revision) {
          applyEngineEnvelope(delta);
        }
      }

      notchInitialized = true;
      let latestNotch = initialNotch;
      for (const delta of bufferedNotch) {
        if (delta.snapshot.revision > latestNotch.revision) {
          latestNotch = delta.snapshot;
        }
      }
      applyNotchSnapshot(latestNotch, true);

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          void invoke('report_first_paint');
        });
      });
    })();

    return () => {
      disposed = true;
      springs.dispose();
      clearTimers();
      clearActivityTimer();
      void unlistenNotchPromise.then((fn) => fn());
      void unlistenEnginePromise.then((fn) => fn());
      void unlistenEngineSnapshotPromise.then((fn) => fn());
    };
  });

  function clearTimers() {
    if (hoverTimer !== null) {
      clearTimeout(hoverTimer);
      hoverTimer = null;
    }
    if (leaveTimer !== null) {
      clearTimeout(leaveTimer);
      leaveTimer = null;
    }
  }

  function onPointerEnter() {
    clearTimers();
    pointerInside = true;
    scheduler.markInteraction(Date.now());
    scheduleNextTick();
    if (hoverExpanded || notchSnapshot?.expanded) return;
    hoverTimer = window.setTimeout(() => {
      hoverExpanded = true;
      syncGeometry(false);
      scheduleNextTick();
    }, 120);
  }

  function onPointerLeave() {
    clearTimers();
    pointerInside = false;
    scheduler.markInteraction(Date.now());
    if (!hoverExpanded) {
      scheduleNextTick();
      return;
    }
    leaveTimer = window.setTimeout(() => {
      hoverExpanded = false;
      syncGeometry(false);
      scheduleNextTick();
    }, 250);
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    scheduler.markInteraction(Date.now());
    if (e.shiftKey) {
      void invoke('trigger_switch');
    } else {
      const step = e.deltaY < 0 ? 4 : -4;
      void invoke('adjust_volume', { delta: step });
    }
  }

  function onAuxClick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault();
      scheduler.markInteraction(Date.now());
      void invoke('toggle_mute');
    }
  }

  async function sendMediaAction(action: MediaAction) {
    scheduler.markInteraction(Date.now());
    scheduleNextTick();
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

  function setIdleMode(mode: IdleMode) {
    scheduler.markInteraction(Date.now());
    syncGeometry(false);
    scheduleNextTick();
    void invoke('set_idle_style', { style: mode }).catch(() => {});
  }

  const insetLeft = $derived(Math.max(0, (CANVAS_W - pillSize.w) / 2));
  const insetBottom = $derived(Math.max(0, CANVAS_H - TOP_PAD - pillSize.h));
  const clipPath = $derived(
    `inset(${TOP_PAD}px ${insetLeft.toFixed(2)}px ${insetBottom.toFixed(2)}px ${insetLeft.toFixed(2)}px round ${pillSize.r.toFixed(2)}px)`,
  );
  const targetBox = $derived(targetGeometry(activeView, isExpanded, effectiveMicMuted, idleMode));

  const rimStroke = $derived.by(() => {
    if (isExpanded) return 'var(--ice-rim)';
    switch (activeView.kind) {
      case 'mic':
        return activeView.muted ? 'var(--ice-danger)' : 'var(--ice-ok)';
      case 'volume':
        return activeView.muted ? 'var(--ice-danger)' : 'var(--ice-rim)';
      case 'plug':
        return activeView.direction === 'arrived' ? 'var(--ice-ok)' : 'var(--ice-warn)';
      case 'switch':
        return 'var(--ice-rim)';
      case 'idle':
      default:
        return effectiveMicMuted ? 'var(--ice-danger)' : 'var(--ice-rim)';
    }
  });
</script>

<div
  class="stage"
  class:hidden={(notchSnapshot?.hiddenForBench ?? false) || isSuspended}
  class:idle-hidden={isIdleHidden}
  class:instant={isInstant}
  style:width={`${CANVAS_W}px`}
  style:height={`${CANVAS_H}px`}
>
  <div
    class="pill-clip"
    style:clip-path={clipPath}
    onpointerenter={onPointerEnter}
    onpointerleave={onPointerLeave}
    onwheel={onWheel}
    onauxclick={onAuxClick}
    role="region"
    aria-label="Ice Audio Notch"
  >
    <div
      class="pill-surface"
      style:--ice-active-accent={activeAccentCss}
      style:--ice-active-glow={activeGlowCss}
    >
      <div class="accent-glow" class:instant={isInstant} aria-hidden="true"></div>
      <div
        class="pill-slot"
        style:top={`${TOP_PAD}px`}
        style:width={`${targetBox.w}px`}
        style:height={`${targetBox.h}px`}
      >
        {#if notchSnapshot && !isExpanded}
          {#if activeView.kind === 'volume'}
            <VolumeActivity activity={activeView} barPercent={pillSize.vol} instant={isInstant} />
          {:else if activeView.kind === 'switch'}
            <SwitchActivity activity={activeView} instant={isInstant} />
          {:else if activeView.kind === 'plug'}
            <PlugActivity activity={activeView} instant={isInstant} />
          {:else if activeView.kind === 'mic'}
            <MicActivity activity={activeView} instant={isInstant} />
          {:else if activeView.kind === 'track'}
            <TrackActivity
              activity={activeView}
              instant={isInstant}
              onMediaAction={(action) => void sendMediaAction(action)}
            />
          {:else}
            <IdlePill
              deviceLabel={shortDeviceName(effectiveDeviceName)}
              volumePercent={effectiveVolumePercent}
              muted={effectiveVolumeMuted}
              micMuted={effectiveMicMuted}
              media={engineState.media}
              {idleMode}
              preset={notchSnapshot.preset}
              instant={isInstant}
              notchVisible={isNotchVisible}
            />
          {/if}
        {:else if notchSnapshot && isExpanded}
          <ExpandedNotch
            playback={engineState.playback}
            activeDeviceName={effectiveDeviceName}
            spatialBadge={effectiveSpatialBadge}
            volumePercent={effectiveVolumePercent}
            muted={effectiveVolumeMuted}
            barPercent={pillSize.vol}
            micMuted={effectiveMicMuted}
            media={engineState.media}
            instant={isInstant}
            {idleMode}
            notchVisible={isNotchVisible}
            activeMonitor={notchSnapshot.activeMonitor}
            monitors={notchSnapshot.monitors}
            preset={notchSnapshot.preset}
            theme={notchSnapshot.theme}
            {frameRate}
            expanded={notchSnapshot.expanded}
            onToggleExpand={() => void invoke('toggle_expand')}
            onToggleMute={() => void invoke('toggle_mute')}
            onAdjustVolume={(delta) => void invoke('adjust_volume', { delta })}
            onSwitchDevice={(deviceId) => void invoke('switch_to_device', { deviceId })}
            onToggleMicMute={() => void invoke('toggle_mic_mute')}
            onMediaAction={(action) => void sendMediaAction(action)}
            onSetIdleMode={setIdleMode}
            onCycleMonitor={() => void invoke('cycle_monitor')}
            onOpenSettings={() => void invoke('open_settings')}
            onSetPreset={(preset) => void invoke('set_preset', { preset })}
            onSetTheme={(theme) => void invoke('set_theme', { theme })}
          />
        {/if}
      </div>
    </div>
  </div>

  <!-- Crisp 1px SVG rim driven by spring (w, h, r) with vector-effect="non-scaling-stroke" -->
  <svg class="rim-svg" width={CANVAS_W} height={CANVAS_H} viewBox={`0 0 ${CANVAS_W} ${CANVAS_H}`} aria-hidden="true">
    <rect
      x={insetLeft + 0.5}
      y={TOP_PAD + 0.5}
      width={Math.max(1, pillSize.w - 1)}
      height={Math.max(1, pillSize.h - 1)}
      rx={pillSize.r}
      ry={pillSize.r}
      fill="none"
      stroke={rimStroke}
      stroke-width="1"
      vector-effect="non-scaling-stroke"
    />
  </svg>
</div>

<style>
  .stage {
    position: relative;
    margin: 0 auto;
    background: transparent;
    filter: drop-shadow(var(--ice-drop-shadow));
  }

  .stage.hidden {
    opacity: 0;
    pointer-events: none;
  }

  .stage.idle-hidden .pill-surface,
  .stage.idle-hidden .rim-svg {
    opacity: 0;
  }

  .pill-clip {
    position: absolute;
    inset: 0;
    background: transparent;
  }

  .pill-surface {
    position: absolute;
    inset: 0;
    background: var(--ice-bg);
  }

  .accent-glow {
    position: absolute;
    inset: 0;
    opacity: 0.28;
    pointer-events: none;
    background: radial-gradient(
      62% 44% at 50% 0%,
      var(--ice-active-glow, var(--ice-accent-glow)),
      transparent 72%
    );
    transition: opacity 400ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .accent-glow.instant {
    transition: none;
  }

  .pill-slot {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    padding: 0 12px;
    overflow: hidden;
  }

  .rim-svg {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
</style>
