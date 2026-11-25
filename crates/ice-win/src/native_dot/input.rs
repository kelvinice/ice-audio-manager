//! Mouse input for the native idle pill (D-19 A1).
//!
//! The pill window takes input as plain window messages, so it costs nothing while the cursor is elsewhere (§2): no hook,
//! no cursor poll. It is click-first: a press paints the pressed state before the handler returns (one frame, so the
//! `Balanced` cold start doesn't feel dead), a release inside opens the notch, and a middle click is a separate input.
//! Hover-to-open is off unless the shell turns it on with a delay from config. The wheel changes the volume (Shift: switch
//! device), like the WebView notch. Windows delivers it to this non-activating window only while "Scroll inactive
//! windows when hovering" is on (the default); with it off the wheel goes to the focused app and the pill does nothing.

use std::sync::Arc;
use std::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, KillTimer, MA_NOACTIVATE, SetTimer, SetWindowLongPtrW,
    WM_CAPTURECHANGED, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_TIMER,
};

use super::{PillMode, SharedState, paint_pill};

/// `WM_MOUSELEAVE` lives in the `Win32_UI_Controls` feature; one message number isn't worth enabling it.
const WM_MOUSELEAVE: u32 = 0x02A3;

/// One-shot timer that fires `HoverOpen` after the configured dwell. Stopped on leave, press and capture loss.
const HOVER_TIMER_ID: usize = 0x4943_4532;

/// What the pointer is doing to the pill; the rasterizer draws a state for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PillInteraction {
    #[default]
    None,
    Hover,
    Pressed,
}

/// What the shell is told. The pill never acts on the engine itself, so wiring stays in `src-tauri`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillInput {
    /// A press and release on the pill. Opens the notch (or the tray quick-switch menu in `Lite`).
    Click,
    /// A middle click. Toggles mute, like the notch.
    MiddleClick,
    /// The cursor dwelt on the idle pill for the configured delay. Only sent when hover-to-open is enabled.
    HoverOpen,
    /// Whole wheel notches over the pill (positive = away from the user, i.e. volume up), with Shift held or not.
    Wheel { notches: i16, shift: bool },
}

/// One wheel notch (`WHEEL_DELTA`). Precision touchpads send fractions of it.
const WHEEL_DELTA: i32 = 120;
/// `MK_SHIFT` in the low word of `WM_MOUSEWHEEL`'s `wparam`.
const MK_SHIFT: usize = 0x0004;

/// Adds a raw wheel delta to `accum` and returns the whole notches it completes, keeping the remainder, so a touchpad's
/// small deltas add up to notches instead of each one stepping the volume.
fn wheel_notches(accum: &mut i32, delta: i32) -> i16 {
    *accum = accum.saturating_add(delta);
    let notches = *accum / WHEEL_DELTA;
    *accum -= notches * WHEEL_DELTA;
    i16::try_from(notches).unwrap_or(if notches > 0 { i16::MAX } else { i16::MIN })
}

pub type InputHandler = Arc<dyn Fn(PillInput) + Send + Sync>;

/// Window-thread state that isn't part of what the pill draws.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct InputState {
    /// `None` is click-first: hovering never opens anything.
    pub hover_open_ms: Option<u32>,
    tracking_leave: bool,
    /// Wheel delta not yet worth a whole notch (precision touchpads).
    wheel_accum: i32,
}

impl InputState {
    pub(super) fn set_hover_open(&mut self, delay: Option<Duration>) {
        self.hover_open_ms = delay.and_then(|d| u32::try_from(d.as_millis()).ok()).map(|ms| ms.max(1));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ev {
    Move,
    Leave,
    Down,
    Up { inside: bool },
    MiddleUp { inside: bool },
    CaptureLost,
    HoverTimer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Timer {
    Keep,
    Arm,
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Step {
    next: PillInteraction,
    emit: Option<PillInput>,
    timer: Timer,
}

/// The whole click/hover policy as a pure function, so it is tested without a window.
/// `hover_armed` is true only while hover-to-open is enabled and the pill shows its idle content.
fn step(cur: PillInteraction, hover_armed: bool, ev: Ev) -> Step {
    use PillInteraction::{Hover, None as Idle, Pressed};
    let keep = Step { next: cur, emit: None, timer: Timer::Keep };
    match (ev, cur) {
        (Ev::Move, Idle) => Step { next: Hover, emit: None, timer: if hover_armed { Timer::Arm } else { Timer::Keep } },
        (Ev::Leave, Pressed) => keep,
        (Ev::Leave, _) => Step { next: Idle, emit: None, timer: Timer::Kill },
        (Ev::Down, _) => Step { next: Pressed, emit: None, timer: Timer::Kill },
        (Ev::Up { inside }, Pressed) => Step {
            next: if inside { Hover } else { Idle },
            emit: inside.then_some(PillInput::Click),
            timer: Timer::Keep,
        },
        (Ev::MiddleUp { inside: true }, _) => Step { emit: Some(PillInput::MiddleClick), ..keep },
        (Ev::CaptureLost, Pressed) => Step { next: Idle, emit: None, timer: Timer::Kill },
        (Ev::HoverTimer, Hover) if hover_armed => {
            Step { next: Hover, emit: Some(PillInput::HoverOpen), timer: Timer::Kill }
        }
        (Ev::HoverTimer, _) => Step { timer: Timer::Kill, ..keep },
        _ => keep,
    }
}

/// Points `hwnd` at the shared state so the window procedure can reach it.
pub(super) fn attach(hwnd: HWND, shared: &Arc<SharedState>) {
    // SAFETY: the dot thread owns `hwnd` and holds `shared` until its message loop ends, which is after the window is
    // destroyed; `detach` clears the pointer on `WM_DESTROY`.
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, Arc::as_ptr(shared) as isize) };
}

pub(super) fn detach(hwnd: HWND) {
    // SAFETY: plain write to the window's own user-data slot.
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
}

/// Handles the pill's input messages; `None` means the caller should use the default procedure.
pub(super) fn dispatch(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if msg == WM_MOUSEACTIVATE {
        // D-15: the pill never takes focus, so clicking it can't pull the foreground app's keyboard away (QA F4).
        return Some(LRESULT(MA_NOACTIVATE as isize));
    }
    // SAFETY: set by `attach` to the `SharedState` the dot thread keeps alive while this window exists.
    let shared = unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const SharedState).as_ref() }?;
    let ev = match msg {
        WM_MOUSEMOVE => Ev::Move,
        WM_MOUSELEAVE => {
            shared.lock().input.tracking_leave = false;
            Ev::Leave
        }
        WM_LBUTTONDOWN => Ev::Down,
        WM_LBUTTONUP => Ev::Up { inside: point_inside(hwnd, lparam) },
        WM_MBUTTONDOWN => return Some(LRESULT(0)),
        WM_MBUTTONUP => Ev::MiddleUp { inside: point_inside(hwnd, lparam) },
        WM_CAPTURECHANGED => Ev::CaptureLost,
        WM_TIMER if wparam.0 == HOVER_TIMER_ID => Ev::HoverTimer,
        WM_MOUSEWHEEL => {
            // High word: the signed delta; low word: key state. Not part of the click/hover state machine.
            let delta = i32::from(((wparam.0 >> 16) & 0xffff) as u16 as i16);
            let shift = wparam.0 & MK_SHIFT != 0;
            let notches = wheel_notches(&mut shared.lock().input.wheel_accum, delta);
            if notches != 0 {
                emit(shared, PillInput::Wheel { notches, shift });
            }
            return Some(LRESULT(0));
        }
        _ => return None,
    };
    apply(hwnd, shared, ev);
    Some(LRESULT(0))
}

/// Cancels any press, hover and timer. Called when the pill hides, because a hidden window gets no leave message.
pub(super) fn reset(hwnd: HWND, shared: &SharedState) {
    {
        let mut rt = shared.lock();
        rt.pill.interaction = PillInteraction::None;
        rt.input.tracking_leave = false;
    }
    // SAFETY: both calls act on this thread's own window and are harmless when nothing is armed or captured.
    unsafe {
        let _ = KillTimer(Some(hwnd), HOVER_TIMER_ID);
        let _ = ReleaseCapture();
    }
}

fn apply(hwnd: HWND, shared: &SharedState, ev: Ev) {
    let (step, repaint, start_tracking, hover_ms) = {
        let mut rt = shared.lock();
        let hover_armed = rt.input.hover_open_ms.is_some() && matches!(rt.pill.mode, PillMode::Idle);
        let step = step(rt.pill.interaction, hover_armed, ev);
        let changed = step.next != rt.pill.interaction;
        rt.pill.interaction = step.next;
        let start_tracking = ev == Ev::Move && !rt.input.tracking_leave;
        rt.input.tracking_leave |= start_tracking;
        let repaint = changed.then(|| (rt.pill.clone(), rt.monitor.clone()));
        (step, repaint, start_tracking, rt.input.hover_open_ms)
    };

    // SAFETY: Win32 calls on this thread's own window; no lock is held, so the messages they trigger can re-enter.
    unsafe {
        match step.timer {
            Timer::Arm => {
                let _ = SetTimer(Some(hwnd), HOVER_TIMER_ID, hover_ms.unwrap_or(1), None);
            }
            Timer::Kill => {
                let _ = KillTimer(Some(hwnd), HOVER_TIMER_ID);
            }
            Timer::Keep => {}
        }
        match ev {
            Ev::Down => {
                SetCapture(hwnd);
            }
            Ev::Up { .. } => {
                let _ = ReleaseCapture();
            }
            _ => {}
        }
        if start_tracking {
            let mut track = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            let _ = TrackMouseEvent(&raw mut track);
        }
    }
    // Painted here, not posted, so the pressed state is on screen before the click is even delivered.
    if let Some((pill, monitor)) = repaint {
        let _ = paint_pill(hwnd, &monitor, &pill);
    }
    if let Some(input) = step.emit {
        emit(shared, input);
    }
}

/// Hands `input` to the shell's handler (no lock held while it runs).
fn emit(shared: &SharedState, input: PillInput) {
    let handler = shared.input_handler.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    if let Some(handler) = handler {
        handler(input);
    }
}

/// Whether a client-area point (the low/high words of `lparam`, signed) lies inside the window.
fn point_inside(hwnd: HWND, lparam: LPARAM) -> bool {
    let x = i32::from((lparam.0 & 0xffff) as u16 as i16);
    let y = i32::from(((lparam.0 >> 16) & 0xffff) as u16 as i16);
    let mut rc = RECT::default();
    // SAFETY: `rc` is a valid stack `RECT` and `hwnd` is this thread's window.
    unsafe { GetClientRect(hwnd, &raw mut rc) }.is_ok() && x >= 0 && y >= 0 && x < rc.right && y < rc.bottom
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use windows::Win32::UI::WindowsAndMessaging::SendMessageW;

    use super::PillInteraction::{Hover, None as Idle, Pressed};
    use super::*;
    use crate::monitors::MonitorInfo;
    use crate::native_dot::NativeDotHandle;

    #[test]
    fn click_needs_press_and_release_inside() {
        let down = step(Idle, false, Ev::Down);
        assert_eq!((down.next, down.emit), (Pressed, None));
        let click = step(Pressed, false, Ev::Up { inside: true });
        assert_eq!((click.next, click.emit), (Hover, Some(PillInput::Click)));
        // Dragging off before release cancels the click.
        let cancelled = step(Pressed, false, Ev::Up { inside: false });
        assert_eq!((cancelled.next, cancelled.emit), (Idle, None));
        // A release with no press (the press landed on another window) does nothing.
        assert_eq!(step(Idle, false, Ev::Up { inside: true }).emit, None);
        // Losing capture (Alt+Tab, another window grabbing the mouse) cancels the press.
        assert_eq!(step(Pressed, false, Ev::CaptureLost).next, Idle);
    }

    #[test]
    fn hover_only_opens_when_enabled() {
        assert_eq!(step(Idle, false, Ev::Move).timer, Timer::Keep, "click-first: no timer by default");
        assert_eq!(step(Idle, true, Ev::Move).timer, Timer::Arm);
        let fired = step(Hover, true, Ev::HoverTimer);
        assert_eq!(fired.emit, Some(PillInput::HoverOpen));
        assert_eq!(step(Hover, false, Ev::HoverTimer).emit, None);
        // Leaving or pressing before the dwell ends stops it.
        assert_eq!(step(Hover, true, Ev::Leave).timer, Timer::Kill);
        assert_eq!(step(Hover, true, Ev::Down).timer, Timer::Kill);
        // A late timer after the cursor left emits nothing.
        assert_eq!(step(Idle, true, Ev::HoverTimer).emit, None);
    }

    #[test]
    fn middle_click_only_counts_inside() {
        assert_eq!(step(Idle, false, Ev::MiddleUp { inside: true }).emit, Some(PillInput::MiddleClick));
        assert_eq!(step(Idle, false, Ev::MiddleUp { inside: false }).emit, None);
    }

    #[test]
    fn leave_keeps_a_captured_press() {
        assert_eq!(step(Pressed, false, Ev::Leave).next, Pressed);
        assert_eq!(step(Hover, false, Ev::Leave).next, Idle);
    }

    #[test]
    fn wheel_deltas_add_up_to_whole_notches() {
        let mut accum = 0;
        assert_eq!(wheel_notches(&mut accum, 120), 1, "a mouse notch");
        assert_eq!(wheel_notches(&mut accum, -240), -2, "two notches down");
        // A precision touchpad: small deltas only step once they make a notch, and the rest carries over.
        assert_eq!(wheel_notches(&mut accum, 40), 0);
        assert_eq!(wheel_notches(&mut accum, 40), 0);
        assert_eq!(wheel_notches(&mut accum, 50), 1);
        assert_eq!(accum, 10, "remainder kept");
    }

    #[test]
    fn window_wheel_reports_notches_and_shift() {
        let dot = NativeDotHandle::spawn(monitor()).expect("spawn native dot window");
        dot.show_idle();
        let (tx, rx) = mpsc::channel();
        dot.set_input_handler(Arc::new(move |input| {
            let _ = tx.send(input);
        }));
        let hwnd = HWND(dot.shared.hwnd_raw.load(std::sync::atomic::Ordering::Acquire) as *mut core::ffi::c_void);
        let wheel = |delta: i16, keys: usize| {
            let wparam = WPARAM(((delta as u16 as usize) << 16) | keys);
            // SAFETY: `hwnd` is the live dot window; `SendMessageW` runs the handler on the dot thread.
            unsafe { SendMessageW(hwnd, WM_MOUSEWHEEL, Some(wparam), Some(LPARAM(0))) };
        };
        wheel(120, 0);
        assert_eq!(rx.recv_timeout(Duration::from_secs(1)), Ok(PillInput::Wheel { notches: 1, shift: false }));
        wheel(-120, MK_SHIFT);
        assert_eq!(rx.recv_timeout(Duration::from_secs(1)), Ok(PillInput::Wheel { notches: -1, shift: true }));
        wheel(60, 0);
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err(), "half a notch does nothing yet");
    }

    fn monitor() -> MonitorInfo {
        MonitorInfo {
            id: "\\\\.\\DISPLAY1".into(),
            display_path: "\\\\?\\DISPLAY#TEST0001#1".into(),
            friendly_name: "Primary Test Display".into(),
            is_primary: true,
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
            work_left: 0,
            work_top: 0,
            work_width: 1920,
            work_height: 1032,
            dpi_x: 96,
            scale_factor: 1.0,
            refresh_hz: 144,
        }
    }

    fn send(dot: &NativeDotHandle, msg: u32, x: i32, y: i32) {
        let hwnd = HWND(dot.shared.hwnd_raw.load(std::sync::atomic::Ordering::Acquire) as *mut core::ffi::c_void);
        let lparam = LPARAM(((y & 0xffff) << 16) as isize | (x & 0xffff) as isize);
        // SAFETY: `hwnd` is the live dot window; `SendMessageW` runs the handler on the dot thread and returns after it.
        unsafe { SendMessageW(hwnd, msg, Some(WPARAM(0)), Some(lparam)) };
    }

    #[test]
    fn window_press_is_painted_before_the_click_arrives() {
        let dot = NativeDotHandle::spawn(monitor()).expect("spawn native dot window");
        dot.show_idle();
        let (tx, rx) = mpsc::channel();
        dot.set_input_handler(Arc::new(move |input| {
            let _ = tx.send(input);
        }));

        send(&dot, WM_LBUTTONDOWN, 10, 8);
        assert_eq!(dot.pill_state().interaction, Pressed, "pressed state is set when the message returns");
        assert!(rx.try_recv().is_err(), "a press alone is not a click");

        send(&dot, WM_LBUTTONUP, 10, 8);
        assert_eq!(rx.recv_timeout(Duration::from_secs(1)), Ok(PillInput::Click));
        assert_ne!(dot.pill_state().interaction, Pressed);

        send(&dot, WM_LBUTTONDOWN, 10, 8);
        send(&dot, WM_LBUTTONUP, -40, -40);
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err(), "releasing outside cancels");
        assert_eq!(dot.pill_state().interaction, Idle);

        send(&dot, WM_MBUTTONUP, 10, 8);
        assert_eq!(rx.recv_timeout(Duration::from_secs(1)), Ok(PillInput::MiddleClick));
    }

    #[test]
    #[ignore = "needs an interactive desktop: moves the cursor and clicks the pill (QA F4)"]
    fn real_click_reaches_the_pill_without_taking_focus() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, mouse_event};
        use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

        use crate::monitors::list_monitors;
        use crate::overlay_win::{cursor_pos, foreground_hwnd, window_rect};

        let primary = list_monitors().expect("monitors").into_iter().find(|m| m.is_primary).expect("a primary monitor");
        let dot = NativeDotHandle::spawn(primary).expect("spawn native dot window");
        dot.show_idle();
        let (tx, rx) = mpsc::channel();
        dot.set_input_handler(Arc::new(move |input| {
            let _ = tx.send(input);
        }));
        std::thread::sleep(Duration::from_millis(300));

        let hwnd = dot.shared.hwnd_raw.load(std::sync::atomic::Ordering::Acquire);
        let (l, t, r, b) = window_rect(hwnd).expect("pill rect");
        let before = foreground_hwnd();
        let saved_cursor = cursor_pos();
        // SAFETY: moves the real cursor onto the pill and presses the left button there; the cursor is restored below.
        unsafe {
            SetCursorPos((l + r) / 2, (t + b) / 2).expect("move cursor");
            std::thread::sleep(Duration::from_millis(60));
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            std::thread::sleep(Duration::from_millis(40));
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        }
        let got = rx.recv_timeout(Duration::from_secs(1));
        if let Some((x, y)) = saved_cursor {
            // SAFETY: puts the cursor back where the tester left it.
            let _ = unsafe { SetCursorPos(x, y) };
        }
        assert_eq!(got, Ok(PillInput::Click));
        assert_eq!(foreground_hwnd(), before, "clicking the pill must not change the foreground window");
    }

    #[test]
    fn pressed_state_is_painted_within_one_frame() {
        let dot = NativeDotHandle::spawn(monitor()).expect("spawn native dot window");
        dot.show_idle();
        let mut ms = Vec::new();
        for _ in 0..30 {
            let start = std::time::Instant::now();
            send(&dot, WM_LBUTTONDOWN, 10, 8);
            ms.push(start.elapsed().as_secs_f64() * 1000.0);
            send(&dot, WM_LBUTTONUP, -40, -40);
        }
        ms.sort_by(f64::total_cmp);
        let (p50, p95) = (ms[ms.len() / 2], ms[ms.len() * 95 / 100]);
        eprintln!("pressed paint: p50 {p50:.3} ms, p95 {p95:.3} ms");
        // One frame on the 144 Hz reference monitor is 6.9 ms; the median keeps a busy CI machine from flaking this.
        assert!(p50 < 6.9, "press feedback took {p50:.2} ms");
    }
}
