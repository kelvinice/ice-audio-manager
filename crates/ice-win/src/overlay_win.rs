//! Overlay window styling, non-activating click handling, click-through, Mica backdrop, and positioning.

use core::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWM_SYSTEMBACKDROP_TYPE, DWM_WINDOW_CORNER_PREFERENCE, DWMSBT_MAINWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, HWND_TOPMOST, MA_NOACTIVATE,
    SPI_GETCLIENTAREAANIMATION, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSENDCHANGING,
    SWP_NOSIZE, SWP_NOZORDER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetWindowDisplayAffinity, SetWindowLongPtrW,
    SetWindowPos, SystemParametersInfoW, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WM_MOUSEACTIVATE, WM_NCDESTROY,
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
};
use windows::core::{BOOL, Result};

use ice_core::placement::{Area, NOTCH_CANVAS_W, NotchAnchor, anchor_origin};

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

/// Where a `logical_w`-wide notch box goes on `monitor` for `anchor` (D-20): centred on the anchor's point in the work
/// area, which reserves the full notch canvas, `top_offset_logical` below the work area's top. Physical px.
/// Shared by the WebView notch and the native pill, so both sit on one centre (`ice_core::placement`).
#[must_use]
pub fn notch_origin(monitor: &MonitorInfo, anchor: NotchAnchor, logical_w: i32, top_offset_logical: i32) -> (i32, i32) {
    let area =
        Area { left: monitor.work_left, top: monitor.work_top, width: monitor.work_width, height: monitor.work_height };
    anchor_origin(
        anchor,
        area,
        monitor.logical_to_physical(NOTCH_CANVAS_W),
        monitor.logical_to_physical(logical_w),
        monitor.logical_to_physical(top_offset_logical),
    )
}

/// Positions and sizes the overlay window at `anchor` on `monitor` (D-20), from logical coordinates.
///
/// # Errors
/// Fails if `hwnd` is null or `SetWindowPos` returns an OS error.
pub fn move_notch(
    hwnd: isize,
    monitor: &MonitorInfo,
    anchor: NotchAnchor,
    logical_w: i32,
    logical_h: i32,
    top_offset_logical: i32,
) -> Result<()> {
    let hwnd = to_hwnd(hwnd)?;
    let phys_w = monitor.logical_to_physical(logical_w);
    let phys_h = monitor.logical_to_physical(logical_h);
    let (x, y) = notch_origin(monitor, anchor, logical_w, top_offset_logical);
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
