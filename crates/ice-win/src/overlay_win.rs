//! Overlay window styling, non-activating click handling, click-through, Mica backdrop, and positioning.

use core::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWM_SYSTEMBACKDROP_TYPE, DWM_WINDOW_CORNER_PREFERENCE, DWMSBT_MAINWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, HWND_TOPMOST,
    MA_NOACTIVATE, SM_CXDRAG, SM_CYDRAG, SM_SWAPBUTTON, SPI_GETCLIENTAREAANIMATION, SWP_ASYNCWINDOWPOS,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSENDCHANGING, SWP_NOSIZE, SWP_NOZORDER,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos,
    SystemParametersInfoW, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WM_MOUSEACTIVATE, WM_NCDESTROY, WS_EX_APPWINDOW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
};
use windows::core::{BOOL, Result};

use ice_core::appearance::NotchStyle;
use ice_core::placement::{
    Area, NOTCH_CANVAS_H, NOTCH_CANVAS_W, NotchAnchor, anchor_origin_at, canvas_edge_offset_for,
};

use crate::monitors::MonitorInfo;

const OVERLAY_SUBCLASS_ID: usize = 0x4943_4530; // "ICE0"

fn to_hwnd(raw: isize) -> Result<HWND> {
    if raw == 0 {
        return Err(windows::core::Error::from_thread());
    }
    Ok(HWND(raw as *mut c_void))
}

/// Applies topmost, tool-window, non-activating styles and installs a `WM_MOUSEACTIVATE` subclass
/// returning `MA_NOACTIVATE` so clicking interactive notch controls never steals foreground focus.
///
/// # Errors
/// Returns an OS error if `hwnd` is null or `SetWindowPos` fails.
pub fn apply_overlay_styles(hwnd: isize) -> Result<()> {
    let hwnd = to_hwnd(hwnd)?;
    // SAFETY: `hwnd` is a live top-level window handle belonging to this process.
    let current = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let desired =
        (current | (WS_EX_NOACTIVATE.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize) | (WS_EX_TOPMOST.0 as isize))
            & !(WS_EX_APPWINDOW.0 as isize);
    if desired != current {
        // SAFETY: `hwnd` is valid; updating extended style bits.
        unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired) };
    }
    // SAFETY: promotes the window to topmost without activating it.
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
        )?;
    }
    // SAFETY: `noactivate_subclass` handles `WM_NCDESTROY` cleanup; window belongs to caller's UI thread.
    let _ = unsafe { SetWindowSubclass(hwnd, Some(noactivate_subclass), OVERLAY_SUBCLASS_ID, 0) };
    Ok(())
}

/// Removes the non-activating subclass if still attached.
pub fn remove_overlay_subclass(hwnd: isize) {
    if let Ok(hwnd) = to_hwnd(hwnd) {
        // SAFETY: safe even if subclass was never installed or already removed.
        let _ = unsafe { RemoveWindowSubclass(hwnd, Some(noactivate_subclass), OVERLAY_SUBCLASS_ID) };
    }
}

unsafe extern "system" fn noactivate_subclass(
    hwnd: HWND,
    umsg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    uidsubclass: usize,
    _dwrefdata: usize,
) -> LRESULT {
    if umsg == WM_MOUSEACTIVATE {
        return LRESULT(MA_NOACTIVATE as isize);
    }
    if umsg == WM_NCDESTROY {
        // SAFETY: detaches this subclass cleanly when the window is destroyed.
        let _ = unsafe { RemoveWindowSubclass(hwnd, Some(noactivate_subclass), uidsubclass) };
    }
    // SAFETY: forwards all other messages down the standard subclass chain.
    unsafe { DefSubclassProc(hwnd, umsg, wparam, lparam) }
}

/// Whether clicks pass through `hwnd` to the windows below: both `WS_EX_TRANSPARENT` and `WS_EX_LAYERED` are set.
/// `WS_EX_TRANSPARENT` alone doesn't do it for a non-layered window (the WebView notch): it kept catching every click
/// on its 648 × 292 canvas. False for a null or dead handle.
#[must_use]
pub fn ignores_cursor(hwnd: isize) -> bool {
    let Ok(hwnd) = to_hwnd(hwnd) else {
        return false;
    };
    let both = (WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0) as isize;
    // SAFETY: reading a window's extended style has no side effects; a dead handle returns 0.
    let current = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    current & both == both
}

/// Toggles `WS_EX_TRANSPARENT` click-through on an **already layered** window (the native pill). The WebView notch is
/// not layered and is driven through tao's `set_ignore_cursor_events` instead (see the shell's cursor poll).
///
/// # Errors
/// Fails if `hwnd` is null.
pub fn set_click_through(hwnd: isize, ignore: bool) -> Result<()> {
    let hwnd = to_hwnd(hwnd)?;
    // SAFETY: `hwnd` is a valid window handle.
    let current = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let bit = WS_EX_TRANSPARENT.0 as isize;
    let desired = if ignore { current | bit } else { current & !bit };
    if desired != current {
        // SAFETY: `hwnd` is valid; updating `WS_EX_TRANSPARENT` only when changed avoids redundant DWM updates.
        unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired) };
    }
    Ok(())
}

/// Leaves the window out of screen captures (desktop duplication, Windows.Graphics.Capture, `BitBlt`) while it stays
/// visible on the user's own screen (D-27 hibernate), or puts it back with `exclude = false`. Documented
/// `SetWindowDisplayAffinity` (`WDA_EXCLUDEFROMCAPTURE`, Windows 10 2004+); call it on the window's own thread.
///
/// # Errors
/// Fails if `hwnd` is null or Windows refuses the affinity (e.g. no DWM composition).
pub fn set_capture_excluded(hwnd: isize, exclude: bool) -> Result<()> {
    let hwnd = to_hwnd(hwnd)?;
    let affinity = if exclude { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE };
    // SAFETY: `hwnd` is a live top-level window owned by this process; only its capture affinity changes.
    unsafe { SetWindowDisplayAffinity(hwnd, affinity) }
}

/// The monitor's work area (the screen minus the taskbar), which every anchor is measured in.
#[must_use]
pub const fn work_area(monitor: &MonitorInfo) -> Area {
    Area { left: monitor.work_left, top: monitor.work_top, width: monitor.work_width, height: monitor.work_height }
}

/// Where a `logical_w` × `logical_h` notch box goes on `monitor` for `anchor` and optional `x_ratio` (D-20): centred
/// horizontally (clamped so the full notch canvas stays on-screen), `edge_offset_logical` in from the anchored edge.
#[must_use]
pub fn notch_origin(
    monitor: &MonitorInfo,
    anchor: NotchAnchor,
    logical_w: i32,
    logical_h: i32,
    edge_offset_logical: i32,
) -> (i32, i32) {
    notch_origin_at(monitor, anchor, None, logical_w, logical_h, edge_offset_logical)
}

/// Like [`notch_origin`], accepting an optional continuous horizontal ratio (`0.0..=1.0`).
#[must_use]
pub fn notch_origin_at(
    monitor: &MonitorInfo,
    anchor: NotchAnchor,
    x_ratio: Option<f32>,
    logical_w: i32,
    logical_h: i32,
    edge_offset_logical: i32,
) -> (i32, i32) {
    anchor_origin_at(
        anchor,
        x_ratio,
        work_area(monitor),
        monitor.logical_to_physical(NOTCH_CANVAS_W),
        monitor.logical_to_physical(logical_w),
        monitor.logical_to_physical(logical_h),
        monitor.logical_to_physical(edge_offset_logical),
    )
}

/// Where the WebView notch canvas (`NOTCH_CANVAS_W` × `NOTCH_CANVAS_H`) goes for `(anchor, x_ratio, style)`: flush at
/// top, flush on taskbar at bottom for Edge Nook, or overhanging by shadow room at bottom for Floating.
#[must_use]
pub fn canvas_origin(
    monitor: &MonitorInfo,
    anchor: NotchAnchor,
    x_ratio: Option<f32>,
    style: NotchStyle,
) -> (i32, i32) {
    notch_origin_at(monitor, anchor, x_ratio, NOTCH_CANVAS_W, NOTCH_CANVAS_H, canvas_edge_offset_for(anchor, style))
}

/// Positions and sizes the WebView notch canvas at `(anchor, x_ratio, style)` on `monitor` (D-20).
///
/// # Errors
/// Fails if `hwnd` is null or `SetWindowPos` returns an OS error.
pub fn move_notch(
    hwnd: isize,
    monitor: &MonitorInfo,
    anchor: NotchAnchor,
    x_ratio: Option<f32>,
    style: NotchStyle,
) -> Result<()> {
    let hwnd = to_hwnd(hwnd)?;
    let phys_w = monitor.logical_to_physical(NOTCH_CANVAS_W);
    let phys_h = monitor.logical_to_physical(NOTCH_CANVAS_H);
    let (x, y) = canvas_origin(monitor, anchor, x_ratio, style);
    // SAFETY: `hwnd` is valid; `SWP_NOACTIVATE | SWP_NOSENDCHANGING` prevents activation or redundant frame flashes.
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            phys_w,
            phys_h,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOSENDCHANGING,
        )
    }
}

/// Restores `HWND_TOPMOST` ordering without moving or activating the window.
///
/// # Errors
/// Fails if `hwnd` is null or `SetWindowPos` fails.
pub fn ensure_topmost(hwnd: isize) -> Result<()> {
    let hwnd = to_hwnd(hwnd)?;
    // SAFETY: `hwnd` is valid; keeps overlay topmost without focus steal.
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOSENDCHANGING,
        )
    }
}

/// Moves a window owned by another thread to `(x, y)` without waiting for that thread (`SWP_ASYNCWINDOWPOS`), never
/// activating it. The D-20 drag loop moves the WebView notch with it every frame.
///
/// # Errors
/// Fails if `hwnd` is null or `SetWindowPos` fails.
pub fn move_window_async(hwnd: isize, x: i32, y: i32) -> Result<()> {
    let hwnd = to_hwnd(hwnd)?;
    // SAFETY: `hwnd` is valid; the move is posted to its thread, so this call never blocks on it.
    unsafe { SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS) }
}

/// Moves an already-sized window to `(x, y)` without changing size or Z-order.
///
/// # Errors
/// Fails if `hwnd` is null or `SetWindowPos` fails.
pub fn move_pos_no_resize(hwnd: isize, x: i32, y: i32) -> Result<()> {
    let hwnd = to_hwnd(hwnd)?;
    // SAFETY: `hwnd` is valid.
    unsafe { SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) }
}

/// Applies Windows 11 Mica backdrop, immersive dark titlebar, and rounded corners to the Settings window.
///
/// # Errors
/// Returns an OS error if `hwnd` is invalid or `DwmSetWindowAttribute` fails.
pub fn apply_settings_mica(hwnd: isize, dark: bool) -> Result<()> {
    let hwnd = to_hwnd(hwnd)?;
    let dark_val = BOOL::from(dark);
    let backdrop: DWM_SYSTEMBACKDROP_TYPE = DWMSBT_MAINWINDOW;
    let corner: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
    // SAFETY: each attribute points to a 4-byte POD stack variable whose exact size is passed.
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&raw const dark_val).cast::<c_void>(),
            u32::try_from(size_of::<BOOL>()).unwrap_or(4),
        )?;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            (&raw const backdrop).cast::<c_void>(),
            u32::try_from(size_of::<DWM_SYSTEMBACKDROP_TYPE>()).unwrap_or(4),
        )?;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&raw const corner).cast::<c_void>(),
            u32::try_from(size_of::<DWM_WINDOW_CORNER_PREFERENCE>()).unwrap_or(4),
        )?;
    }
    Ok(())
}

/// Returns current screen cursor coordinates `(x, y)` in physical pixels.
#[must_use]
pub fn cursor_pos() -> Option<(i32, i32)> {
    let mut pt = POINT::default();
    // SAFETY: stack `POINT` pointer is valid.
    unsafe { GetCursorPos(&raw mut pt) }.ok()?;
    Some((pt.x, pt.y))
}

/// Whether the primary mouse button is down right now (the D-20 drag loop's "still dragging" check). Reads the async
/// key state: no hook, nothing installed (code-practices §1.1). Honours swapped buttons.
#[must_use]
pub fn primary_button_down() -> bool {
    // SAFETY: plain reads of system metrics and key state; no pointers.
    let swapped = unsafe { GetSystemMetrics(SM_SWAPBUTTON) } != 0;
    let key = if swapped { VK_RBUTTON } else { VK_LBUTTON };
    // SAFETY: as above.
    let state = unsafe { GetAsyncKeyState(i32::from(key.0)) };
    state < 0
}

/// Whether Esc is down right now (cancels a drag). Async key state, no hook.
#[must_use]
pub fn escape_down() -> bool {
    // SAFETY: plain read of the key state.
    let state = unsafe { GetAsyncKeyState(i32::from(VK_ESCAPE.0)) };
    state < 0
}

/// Windows' drag threshold (`SM_CXDRAG`, `SM_CYDRAG`, physical px): a press that moves further is a drag, not a click.
#[must_use]
pub fn drag_threshold() -> (i32, i32) {
    // SAFETY: plain reads of system metrics.
    unsafe { (GetSystemMetrics(SM_CXDRAG).max(1), GetSystemMetrics(SM_CYDRAG).max(1)) }
}

/// Returns the physical bounding rectangle `(left, top, right, bottom)` of `hwnd`.
#[must_use]
pub fn window_rect(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    let hwnd = to_hwnd(hwnd).ok()?;
    let mut rc = RECT::default();
    // SAFETY: stack `RECT` pointer is valid.
    unsafe { GetWindowRect(hwnd, &raw mut rc) }.ok()?;
    Some((rc.left, rc.top, rc.right, rc.bottom))
}

/// Returns the raw foreground `HWND` (or `0` if none).
#[must_use]
pub fn foreground_hwnd() -> isize {
    // SAFETY: plain query returning foreground HWND or null.
    unsafe { GetForegroundWindow() }.0 as isize
}

/// Returns `true` if Windows "Animation effects" are disabled (`SPI_GETCLIENTAREAANIMATION == FALSE`).
#[must_use]
pub fn reduce_motion_enabled() -> bool {
    let mut anim_enabled: i32 = 1;
    // SAFETY: `pvparam` points to a stack `i32` (`BOOL`) as documented for `SPI_GETCLIENTAREAANIMATION`.
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut anim_enabled).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    ok.is_ok() && anim_enabled == 0
}
