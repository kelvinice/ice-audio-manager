//! D-20 drag & snap: one drag loop for both hosts (the WebView notch and the native pill).
//!
//! A host reports that a press moved past Windows' drag threshold (the WebView through `start_notch_drag`, the native
//! pill through `PillInput::DragStart`). From then on the shell follows the cursor itself on its own thread: it never
//! uses the system move loop (`HTCAPTION`/`SC_MOVE`), which activated the window in Codex's D-20 measurement, so the
//! app you were typing in keeps focus. Ghost slots show on every monitor; the one nearest the cursor lights up.
//! Releasing the button springs the notch into that slot and saves it (re-pinning the monitor if it changed); Esc
//! springs it back. Reduce motion = instant.
//!
//! The loop is a bounded timer (code-practices §2): it ticks at ≈ 120 Hz **only while a drag runs** and stops when the
//! button is released or Esc is pressed. It reads the button and Esc with `GetAsyncKeyState` (no hook).

use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use ice_core::engine::Command;
use ice_core::placement::{DropArea, NotchAnchor, NotchPlacement, drop_slots, nearest_slot, snap_path};
use ice_win::drag_ghosts::DragGhosts;
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
    let anchor = state.placement().anchor;
    let grab = if host == Host::WebView {
        let hr = *state.hit_rect.lock().unwrap_or_else(PoisonError::into_inner);
        if hr.height > 70.0 {
            let scale = f64::from((right - left).max(1)) / f64::from(ice_core::placement::NOTCH_CANVAS_W);
            let cx = (f64::from(ice_core::placement::NOTCH_CANVAS_W) * 0.5 * scale).round() as i32;
            let cy = if anchor.is_bottom() {
                (f64::from(ice_core::placement::NOTCH_CANVAS_H - 24 - 14) * scale).round() as i32
            } else {
                (f64::from(6 + 14) * scale).round() as i32
            };
            (cx, cy)
        } else {
            (cursor.0 - left, cursor.1 - top)
        }
    } else {
        (cursor.0 - left, cursor.1 - top)
    };
    // The card collapses while it's carried; dropping it shows the pill at its new place.
    state.update_and_emit(app, |s| s.expanded = false);
    let job = Drag {
        host,
        hwnd,
        grab,
        start: (left, top),
        monitors: state.monitors.lock().unwrap_or_else(PoisonError::into_inner).clone(),
        style: settings.appearance.notch_style,
        anchor,
        pinned: settings.pinned_monitor.clone(),
        saved_anchor: settings.notch_anchor,
        reduce_motion: state.snapshot_clone().system_reduce_motion
            || settings.appearance.preset == ice_core::appearance::SpringPreset::Off,
    };
    let st = Arc::clone(state);
    let spawned = std::thread::Builder::new().name("ice-notch-drag".into()).spawn(move || {
        job.run(&st);
        st.dragging.store(false, Ordering::Release);
    });
    if spawned.is_err() {
        state.dragging.store(false, Ordering::Release);
    }
}

struct Drag {
    host: Host,
    hwnd: isize,
    /// Cursor offset inside the window at the press, kept while it moves.
    grab: (i32, i32),
    /// Where the window was, for Esc.
    start: (i32, i32),
    monitors: Vec<MonitorInfo>,
    style: ice_core::appearance::NotchStyle,
    /// The anchor in use when the drag began.
    anchor: NotchAnchor,
    pinned: Option<String>,
    saved_anchor: NotchAnchor,
    reduce_motion: bool,
}

impl Drag {
    fn run(&self, state: &SharedRuntime) {
        let areas: Vec<DropArea> =
            self.monitors.iter().map(|m| DropArea { area: overlay_win::work_area(m), scale: m.scale_factor }).collect();
        let slots = drop_slots(&areas, self.style);
        let mut ghosts = DragGhosts::show(&slots);
        let begun = Instant::now();
        let mut pos = self.start;
        let dropped = loop {
            DragGhosts::pump();
            if overlay_win::escape_down() {
                break None;
            }
            let Some(cursor) = overlay_win::cursor_pos() else { break None };
            let released = !overlay_win::primary_button_down() || begun.elapsed() > MAX_DRAG;
            let nearest = nearest_slot(&slots, cursor);
            if released {
                break nearest;
            }
            pos = (cursor.0 - self.grab.0, cursor.1 - self.grab.1);
            self.move_host(state, pos);
            if let Some(slot) = nearest {
                ghosts.highlight(&slot);
            }
            std::thread::sleep(FRAME);
        };
        drop(ghosts);

        let Some(slot) = dropped else {
            // Esc (or no cursor): back where it was, nothing saved.
            self.spring(state, pos, self.start);
            self.end_native(state, None);
            return;
        };
        let Some(monitor) = self.monitors.get(slot.monitor) else { return };
        let target = match self.host {
            Host::WebView => overlay_win::canvas_origin(monitor, slot.anchor),
            Host::Native => with_native(state, |d| d.origin_for(monitor, slot.anchor)).unwrap_or(pos),
        };
        // The WebView lays its pill out from the anchored edge, so a top↔bottom change flips the layout: move it in
        // one step and let the new layout land with it, instead of springing a canvas that is still laid out for the
        // old edge.
        if self.host == Host::WebView && slot.anchor.is_bottom() != self.anchor.is_bottom() {
            let _ = overlay_win::move_window_async(self.hwnd, target.0, target.1);
        } else {
            self.spring(state, pos, target);
        }
        self.end_native(state, Some((monitor, slot.anchor)));
        let repin = self.pinned.as_deref().is_none_or(|p| !p.eq_ignore_ascii_case(&monitor.display_path));
        if repin {
            state.engine.send(Command::SetPinnedMonitor { display_path: monitor.display_path.clone() });
        }
        if slot.anchor != self.saved_anchor {
            state.engine.send(Command::SetNotchAnchor { anchor: slot.anchor });
        }
        tracing::debug!(anchor = ?slot.anchor, monitor = slot.monitor, repin, "Notch dropped");
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

    /// Hands the native pill back to its anchor: the new one first (so it doesn't jump back), then ends the override.
    fn end_native(&self, state: &SharedRuntime, placed: Option<(&MonitorInfo, NotchAnchor)>) {
        if self.host != Host::Native {
            return;
        }
        with_native(state, |dot| {
            if let Some((monitor, anchor)) = placed {
                dot.set_anchor(anchor);
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
