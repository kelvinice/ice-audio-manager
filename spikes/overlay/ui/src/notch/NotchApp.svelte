<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import type { HitRect, NotchDelta, NotchSnapshot, SpringPreset, ThemeKind } from '../lib/ipc/types';
  import { SpringGroup } from '../lib/motion/spring.svelte';

  const CANVAS_W = 420;
  const CANVAS_H = 190;
  const TOP_PAD = 6;

  let snapshot = $state<NotchSnapshot | null>(null);
  let hoverExpanded = $state(false);
  let pillSize = $state({ w: 112, h: 28, r: 14, vol: 64 });
  let frameRate = $state(0);

  let hoverTimer: number | null = null;
  let leaveTimer: number | null = null;
  let lastReportedSeq = 0;
  let lastFrameTs = 0;
  let frameCount = 0;
  let fpsWindowStart = 0;

  const isExpanded = $derived((snapshot?.expanded ?? false) || hoverExpanded);

  function computeTarget(snap: NotchSnapshot | null, expandedNow: boolean) {
    if (!snap) return { w: 112, h: 28, r: 14, vol: 64 };
    if (expandedNow) {
      return { w: 368, h: 158, r: 22, vol: snap.volumePercent };
    }
    switch (snap.activity) {
      case 'volume':
        return { w: 232, h: 38, r: 19, vol: snap.volumePercent };
      case 'switch':
        return { w: 292, h: 44, r: 21, vol: snap.volumePercent };
      case 'mic_mute':
        return { w: 204, h: 36, r: 18, vol: snap.volumePercent };
      case 'idle':
      default:
        return { w: 112, h: 28, r: 14, vol: snap.volumePercent };
    }
  }

  const springs = new SpringGroup<'w' | 'h' | 'r' | 'vol'>(
    { w: 112, h: 28, r: 14, vol: 64 },
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

  function effectivePreset(snap: NotchSnapshot): SpringPreset {
    return snap.systemReduceMotion ? 'off' : snap.preset;
  }

  function applySnapshot(next: NotchSnapshot, instant = false) {
    snapshot = next;
    document.documentElement.dataset.theme = next.theme;
    const preset = effectivePreset(next);
    springs.setPaused(next.hiddenForBench);
    springs.setPreset(preset);

    const expandedNow = next.expanded || hoverExpanded;
    const target = computeTarget(next, expandedNow);
    if (instant || preset === 'off') {
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

    if (next.eventSeq > lastReportedSeq) {
      const seq = next.eventSeq;
      lastReportedSeq = seq;
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          void invoke('report_event_rendered', { eventSeq: seq });
        });
      });
    }
  }

  onMount(() => {
    let disposed = false;
    const buffered: NotchDelta[] = [];
    let initialized = false;

    const unlistenPromise = listen<NotchDelta>('notch://delta/v1', (ev) => {
      if (disposed) return;
      if (!initialized) {
        buffered.push(ev.payload);
        return;
      }
      const next = ev.payload.snapshot;
      if (!snapshot || next.revision > snapshot.revision) {
        applySnapshot(next);
      }
    });

    void (async () => {
      const initial = await invoke<NotchSnapshot>('get_notch_snapshot');
      if (disposed) return;
      initialized = true;
      let latest = initial;
      for (const delta of buffered) {
        if (delta.snapshot.revision > latest.revision) {
          latest = delta.snapshot;
        }
      }
      applySnapshot(latest, true);
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          void invoke('report_first_paint');
        });
      });
    })();

    return () => {
      disposed = true;
      springs.dispose();
      if (hoverTimer !== null) clearTimeout(hoverTimer);
      if (leaveTimer !== null) clearTimeout(leaveTimer);
      void unlistenPromise.then((fn) => fn());
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
    if (hoverExpanded || snapshot?.expanded) return;
    hoverTimer = window.setTimeout(() => {
      hoverExpanded = true;
      if (snapshot) applySnapshot(snapshot);
    }, 120);
  }

  function onPointerLeave() {
    clearTimers();
    if (!hoverExpanded) return;
    leaveTimer = window.setTimeout(() => {
      hoverExpanded = false;
      if (snapshot) applySnapshot(snapshot);
    }, 250);
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
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
      void invoke('toggle_mute');
    }
  }

  const insetLeft = $derived(Math.max(0, (CANVAS_W - pillSize.w) / 2));
  const insetBottom = $derived(Math.max(0, CANVAS_H - TOP_PAD - pillSize.h));
  const clipPath = $derived(
    `inset(${TOP_PAD}px ${insetLeft.toFixed(2)}px ${insetBottom.toFixed(2)}px ${insetLeft.toFixed(2)}px round ${pillSize.r.toFixed(2)}px)`,
  );
  const currentMonitor = $derived(snapshot?.monitors[snapshot.activeMonitor]);
  const targetBox = $derived(computeTarget(snapshot, isExpanded));
</script>

<div
  class="stage"
  class:hidden={snapshot?.hiddenForBench ?? false}
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
    <div class="pill-surface">
      <div
        class="pill-slot"
        style:top={`${TOP_PAD}px`}
        style:width={`${targetBox.w}px`}
        style:height={`${targetBox.h}px`}
      >
        {#if snapshot && !isExpanded}
          {#if snapshot.activity === 'idle'}
            <div class="row idle-row">
              <span class="dot" class:mic-muted={snapshot.micMuted}></span>
              <span class="idle-label">{snapshot.activeDevice.split('(')[0].trim()}</span>
              <span class="vol-mini">{snapshot.muted ? 'Mute' : `${snapshot.volumePercent}%`}</span>
            </div>
          {:else if snapshot.activity === 'volume'}
            <div class="row compact-row">
              <span class="badge-icon">{snapshot.muted ? '🔇' : '🔊'}</span>
              <div class="bar-track">
                <div
                  class="bar-fill"
                  class:muted={snapshot.muted}
                  style:transform={`scaleX(${Math.max(0, Math.min(1, pillSize.vol / 100)).toFixed(4)})`}
                ></div>
              </div>
              <span class="pct">{snapshot.muted ? 'Mute' : `${snapshot.volumePercent}%`}</span>
            </div>
          {:else if snapshot.activity === 'switch'}
            <div class="row compact-row">
              <span class="badge-icon">🎧</span>
              <span class="dev-name">{snapshot.activeDevice}</span>
              {#if snapshot.spatialBadge}
                <span class="spatial-pill">{snapshot.spatialBadge}</span>
              {/if}
            </div>
          {:else}
            <div class="row compact-row">
              <span class="dot mic-dot" class:mic-muted={snapshot.micMuted}></span>
              <span class="dev-name">{snapshot.micMuted ? 'Mic Muted' : 'Mic Live'}</span>
            </div>
          {/if}
        {:else if snapshot && isExpanded}
          <div class="expanded-card">
            <div class="exp-header">
              <div class="exp-title">
                <span class="accent-dot"></span>
                <span>{snapshot.activeDevice}</span>
                {#if snapshot.spatialBadge}
                  <span class="spatial-pill">{snapshot.spatialBadge}</span>
                {/if}
              </div>
              <button class="chip" type="button" onclick={() => void invoke('toggle_expand')}>Pin</button>
            </div>

            <div class="exp-vol">
              <button class="chip" type="button" onclick={() => void invoke('toggle_mute')}>
                {snapshot.muted ? 'Unmute' : 'Mute'}
              </button>
              <div class="bar-track">
                <div
                  class="bar-fill"
                  class:muted={snapshot.muted}
                  style:transform={`scaleX(${Math.max(0, Math.min(1, pillSize.vol / 100)).toFixed(4)})`}
                ></div>
              </div>
              <span class="pct">{snapshot.volumePercent}%</span>
              <button class="chip" type="button" onclick={() => void invoke('adjust_volume', { delta: -8 })}>-</button>
              <button class="chip" type="button" onclick={() => void invoke('adjust_volume', { delta: 8 })}>+</button>
            </div>

            <div class="exp-actions">
              <button class="chip primary" type="button" onclick={() => void invoke('trigger_switch')}>
                Switch ↔ {snapshot.targetDevice.split('(')[0].trim()}
              </button>
              <button
                class="chip"
                class:danger-chip={snapshot.micMuted}
                type="button"
                onclick={() => void invoke('toggle_mic_mute')}
              >
                {snapshot.micMuted ? 'Mic Off' : 'Mic On'}
              </button>
              {#if snapshot.monitors.length > 1}
                <button class="chip" type="button" onclick={() => void invoke('cycle_monitor')}>
                  Mon {snapshot.activeMonitor + 1} ({currentMonitor?.refreshHz ?? 60}Hz)
                </button>
              {/if}
              <button class="chip" type="button" onclick={() => void invoke('open_settings')}>Settings</button>
            </div>

            <div class="exp-footer">
              <div class="preset-row">
                {#each ['snappy', 'smooth', 'bouncy', 'off'] as SpringPreset[] as p}
                  <button
                    class="mini-btn"
                    class:active={snapshot.preset === p}
                    type="button"
                    onclick={() => void invoke('set_preset', { preset: p })}
                  >
                    {p}
                  </button>
                {/each}
              </div>
              <div class="meta">
                {#each ['ice', 'midnight', 'fluent', 'minimal'] as ThemeKind[] as t}
                  <button
                    class="mini-btn"
                    class:active={snapshot.theme === t}
                    type="button"
                    onclick={() => void invoke('set_theme', { theme: t })}
                  >
                    {t[0].toUpperCase()}
                  </button>
                {/each}
                <span class="fps">{frameRate > 0 ? `${frameRate}Hz` : `${currentMonitor?.refreshHz ?? 60}Hz`}</span>
              </div>
            </div>
          </div>
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
      stroke={snapshot?.micMuted && !isExpanded ? 'var(--ice-danger)' : 'var(--ice-rim)'}
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
    filter: drop-shadow(0 8px 18px rgba(0, 0, 0, 0.45));
  }

  .stage.hidden {
    opacity: 0;
    pointer-events: none;
  }

  .pill-clip {
    position: absolute;
    inset: 0;
    background: transparent;
  }

  .pill-surface {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(65% 45% at 50% 0%, var(--ice-accent-glow), transparent 72%),
      var(--ice-bg);
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

  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }

  .idle-row {
    justify-content: space-between;
    font-size: 11px;
  }

  .idle-label,
  .dev-name {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ice-fg);
  }

  .vol-mini,
  .pct {
    font-family: var(--ice-mono);
    font-size: 11px;
    color: var(--ice-fg-muted);
  }

  .dot,
  .accent-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--ice-accent);
    flex-shrink: 0;
  }

  .dot.mic-muted {
    background: var(--ice-danger);
  }

  .badge-icon {
    font-size: 12px;
    flex-shrink: 0;
  }

  .bar-track {
    flex: 1;
    height: var(--ice-bar-height);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.14);
    overflow: hidden;
  }

  .bar-fill {
    width: 100%;
    height: 100%;
    border-radius: 999px;
    transform-origin: left center;
    background: linear-gradient(90deg, var(--ice-accent), var(--ice-accent-strong));
  }

  .bar-fill.muted {
    background: var(--ice-danger);
  }

  .spatial-pill {
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 10px;
    background: var(--ice-accent-soft);
    color: var(--ice-accent-strong);
    border: 1px solid var(--ice-border);
    flex-shrink: 0;
  }

  .expanded-card {
    width: 100%;
    height: 100%;
    padding: 10px 2px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 8px;
  }

  .exp-header,
  .exp-vol,
  .exp-actions,
  .exp-footer,
  .preset-row,
  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .exp-header,
  .exp-footer {
    justify-content: space-between;
  }

  .exp-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip,
  .mini-btn {
    appearance: none;
    border: 1px solid var(--ice-border);
    background: var(--ice-surface);
    color: var(--ice-fg);
    border-radius: 8px;
    padding: 3px 8px;
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
  }

  .chip.primary {
    border-color: var(--ice-accent);
    background: var(--ice-accent-soft);
    color: var(--ice-accent-strong);
  }

  .chip.danger-chip {
    border-color: var(--ice-danger);
    color: var(--ice-danger);
  }

  .mini-btn {
    padding: 2px 6px;
    font-size: 10px;
    color: var(--ice-fg-muted);
  }

  .mini-btn.active {
    border-color: var(--ice-accent);
    color: var(--ice-accent-strong);
  }

  .fps {
    font-family: var(--ice-mono);
    font-size: 10px;
    color: var(--ice-fg-muted);
  }
</style>
