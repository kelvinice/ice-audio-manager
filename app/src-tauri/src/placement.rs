use std::sync::{Arc, Mutex, PoisonError, atomic::Ordering};

use ice_core::appearance::NotchStyle;
use ice_core::engine::{Command, PerformanceProfile, SettingsView};
use ice_core::placement::NotchAnchor;
use ice_win::fullscreen::{self, FullscreenEvent, FullscreenWatcher};
use ice_win::monitors::{MonitorInfo, PlacementMode, list_monitors, resolve_target_monitor};
use tauri::{AppHandle, Manager};

use super::{ActivityKind, MonitorSummary, Preview, SharedRuntime, request_preview};

#[derive(Default)]
pub(super) struct Placement {
    profile: Option<PerformanceProfile>,
    pinned: Option<String>,
    /// D-20: where on the pinned monitor's top or bottom edge both notch hosts sit (`ice_core::placement`).
    pub(super) anchor: NotchAnchor,
    /// D-20: optional continuous horizontal offset (`0.0..=1.0`) between Left and Right; `None` = preset anchor.
    pub(super) x_ratio: Option<f32>,
    /// D-20: Edge Nook (flush to edge) vs Floating (inset with shadow room).
    pub(super) style: NotchStyle,
    coverage: Option<fullscreen::Snapshot>,
    pub(super) gaming: bool,
    pub(super) generation: u64,
}

impl Placement {
    pub(super) fn allows_webview(&self) -> bool {
        !self.gaming && matches!(self.profile, Some(PerformanceProfile::Instant | PerformanceProfile::Balanced))
    }

    /// Whether the notch WebView hands over to the native pill after the idle linger. `Instant` keeps the WebView
    /// awake so the notch never falls back to the static pill (owner request, 2026-11-24); `Balanced` stays light.
    pub(super) fn suspends_on_idle(&self) -> bool {
        self.allows_webview() && self.profile == Some(PerformanceProfile::Balanced)
    }

    pub(super) fn can_reveal(&self, generation: u64) -> bool {
        self.allows_webview() && self.generation == generation
    }
}

pub(super) fn summaries(monitors: &[MonitorInfo]) -> Vec<MonitorSummary> {
    monitors
        .iter()
        .enumerate()
        .map(|(index, m)| MonitorSummary {
            index,
            id: m.id.clone(),
            display_path: m.display_path.clone(),
            name: m.friendly_name.clone(),
            is_primary: m.is_primary,
            left: m.left,
            top: m.top,
            width: m.width,
            height: m.height,
            scale_factor: m.scale_factor,
            refresh_hz: m.refresh_hz,
        })
        .collect()
}

// Keep the watcher alive until Exit and break the callback's AppHandle reference there.
type Watcher = Mutex<Option<FullscreenWatcher>>;

pub(super) fn watch(app: &AppHandle, state: &Arc<SharedRuntime>) {
    state.placement().coverage = fullscreen::snapshot().ok().flatten();
    let handle = app.clone();
    let shared = Arc::clone(state);
    match FullscreenWatcher::start(move |event| {
        let app = handle.clone();
        let state = Arc::clone(&shared);
        let _ = handle.run_on_main_thread(move || match event {
            FullscreenEvent::Changed(snapshot) => {
                state.placement().coverage = Some(snapshot);
                reconcile(&app, &state, None, false);
            }
            FullscreenEvent::DisplaysChanged(snapshot) => {
                if let Some(snapshot) = snapshot {
                    state.placement().coverage = Some(snapshot);
                }
                match list_monitors() {
                    Ok(monitors) => {
                        *state.monitors.lock().unwrap_or_else(PoisonError::into_inner) = monitors;
                        reconcile(&app, &state, None, true);
                    }
                    Err(error) => tracing::warn!(%error, "Could not refresh connected monitors"),
                }
            }
        });
    }) {
        Ok(watcher) => {
            app.manage(Watcher::new(Some(watcher)));
        }
        Err(error) => tracing::warn!(%error, "Fullscreen watcher unavailable"),
    }
}

pub(super) fn stop(app: &AppHandle) {
    if let Some(watcher) = app.try_state::<Watcher>() {
        watcher.lock().unwrap_or_else(PoisonError::into_inner).take();
    }
}

pub(super) fn settings(app: &AppHandle, state: &Arc<SharedRuntime>, settings: &SettingsView) {
    reconcile(app, state, Some(settings), false);
}

impl SharedRuntime {
    pub(super) fn set_notch_visible(&self, app: &AppHandle, visible: bool) {
        self.notch_visible.store(visible, Ordering::Release);
        super::meter::reconcile(app, self);
    }
}

pub(super) fn notch_destroyed(state: &SharedRuntime) {
    state.notch_visible.store(false, Ordering::Release);
    state.notch_suspended.store(false, Ordering::Release);
    state.want_suspended.store(false, Ordering::Release);
    state.pending_wake_seq.store(0, Ordering::Release);
    state.notch_hwnd.store(0, Ordering::Release);
    super::meter::released(state);
    if !state.bench_mode
        && let Some(dot) = state.native_dot.lock().unwrap_or_else(PoisonError::into_inner).as_ref()
    {
        dot.set_standby_only(state.placement().gaming);
    }
}

// All policy transitions and native window changes run serially on the shell's main thread.
fn reconcile(app: &AppHandle, state: &Arc<SharedRuntime>, settings: Option<&SettingsView>, topology: bool) {
    let mut p = state.placement();
    let previous = (p.profile, p.gaming);
    let mut repin = false;
    if let Some(settings) = settings {
        p.profile = Some(settings.performance_profile);
        if p.pinned != settings.pinned_monitor {
            p.pinned.clone_from(&settings.pinned_monitor);
            repin = true;
        }
        let anchor = settings.notch_anchor;
        let x_ratio = settings.notch_x_ratio;
        let style = settings.appearance.notch_style;
        let ratio_changed = match (p.x_ratio, x_ratio) {
            (None, None) => false,
            (Some(a), Some(b)) => (a - b).abs() >= 1e-4,
            _ => true,
        };
        if p.anchor != anchor || ratio_changed || p.style != style {
            p.anchor = anchor;
            p.x_ratio = x_ratio;
            p.style = style;
            repin = true;
        }
    }
    let (anchor, x_ratio, style) = (p.anchor, p.x_ratio, p.style);
    let Some(profile) = p.profile else { return };
    let monitors = state.monitors.lock().unwrap_or_else(PoisonError::into_inner);
    let target = resolve_target_monitor(&monitors, PlacementMode::Pinned, p.pinned.as_deref(), None);
    let first_pin = if p.pinned.is_none() { target.map(|(_, m)| m.display_path.clone()) } else { None };
    if let Some(path) = &first_pin {
        p.pinned = Some(path.clone());
    }
    let target = target.map(|(index, m)| (index, m.clone()));
    p.gaming =
        target.as_ref().is_none_or(|(_, m)| p.coverage.as_ref().is_some_and(|s| fullscreen::is_monitor_covered(s, m)));
    let gaming = p.gaming;
    let allowed = p.allows_webview();
    let changed = previous != (p.profile, gaming);
    if changed && !allowed {
        p.generation = p.generation.wrapping_add(1);
    }
    drop(p);
    let monitor_summaries = topology.then(|| summaries(&monitors));
    drop(monitors);
    if changed {
        super::meter::reconcile(app, state);
    }
    if let Some(display_path) = first_pin {
        state.engine.send(Command::SetPinnedMonitor { display_path });
    }

    if !changed && !topology && !repin {
        return;
    }
    if changed && !allowed {
        state.pending_wake_seq.store(0, Ordering::Release);
        state.suspend_gen.fetch_add(1, Ordering::AcqRel);
        state.set_notch_visible(app, false);
        state.want_suspended.store(false, Ordering::Release);
        state.notch_suspended.store(false, Ordering::Release);
        if let Some(win) = app.get_webview_window("notch") {
            let _ = win.hide();
            if let Err(error) = win.destroy() {
                tracing::warn!(%error, "Could not release notch WebView");
            }
        }
    }
    state.update_and_emit(app, |view| {
        view.anchor = anchor;
        view.active_monitor = target.as_ref().map_or(0, |(index, _)| *index);
        if let Some(summaries) = monitor_summaries {
            view.monitors = summaries;
        }
        if !allowed {
            view.expanded = false;
            view.suspended = false;
            view.activity = ActivityKind::Idle;
        }
    });
    if let Some(dot) = state.native_dot.lock().unwrap_or_else(PoisonError::into_inner).as_ref() {
        if gaming {
            dot.set_standby_only(true);
            dot.hide();
        }
        dot.set_position(anchor, x_ratio);
        if let Some((_, monitor)) = &target {
            dot.move_to_monitor(monitor.clone());
        }
        if !gaming && !state.notch_visible.load(Ordering::Acquire) {
            dot.set_standby_only(false);
            dot.show_idle();
        }
    }
    if let Some((_, monitor)) = target
        && allowed
    {
        let hwnd = state.notch_hwnd.load(Ordering::Acquire);
        if hwnd != 0
            && let Err(error) = super::overlay_win::move_notch(hwnd, &monitor, anchor, x_ratio, style)
        {
            tracing::warn!(%error, "Could not reposition notch");
        }
    }
    let preview = state.preview_on_attach.swap(false, Ordering::AcqRel);
    let minimized = state.start_minimized.swap(false, Ordering::AcqRel);
    let has_window = app.get_webview_window("notch").is_some();
    // Switching to Instant also wakes a notch that Balanced had suspended, so the native pill doesn't linger.
    let wake_instant = changed
        && profile == PerformanceProfile::Instant
        && !minimized
        && (!has_window || !state.notch_visible.load(Ordering::Acquire));
    if allowed && ((preview && !has_window) || wake_instant) {
        request_preview(app, state, Preview::Notch);
    }
    tracing::debug!(?profile, gaming, "Applied notch placement");
}

/// Top-left (physical px) that centres a `logical_w × logical_h` window in `monitor`'s work area, so Settings opens
/// on the monitor the notch is on (owner request, 2026-11-23). Clamped to the work area's top-left if it doesn't fit.
pub(super) fn centred_in_work_area(monitor: &MonitorInfo, logical_w: i32, logical_h: i32) -> (i32, i32) {
    let (w, h) = (monitor.logical_to_physical(logical_w), monitor.logical_to_physical(logical_h));
    let x = monitor.work_left + ((monitor.work_width - w) / 2).max(0);
    let y = monitor.work_top + ((monitor.work_height - h) / 2).max(0);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_centre_on_the_notch_monitor_work_area() {
        // A 150 % secondary monitor to the right of a 1920 px primary, with a 48 px bottom taskbar.
        let monitor = MonitorInfo {
            id: r"\\.\DISPLAY2".into(),
            display_path: "DISPLAY2".into(),
            friendly_name: "Secondary".into(),
            is_primary: false,
            left: 1920,
            top: 0,
            width: 2560,
            height: 1440,
            work_left: 1920,
            work_top: 0,
            work_width: 2560,
            work_height: 1392,
            dpi_x: 144,
            scale_factor: 1.5,
            refresh_hz: 75,
        };
        // 560×420 logical = 840×630 physical at 150 %.
        assert_eq!(centred_in_work_area(&monitor, 560, 420), (1920 + 860, 381));
        // Too big for the work area: pinned to its top-left instead of going off-screen.
        assert_eq!(centred_in_work_area(&monitor, 4000, 2000), (1920, 0));
    }

    #[test]
    fn gaming_and_lite_override_every_webview_profile() {
        for profile in [PerformanceProfile::Instant, PerformanceProfile::Balanced, PerformanceProfile::Lite] {
            for gaming in [false, true] {
                let policy = Placement { profile: Some(profile), gaming, generation: 2, ..Default::default() };
                assert_eq!(policy.can_reveal(2), !gaming && profile != PerformanceProfile::Lite);
                assert!(!policy.can_reveal(1));
            }
        }
        assert!(!Placement::default().allows_webview());
    }
}
