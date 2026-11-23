//! Phase 0.5 / Phase 2 fullscreen coverage probe and production watcher for the notch's display.

use core::ffi::c_void;
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WAIT_FAILED, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use windows::Win32::UI::Shell::{QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState};
use windows::Win32::UI::WindowsAndMessaging::{
    CHILDID_SELF, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, EVENT_OBJECT_LOCATIONCHANGE,
    EVENT_OBJECT_STATECHANGE, EVENT_SYSTEM_FOREGROUND, GWL_STYLE, GetDesktopWindow, GetForegroundWindow, GetMessageW,
    GetShellWindow, GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindowVisible, IsZoomed, MONITORINFOF_PRIMARY, MSG,
    MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, OBJID_WINDOW, PM_REMOVE, PeekMessageW, PostMessageW,
    PostQuitMessage, PostThreadMessageW, QS_ALLINPUT, RegisterClassExW, TranslateMessage, WINEVENT_OUTOFCONTEXT,
    WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE, WM_NULL, WM_QUIT, WNDCLASSEXW, WS_CAPTION, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{BOOL, Error, Result, w};

use crate::monitors::MonitorInfo;

#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    /// Diagnostic label for this session. Persistent monitor identity uses `display_path`.
    pub name: String,
    pub bounds: RECT,
    pub primary: bool,
    pub covered: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub foreground_frame: Option<RECT>,
    pub displays: Vec<Display>,
    /// A session-wide hint with no display identity.
    pub shell_reports_d3d_fullscreen: Option<bool>,
}

/// Returns `true` when `monitor` matches an entry in `snapshot` that is currently covered fullscreen.
#[must_use]
pub fn is_monitor_covered(snapshot: &Snapshot, monitor: &MonitorInfo) -> bool {
    let target_bounds = (
        monitor.left,
        monitor.top,
        monitor.left.saturating_add(monitor.width),
        monitor.top.saturating_add(monitor.height),
    );
    snapshot
        .displays
        .iter()
        .find(|d| {
            d.name.eq_ignore_ascii_case(&monitor.id)
                || (d.bounds.left, d.bounds.top, d.bounds.right, d.bounds.bottom) == target_bounds
        })
        .is_some_and(|d| d.covered)
}

/// Samples visible foreground bounds and each display in physical pixels.
/// Returns `None` when the foreground is still changing, so callers can retain the last state.
///
/// # Errors
/// Fails if setting per-monitor DPI awareness context or enumerating displays returns an OS error.
pub fn snapshot() -> Result<Option<Snapshot>> {
    // SAFETY: this changes only the calling thread's DPI context. The guard restores it.
    let previous = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    if previous.is_invalid() {
        return Err(Error::from_thread());
    }
    let _dpi_context = DpiContext(previous);

    let mut displays: Vec<Display> = Vec::new();
    // SAFETY: EnumDisplayMonitors calls collect_display synchronously. The vector remains alive
    // and is accessed only by the callback while enumeration is in progress.
    unsafe { EnumDisplayMonitors(None, None, Some(collect_display), LPARAM(&raw mut displays as isize)) }.ok()?;
    displays.sort_by(|left, right| left.name.cmp(&right.name));

    let Ok(foreground) = foreground_frame().or_else(|()| foreground_frame()) else {
        return Ok(None);
    };
    for display in &mut displays {
        display.covered = foreground.is_some_and(|(frame, maximized_with_caption)| {
            covers_display(frame, display.bounds, maximized_with_caption)
        });
    }

    // SAFETY: the API returns a value and takes no caller-supplied pointers.
    let shell_reports_d3d_fullscreen =
        unsafe { SHQueryUserNotificationState() }.ok().map(|state| state == QUNS_RUNNING_D3D_FULL_SCREEN);

    Ok(Some(Snapshot { foreground_frame: foreground.map(|(frame, _)| frame), displays, shell_reports_d3d_fullscreen }))
}

struct DpiContext(DPI_AWARENESS_CONTEXT);

impl Drop for DpiContext {
    fn drop(&mut self) {
        // SAFETY: this guard stays on the sampling thread and holds its previous valid context.
        unsafe { SetThreadDpiAwarenessContext(self.0) };
    }
}

unsafe extern "system" fn collect_display(monitor: HMONITOR, _dc: HDC, _bounds: *mut RECT, data: LPARAM) -> BOOL {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: the first member of MONITORINFOEXW is MONITORINFO. cbSize describes the whole buffer.
    if unsafe { GetMonitorInfoW(monitor, (&raw mut info).cast()) }.as_bool() {
        let end = info.szDevice.iter().position(|&ch| ch == 0).unwrap_or(info.szDevice.len());
        // SAFETY: snapshot supplied this aligned, exclusive pointer for synchronous enumeration.
        let displays = unsafe { &mut *(data.0 as *mut Vec<Display>) };
        displays.push(Display {
            name: String::from_utf16_lossy(&info.szDevice[..end]),
            bounds: info.monitorInfo.rcMonitor,
            primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
            covered: false,
        });
    }
    true.into()
}

fn foreground_frame() -> std::result::Result<Option<(RECT, bool)>, ()> {
    // SAFETY: these read-only HWND queries use handles from Win32 and no borrowed pointers.
    let window = unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return Err(());
        }
        if window == GetDesktopWindow()
            || window == GetShellWindow()
            || IsIconic(window).as_bool()
            || !IsWindowVisible(window).as_bool()
        {
            return if GetForegroundWindow() == window { Ok(None) } else { Err(()) };
        }
        window
    };

    let mut bounds = RECT::default();
    // SAFETY: DWMWA_EXTENDED_FRAME_BOUNDS writes a RECT into this correctly sized buffer.
    let frame = unsafe {
        DwmGetWindowAttribute(window, DWMWA_EXTENDED_FRAME_BOUNDS, (&raw mut bounds).cast(), size_of::<RECT>() as u32)
    };
    if frame.is_err() {
        // SAFETY: bounds is a valid writable RECT. The calling thread is per-monitor DPI-aware.
        unsafe { GetWindowRect(window, &raw mut bounds) }.map_err(|_| ())?;
    }

    // SAFETY: these read the style/placement of a Win32 handle. Check that focus did not move
    // during the sample before returning it.
    unsafe {
        let caption = WS_CAPTION.0 as isize;
        let maximized_with_caption =
            IsZoomed(window).as_bool() && GetWindowLongPtrW(window, GWL_STYLE) & caption == caption;
        if GetForegroundWindow() != window {
            return Err(());
        }
        Ok(Some((bounds, maximized_with_caption)))
    }
}

fn covers_display(frame: RECT, display: RECT, maximized_with_caption: bool) -> bool {
    !maximized_with_caption
        && frame.left < frame.right
        && frame.top < frame.bottom
        && frame.left <= display.left
        && frame.top <= display.top
        && frame.right >= display.right
        && frame.bottom >= display.bottom
}

thread_local! {
    static FOREGROUND_CHANGED: Cell<bool> = const { Cell::new(false) };
    static DISPLAY_CHANGED: Cell<bool> = const { Cell::new(false) };
    static WINDOW_DESTROYED: Cell<bool> = const { Cell::new(false) };
}

struct WinEventSubscription(HWINEVENTHOOK);

impl WinEventSubscription {
    fn new(first: u32, last: u32) -> Result<Self> {
        // SAFETY: a static callback is delivered on this thread when it pumps messages.
        // OUTOFCONTEXT does not load this code into other processes.
        let handle = unsafe { SetWinEventHook(first, last, None, Some(on_window_event), 0, 0, WINEVENT_OUTOFCONTEXT) };
        if handle.is_invalid() { Err(Error::from_thread()) } else { Ok(Self(handle)) }
    }
}

impl Drop for WinEventSubscription {
    fn drop(&mut self) {
        // SAFETY: this subscription is scoped to the same thread that registered the callback.
        let _ = unsafe { UnhookWinEvent(self.0) };
    }
}

unsafe extern "system" fn on_window_event(
    _subscription: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    object: i32,
    child: i32,
    _thread: u32,
    _timestamp: u32,
) {
    if event == EVENT_SYSTEM_FOREGROUND
        || (!window.is_invalid() && object == OBJID_WINDOW.0 && child == CHILDID_SELF as i32 && {
            // SAFETY: this read-only query accepts no pointers.
            window == unsafe { GetForegroundWindow() }
        })
    {
        // Out-of-context WinEvents can be delivered inside GetMessageW without making it return.
        // Wake the sampling loop once per burst so F17 also stops on a quiet desktop.
        if !FOREGROUND_CHANGED.replace(true) {
            // SAFETY: this callback runs on the watcher's message-loop thread. WM_NULL carries no data.
            unsafe {
                let _ = PostThreadMessageW(GetCurrentThreadId(), WM_NULL, WPARAM(0), LPARAM(0));
            }
        }
    }
}

/// Event pushed by [`FullscreenWatcher`].
#[derive(Debug, Clone, PartialEq)]
pub enum FullscreenEvent {
    /// Foreground/window bounds settled and may have entered or left fullscreen on a display.
    Changed(Snapshot),
    /// Windows broadcast `WM_DISPLAYCHANGE` (monitor attach/detach, resolution, or DPI scale change).
    DisplaysChanged(Option<Snapshot>),
}

/// Event-driven fullscreen + `WM_DISPLAYCHANGE` monitor running on a dedicated hidden-window thread.
pub struct FullscreenWatcher {
    closing: Arc<AtomicBool>,
    hwnd_raw: Arc<AtomicIsize>,
    join: Option<JoinHandle<()>>,
}

impl FullscreenWatcher {
    /// Spawns the watcher thread and begins pushing [`FullscreenEvent`] updates to `sink`.
    ///
    /// `sink` runs on the watcher thread and must return quickly (e.g. dispatch to Tauri main thread).
    ///
    /// # Errors
    /// Fails if the thread cannot be spawned or the WinEvent hooks / watcher window fail to initialize.
    pub fn start(sink: impl Fn(FullscreenEvent) + Send + 'static) -> Result<Self> {
        let closing = Arc::new(AtomicBool::new(false));
        let hwnd_raw = Arc::new(AtomicIsize::new(0));
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<isize>>(1);

        let closing_thread = Arc::clone(&closing);
        let hwnd_thread = Arc::clone(&hwnd_raw);
        let join = std::thread::Builder::new()
            .name("ice-fullscreen-watch".into())
            .spawn(move || watcher_thread_main(closing_thread, hwnd_thread, ready_tx, &sink))
            .map_err(|_| Error::from_thread())?;

        match ready_rx.recv() {
            Ok(Ok(_)) => Ok(Self { closing, hwnd_raw, join: Some(join) }),
            Ok(Err(err)) => {
                let _ = join.join();
                Err(err)
            }
            Err(_) => {
                let _ = join.join();
                Err(Error::from_thread())
            }
        }
    }
}

impl Drop for FullscreenWatcher {
    fn drop(&mut self) {
        self.closing.store(true, Ordering::Release);
        let raw = self.hwnd_raw.swap(0, Ordering::AcqRel);
        if raw != 0 {
            let hwnd = HWND(raw as *mut c_void);
            // SAFETY: `hwnd` belongs to `ice-fullscreen-watch` and stays valid until destroyed there.
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

unsafe extern "system" fn watcher_wnd_proc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match umsg {
        WM_DISPLAYCHANGE => {
            DISPLAY_CHANGED.set(true);
            LRESULT(0)
        }
        WM_CLOSE => {
            // SAFETY: destroys only our own hidden watcher window on its owning thread.
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_DESTROY => {
            WINDOW_DESTROYED.set(true);
            // SAFETY: signals `GetMessageW` to return 0 and exit cleanly.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => {
            // SAFETY: standard default message processing.
            unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) }
        }
    }
}

fn create_watcher_window() -> Result<HWND> {
    let class_name = w!("IceFullscreenWatcherClass");
    // SAFETY: `GetModuleHandleW(None)` returns the current executable's HINSTANCE.
    let hinstance = unsafe { GetModuleHandleW(None) }?;
    let wc = WNDCLASSEXW {
        cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(0),
        lpfnWndProc: Some(watcher_wnd_proc),
        hInstance: hinstance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    // SAFETY: registering an immutable static class name; repeated registration is benign.
    unsafe { RegisterClassExW(&raw const wc) };

    // Top-level invisible WS_POPUP window (never shown, not HWND_MESSAGE) so broadcast WM_DISPLAYCHANGE arrives.
    // SAFETY: creates a private 0x0 tool window owned by this thread.
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name,
            w!("IceFullscreenWatcher"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
    }
}

fn watcher_thread_main(
    closing: Arc<AtomicBool>,
    hwnd_raw: Arc<AtomicIsize>,
    ready_tx: mpsc::SyncSender<Result<isize>>,
    sink: &dyn Fn(FullscreenEvent),
) {
    let hwnd = match create_watcher_window() {
        Ok(h) => h,
        Err(err) => {
            let _ = ready_tx.send(Err(err));
            return;
        }
    };
    let _foreground = match WinEventSubscription::new(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND) {
        Ok(sub) => sub,
        Err(err) => {
            // SAFETY: destroys window created above before aborting startup.
            let _ = unsafe { DestroyWindow(hwnd) };
            let _ = ready_tx.send(Err(err));
            return;
        }
    };
    let _bounds = match WinEventSubscription::new(EVENT_OBJECT_STATECHANGE, EVENT_OBJECT_LOCATIONCHANGE) {
        Ok(sub) => sub,
        Err(err) => {
            // SAFETY: destroys window created above before aborting startup.
            let _ = unsafe { DestroyWindow(hwnd) };
            let _ = ready_tx.send(Err(err));
            return;
        }
    };

    FOREGROUND_CHANGED.set(false);
    DISPLAY_CHANGED.set(false);
    WINDOW_DESTROYED.set(false);
    hwnd_raw.store(hwnd.0 as isize, Ordering::Release);
    let _ = ready_tx.send(Ok(hwnd.0 as isize));

    if !closing.load(Ordering::Acquire) {
        match snapshot() {
            Ok(Some(initial)) => sink(FullscreenEvent::Changed(initial)),
            Ok(None) => {}
            Err(err) => tracing::warn!(error = %err, "initial fullscreen snapshot failed"),
        }
    }

    let mut msg = MSG::default();
    loop {
        if closing.load(Ordering::Acquire) {
            break;
        }
        // SAFETY: `GetMessageW` blocks until a message arrives; returns 0 on WM_QUIT and -1 on error.
        let ret = unsafe { GetMessageW(&raw mut msg, None, 0, 0) };
        if ret.0 <= 0 {
            break;
        }
        // SAFETY: dispatches the retrieved message on this thread.
        unsafe {
            let _ = TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);
        }

        // Drain any remaining queued WinEvents/messages so bursts coalesce before sampling.
        let mut quit = false;
        // SAFETY: drains queued thread messages without blocking.
        while unsafe { PeekMessageW(&raw mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            if msg.message == WM_QUIT {
                quit = true;
                break;
            }
            // SAFETY: dispatches queued message on this thread.
            unsafe {
                let _ = TranslateMessage(&raw const msg);
                DispatchMessageW(&raw const msg);
            }
        }
        if quit || closing.load(Ordering::Acquire) {
            break;
        }

        if DISPLAY_CHANGED.replace(false) {
            FOREGROUND_CHANGED.set(false);
            match snapshot() {
                Ok(snap) => sink(FullscreenEvent::DisplaysChanged(snap)),
                Err(err) => {
                    tracing::warn!(error = %err, "fullscreen snapshot failed on display change");
                    sink(FullscreenEvent::DisplaysChanged(None));
                }
            }
        } else if FOREGROUND_CHANGED.replace(false) {
            match snapshot() {
                Ok(Some(current)) => sink(FullscreenEvent::Changed(current)),
                Ok(None) => {}
                Err(err) => tracing::warn!(error = %err, "fullscreen snapshot failed"),
            }
        }
    }

    hwnd_raw.store(0, Ordering::Release);
    if !WINDOW_DESTROYED.get() {
        // SAFETY: window belongs to this thread and has not been destroyed yet.
        let _ = unsafe { DestroyWindow(hwnd) };
    }
}

/// Dedicated message loop for the spike. Run it on a thread with no other message-loop owner.
pub fn watch_for(duration: Duration, mut report: impl FnMut(Snapshot)) -> Result<()> {
    let _foreground = WinEventSubscription::new(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND)?;
    let _bounds = WinEventSubscription::new(EVENT_OBJECT_STATECHANGE, EVENT_OBJECT_LOCATIONCHANGE)?;
    FOREGROUND_CHANGED.set(false);
    if let Some(initial) = snapshot()? {
        report(initial);
    }

    let started = Instant::now();
    while let Some(remaining) = duration.checked_sub(started.elapsed()) {
        let timeout = remaining.as_millis().clamp(1, u128::from(u32::MAX - 1)) as u32;
        // SAFETY: this waits for activity in the current thread's queue. No handles are borrowed.
        if unsafe { MsgWaitForMultipleObjectsEx(None, timeout, QS_ALLINPUT, MWMO_INPUTAVAILABLE) } == WAIT_FAILED {
            return Err(Error::from_thread());
        }

        let mut message = MSG::default();
        // SAFETY: message is a writable MSG. The retrieved message is dispatched on the same thread.
        while unsafe { PeekMessageW(&raw mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            if message.message == WM_QUIT {
                return Ok(());
            }
            // SAFETY: PeekMessageW initialised this MSG on the current thread.
            unsafe { DispatchMessageW(&raw const message) };
        }

        if FOREGROUND_CHANGED.replace(false)
            && let Some(current) = snapshot()?
        {
            report(current);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: RECT = RECT { left: -1920, top: 0, right: 0, bottom: 1080 };
    const PRIMARY: RECT = RECT { left: 0, top: -120, right: 2560, bottom: 1320 };

    #[test]
    fn coverage_uses_the_target_monitor_even_with_negative_coordinates() {
        assert!(covers_display(LEFT, LEFT, false));
        assert!(!covers_display(LEFT, PRIMARY, false));
        assert!(covers_display(PRIMARY, PRIMARY, false));
        assert!(!covers_display(PRIMARY, LEFT, false));
    }

    #[test]
    fn maximized_caption_and_partial_coverage_do_not_start_gaming_mode() {
        let work_area = RECT { bottom: LEFT.bottom - 40, ..LEFT };
        assert!(!covers_display(work_area, LEFT, false));
        assert!(!covers_display(LEFT, LEFT, true));
    }

    #[test]
    fn overscan_that_contains_the_monitor_counts_but_an_empty_frame_does_not() {
        let overscan = RECT { left: LEFT.left - 8, right: LEFT.right + 8, ..LEFT };
        assert!(covers_display(overscan, LEFT, false));
        assert!(!covers_display(RECT::default(), LEFT, false));
    }

    #[test]
    fn is_monitor_covered_matches_by_name_or_bounds() {
        let snap = Snapshot {
            foreground_frame: Some(LEFT),
            displays: vec![
                Display { name: "\\\\.\\DISPLAY1".into(), bounds: PRIMARY, primary: true, covered: false },
                Display { name: "\\\\.\\DISPLAY2".into(), bounds: LEFT, primary: false, covered: true },
            ],
            shell_reports_d3d_fullscreen: Some(false),
        };

        let mon_by_name = MonitorInfo {
            id: "\\\\.\\display2".into(),
            display_path: "\\\\?\\DISPLAY#TEST#2".into(),
            friendly_name: "Left".into(),
            is_primary: false,
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
            refresh_hz: 75,
        };
        assert!(is_monitor_covered(&snap, &mon_by_name));

        let mon_by_bounds = MonitorInfo { id: "\\\\.\\DISPLAY9".into(), left: -1920, ..mon_by_name.clone() };
        assert!(is_monitor_covered(&snap, &mon_by_bounds));

        let mon_primary = MonitorInfo {
            id: "\\\\.\\DISPLAY1".into(),
            is_primary: true,
            left: 0,
            top: -120,
            width: 2560,
            height: 1440,
            ..mon_by_name
        };
        assert!(!is_monitor_covered(&snap, &mon_primary));
    }

    #[test]
    fn fullscreen_watcher_starts_and_drops_cleanly() {
        let initial_probe = snapshot().expect("fullscreen snapshot API should succeed");
        let (tx, rx) = mpsc::channel();
        let watcher = FullscreenWatcher::start(move |ev| {
            let _ = tx.send(ev);
        })
        .expect("start fullscreen watcher");
        if initial_probe.is_some() {
            let first = rx.recv_timeout(Duration::from_secs(2));
            assert!(first.is_ok(), "expected initial fullscreen snapshot");
        }
        let t0 = Instant::now();
        drop(watcher);
        assert!(t0.elapsed() < Duration::from_secs(1), "watcher drop hung");
    }
}
