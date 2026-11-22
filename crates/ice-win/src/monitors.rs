//! Display monitor enumeration, DPI scaling, and refresh rates.

use std::mem::size_of;

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW, HDC, HMONITOR,
    MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MONITORINFOEXW, MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::core::{BOOL, PCWSTR, Result};

/// Snapshot of a connected active display monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    /// Device name (e.g. `\\.\DISPLAY1`).
    pub id: String,
    pub is_primary: bool,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub work_left: i32,
    pub work_top: i32,
    pub work_width: i32,
    pub work_height: i32,
    pub dpi_x: u32,
    pub scale_factor: f64,
    pub refresh_hz: u32,
}

impl MonitorInfo {
    /// Converts logical CSS/UI pixels to physical Win32 pixels on this monitor.
    #[must_use]
    pub fn logical_to_physical(&self, logical_px: i32) -> i32 {
        (f64::from(logical_px) * self.scale_factor).round() as i32
    }
}

/// Enumerates all active monitors, ordered with the primary monitor first.
///
/// # Errors
/// Fails only if `EnumDisplayMonitors` returns an OS failure or finds zero monitors.
pub fn list_monitors() -> Result<Vec<MonitorInfo>> {
    let mut monitors: Vec<MonitorInfo> = Vec::new();
    let ptr = (&raw mut monitors).cast::<()>() as isize;
    // SAFETY: `ptr` points to `monitors` for the synchronous duration of `EnumDisplayMonitors`.
    let ok = unsafe { EnumDisplayMonitors(None, None, Some(enum_monitor_proc), LPARAM(ptr)) };
    if !ok.as_bool() && monitors.is_empty() {
        return Err(windows::core::Error::from_thread());
    }
    if monitors.is_empty() {
        // Fallback to primary monitor at origin if enumeration yielded nothing.
        // SAFETY: querying primary monitor at (0, 0) is always safe.
        let hmon =
            unsafe { MonitorFromPoint(windows::Win32::Foundation::POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        if let Some(info) = query_monitor(hmon) {
            monitors.push(info);
        }
    }
    monitors.sort_by_key(|m| (!m.is_primary, m.left, m.top));
    Ok(monitors)
}

unsafe extern "system" fn enum_monitor_proc(hmonitor: HMONITOR, _hdc: HDC, _rect: *mut RECT, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` was constructed in `list_monitors` from `&raw mut monitors`.
    let out = unsafe { &mut *(lparam.0 as *mut Vec<MonitorInfo>) };
    if let Some(info) = query_monitor(hmonitor) {
        out.push(info);
    }
    BOOL(1)
}

fn query_monitor(hmonitor: HMONITOR) -> Option<MonitorInfo> {
    let mut mi = MONITORINFOEXW::default();
    mi.monitorInfo.cbSize = u32::try_from(size_of::<MONITORINFOEXW>()).ok()?;
    // SAFETY: `MONITORINFOEXW` begins with `MONITORINFO` and has `cbSize` set to the full struct size.
    let ok = unsafe { GetMonitorInfoW(hmonitor, (&raw mut mi).cast::<MONITORINFO>()) };
    if !ok.as_bool() {
        return None;
    }

    let end = mi.szDevice.iter().position(|&c| c == 0).unwrap_or(mi.szDevice.len());
    let id = String::from_utf16_lossy(&mi.szDevice[..end]);

    let mut dpi_x = 96u32;
    let mut dpi_y = 96u32;
    // SAFETY: `hmonitor` is valid during enumeration; ignore failure and keep 96 DPI default.
    let _ = unsafe { GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &raw mut dpi_x, &raw mut dpi_y) };
    let scale_factor = f64::from(dpi_x) / 96.0;

    let mut dm = DEVMODEW { dmSize: u16::try_from(size_of::<DEVMODEW>()).unwrap_or(0), ..Default::default() };
    // SAFETY: `mi.szDevice` is null-terminated; `dmSize` is initialized.
    let refresh_hz = if unsafe {
        EnumDisplaySettingsW(PCWSTR(mi.szDevice.as_ptr()), ENUM_CURRENT_SETTINGS, &raw mut dm)
    }
    .as_bool()
    {
        dm.dmDisplayFrequency
    } else {
        60
    };

    let rc = mi.monitorInfo.rcMonitor;
    let work = mi.monitorInfo.rcWork;
    const MONITORINFOF_PRIMARY_BIT: u32 = 1;
    Some(MonitorInfo {
        id,
        is_primary: (mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY_BIT) != 0,
        left: rc.left,
        top: rc.top,
        width: rc.right - rc.left,
        height: rc.bottom - rc.top,
        work_left: work.left,
        work_top: work.top,
        work_width: work.right - work.left,
        work_height: work.bottom - work.top,
        dpi_x,
        scale_factor,
        refresh_hz: refresh_hz.max(1),
    })
}
