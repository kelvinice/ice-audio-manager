//! D-20 flexible drag & drop: one drag loop for both hosts (the WebView notch and the native pill).
//!
//! A host reports that a press moved past Windows' drag threshold (the WebView through `start_notch_drag`, the native
//! pill through `PillInput::DragStart`). From then on the shell follows the cursor itself on its own thread: it never
//! uses the system move loop (`HTCAPTION`/`SC_MOVE`), so the app you were typing in keeps focus. No slot/target ghost
//! marks are displayed while dragging; releasing springs the notch cleanly to the top or bottom edge at your chosen
//! horizontal pixel (strictly clamped within monitor bounds so the notch never clips off-screen, with a subtle silent
//! magnet near Left, Centre, and Right), and triggers a brief landing settle + rim flash. Esc springs back.
//!
//! The loop is a bounded timer (code-practices §2): it ticks at ≈ 120 Hz **only while a drag runs** and stops when the
//! button is released or Esc is pressed. It reads the button and Esc with `GetAsyncKeyState` (no hook).

use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use ice_core::appearance::NotchStyle;
use ice_core::engine::Command;
use ice_core::placement::{
    DropArea, NOTCH_CANVAS_H, NOTCH_CANVAS_W, NotchAnchor, NotchPlacement, resolve_drop_target, snap_path,
};
use ice_win::monitors::MonitorInfo;
use ice_win::native_dot::NativeDotHandle;
use ice_win::overlay_win;
use tauri::AppHandle;

use super::SharedRuntime;

/// Which window is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Host {
    WebView,
    Native,
}

/// One drag frame (≈ 120 Hz), for following the cursor and for the release spring.
const FRAME: Duration = Duration::from_millis(8);
const FRAME_MS: u32 = 8;
/// A drag that somehow never ends (a lost button-up) gives up after this long and snaps where it is.
const MAX_DRAG: Duration = Duration::from_secs(120);

fn same_ratio(a: Option<f32>, b: Option<f32>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => (x - y).abs() < 1e-4,
        _ => false,
    }
}

/// Starts a drag of `host` if dragging is on and none is running. Returns at once; the drag runs on its own thread.
pub(super) fn start(app: &AppHandle, state: &Arc<SharedRuntime>, host: Host) {
    let settings = state.engine.snapshot().settings;
    if settings.notch_placement != NotchPlacement::DragSnap || settings.appearance.hibernate {
        return;
    }
    let hwnd = match host {
        Host::WebView => state.notch_hwnd.load(Ordering::Acquire),
        Host::Native => with_native(state, NativeDotHandle::hwnd_raw).unwrap_or(0),
    };
    let (Some(cursor), Some((left, top, right, _))) = (overlay_win::cursor_pos(), overlay_win::window_rect(hwnd))
    else {
        return;
    };
    if hwnd == 0 || state.dragging.swap(true, Ordering::AcqRel) {
        return;
    }
    let win_w = (right - left).max(1);
    let anchor = state.placement().anchor;
    let style = settings.appearance.notch_style;
    let grab = if host == Host::WebView {
        let hr = *state.hit_rect.lock().unwrap_or_else(PoisonError::into_inner);
        if hr.height > 70.0 {
            let scale = f64::from(win_w) / f64::from(NOTCH_CANVAS_W);
            let cx = (f64::from(NOTCH_CANVAS_W) * 0.5 * scale).round() as i32;
            let logical_y = match (anchor.is_bottom(), style) {
                (false, NotchStyle::Nook) => 15.0,
                (false, NotchStyle::Floating) => 6.0 + 14.0,
                (true, NotchStyle::Nook) => f64::from(NOTCH_CANVAS_H - 15),
                (true, NotchStyle::Floating) => f64::from(NOTCH_CANVAS_H - 24 - 14),
            };
            let cy = (logical_y * scale).round() as i32;
            (cx, cy)
        } else {
            (cursor.0 - left, cursor.1 - top)
        }
    } else {
        (cursor.0 - left, cursor.1 - top)
    };
    // The card collapses while it's carried (the WebView lifts it into a full capsule); dropping it shows the pill at
    // its new place.
    state.update_and_emit(app, |s| {
        s.expanded = false;
        s.dragging = true;
    });
    let job = Drag {
        app: app.clone(),
        host,
        hwnd,
        win_w,
        grab,
        start: (left, top),
        monitors: state.monitors.lock().unwrap_or_else(PoisonError::into_inner).clone(),
        style,
        anchor,
        pinned: settings.pinned_monitor.clone(),
        saved_anchor: settings.notch_anchor,
        saved_x_ratio: settings.notch_x_ratio,
        reduce_motion: state.snapshot_clone().system_reduce_motion
            || settings.appearance.preset == ice_core::appearance::SpringPreset::Off,
    };
    let st = Arc::clone(state);
    let spawned = std::thread::Builder::new().name("ice-notch-drag".into()).spawn(move || {
        job.run(&st);
        job.released(&st); // idempotent: covers every early return in `run`
        st.dragging.store(false, Ordering::Release);
    });
    if spawned.is_err() {
        state.dragging.store(false, Ordering::Release);
        state.update_and_emit(app, |s| s.dragging = false);
    }
}

struct Drag {
    app: AppHandle,
    host: Host,
    hwnd: isize,
    win_w: i32,
    /// Cursor offset inside the window at the press, kept while it moves.
    grab: (i32, i32),
    /// Where the window was, for Esc.
    start: (i32, i32),
    monitors: Vec<MonitorInfo>,
    style: NotchStyle,
    /// The anchor in use when the drag began.
    anchor: NotchAnchor,
    pinned: Option<String>,
    saved_anchor: NotchAnchor,
    saved_x_ratio: Option<f32>,
    reduce_motion: bool,
}

impl Drag {
    fn run(&self, state: &SharedRuntime) {
        let areas: Vec<DropArea> =
            self.monitors.iter().map(|m| DropArea { area: overlay_win::work_area(m), scale: m.scale_factor }).collect();
        let begun = Instant::now();
        let mut pos = self.start;
        let dropped = loop {
            if overlay_win::escape_down() {
                break None;
            }
            let Some(cursor) = overlay_win::cursor_pos() else { break None };
            let released = !overlay_win::primary_button_down() || begun.elapsed() > MAX_DRAG;
            if released {
                let drop_point = (pos.0 + self.win_w / 2, cursor.1);
                break resolve_drop_target(&areas, self.style, drop_point);
            }
            pos = (cursor.0 - self.grab.0, cursor.1 - self.grab.1);
            self.move_host(state, pos);
            std::thread::sleep(FRAME);
        };
        // Let go: the WebView starts settling its shape back onto the edge while the window springs into place.
        self.released(state);

        let Some(target_drop) = dropped else {
            // Esc (or no cursor): back where it was, nothing saved.
            self.spring(state, pos, self.start);
            self.end_native(state, None);
            return;
        };
        let Some(monitor) = self.monitors.get(target_drop.monitor) else { return };
        let target = match self.host {
            Host::WebView => overlay_win::canvas_origin(monitor, target_drop.anchor, target_drop.x_ratio, self.style),
            Host::Native => {
                with_native(state, |d| d.origin_for(monitor, target_drop.anchor, target_drop.x_ratio)).unwrap_or(pos)
            }
        };
        // The WebView lays its pill out from the anchored edge, so a top↔bottom change flips the layout: move it in
        // one step and let the new layout land with it, instead of springing a canvas that is still laid out for the
        // old edge.
        if self.host == Host::WebView && target_drop.anchor.is_bottom() != self.anchor.is_bottom() {
            let _ = overlay_win::move_window_async(self.hwnd, target.0, target.1);
        } else {
            self.spring(state, pos, target);
        }
        self.end_native(state, Some((monitor, target_drop.anchor, target_drop.x_ratio)));
        match self.host {
            Host::WebView => {
                state.update_and_emit(&self.app, |s| {
                    s.anchor = target_drop.anchor;
                    s.placed_seq = s.placed_seq.wrapping_add(1);
                });
            }
            Host::Native => {
                if !self.reduce_motion {
                    with_native(state, NativeDotHandle::flash_placed);
                }
            }
        }

        let repin = self.pinned.as_deref().is_none_or(|p| !p.eq_ignore_ascii_case(&monitor.display_path));
        if repin {
            state.engine.send(Command::SetPinnedMonitor { display_path: monitor.display_path.clone() });
        }
        if target_drop.anchor != self.saved_anchor || !same_ratio(target_drop.x_ratio, self.saved_x_ratio) {
            state.engine.send(Command::SetNotchPosition { anchor: target_drop.anchor, x_ratio: target_drop.x_ratio });
        }
        tracing::debug!(
            anchor = ?target_drop.anchor,
            x_ratio = ?target_drop.x_ratio,
            monitor = target_drop.monitor,
            repin,
            "Notch dropped"
        );
    }

    /// Ends the carried state for the WebView (`NotchSnapshot.dragging`). Emits only on a change, so calling it twice
    /// is free.
    fn released(&self, state: &SharedRuntime) {
        state.update_and_emit(&self.app, |s| s.dragging = false);
    }

    fn move_host(&self, state: &SharedRuntime, pos: (i32, i32)) {
        match self.host {
            Host::WebView => {
                let _ = overlay_win::move_window_async(self.hwnd, pos.0, pos.1);
            }
            Host::Native => {
                with_native(state, |dot| dot.drag_move(pos));
            }
        }
    }

    /// The release: a critically damped spring from `from` to `to` (instant with reduce motion).
    fn spring(&self, state: &SharedRuntime, from: (i32, i32), to: (i32, i32)) {
        for step in snap_path(from, to, FRAME_MS, self.reduce_motion) {
            self.move_host(state, step);
            std::thread::sleep(FRAME);
        }
    }

    /// Hands the native pill back to its position: the new one first (so it doesn't jump back), then ends the override.
    fn end_native(&self, state: &SharedRuntime, placed: Option<(&MonitorInfo, NotchAnchor, Option<f32>)>) {
        if self.host != Host::Native {
            return;
        }
        with_native(state, |dot| {
            if let Some((monitor, anchor, x_ratio)) = placed {
                dot.set_position(anchor, x_ratio);
                dot.move_to_monitor(monitor.clone());
            }
            dot.drag_end();
        });
    }
}

/// Runs `f` on the native pill, if there is one.
fn with_native<R>(state: &SharedRuntime, f: impl FnOnce(&NativeDotHandle) -> R) -> Option<R> {
    state.native_dot.lock().unwrap_or_else(PoisonError::into_inner).as_ref().map(f)
}
