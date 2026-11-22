//! Phase 0.5 probe for fullscreen coverage on the notch's display.

use std::cell::Cell;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, RECT, WAIT_FAILED};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use windows::Win32::UI::Shell::{QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState};
use windows::Win32::UI::WindowsAndMessaging::{
    CHILDID_SELF, DispatchMessageW, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_STATECHANGE, EVENT_SYSTEM_FOREGROUND,
    GWL_STYLE, GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetWindowLongPtrW, GetWindowRect, IsIconic,
    IsWindowVisible, IsZoomed, MONITORINFOF_PRIMARY, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx,
    OBJID_WINDOW, PM_REMOVE, PeekMessageW, QS_ALLINPUT, WINEVENT_OUTOFCONTEXT, WM_QUIT, WS_CAPTION,
};
use windows::core::{BOOL, Error, Result};

#[derive(Debug, PartialEq)]
pub struct Display {
    /// Diagnostic label for this session. The app should persist a display path instead.
    pub name: String,
    pub bounds: RECT,
    pub primary: bool,
    pub covered: bool,
}

#[derive(Debug, PartialEq)]
pub struct Snapshot {
    pub foreground_frame: Option<RECT>,
    pub displays: Vec<Display>,
    /// A session-wide hint with no display identity.
    pub shell_reports_d3d_fullscreen: Option<bool>,
}

/// Samples visible foreground bounds and each display in physical pixels.
/// Returns `None` when the foreground is still changing, so callers can retain the last state.
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
        // A single flag coalesces bursts. Sampling and logging happen outside the WinEvent callback.
        FOREGROUND_CHANGED.set(true);
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
}
