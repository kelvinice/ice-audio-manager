//! Display monitor enumeration, persistent display paths, DPI scaling, refresh rates, and placement.

use std::collections::HashMap;
use std::mem::size_of;

use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME,
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW, HDC, HMONITOR,
    MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MONITORINFOEXW, MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::core::{BOOL, PCWSTR, Result};

/// Strategy used to select which monitor displays the notch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlacementMode {
    /// Remember a specific monitor by its persistent Windows display path (F14 default).
    #[default]
    Pinned,
    /// Always use Windows' primary display.
    Primary,
    /// Follow whichever display currently holds the mouse cursor.
    Cursor,
}

/// Snapshot of a connected active display monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    /// Session-local GDI device name (e.g. `\\.\DISPLAY1`). Not stable across reconnects.
    pub id: String,
    /// Persistent Windows target device interface path (`monitorDevicePath`), falling back to `id`.
    pub display_path: String,
    /// Human-readable EDID / monitor name (`monitorFriendlyDeviceName`), falling back to `id`.
    pub friendly_name: String,
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

    /// Returns `true` when physical screen coordinate `(x, y)` lies inside this monitor's bounds.
    #[must_use]
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.left
            && x < self.left.saturating_add(self.width)
            && y >= self.top
            && y < self.top.saturating_add(self.height)
    }
}

/// Resolves the target monitor `(index, &MonitorInfo)` according to [`PlacementMode`].
///
/// In `Pinned` mode, matches `pinned_display_path` (case-insensitively on `display_path`, or `id`
/// as a legacy fallback). If the pinned monitor is disconnected, falls back to the primary monitor
/// (then index `0`) without forgetting the saved path so reconnection restores automatically.
#[must_use]
pub fn resolve_target_monitor<'a>(
    monitors: &'a [MonitorInfo],
    mode: PlacementMode,
    pinned_display_path: Option<&str>,
    cursor_pos: Option<(i32, i32)>,
) -> Option<(usize, &'a MonitorInfo)> {
    if monitors.is_empty() {
        return None;
    }
    let primary_fallback =
        || monitors.iter().enumerate().find(|(_, m)| m.is_primary).or_else(|| monitors.first().map(|m| (0, m)));

    match mode {
        PlacementMode::Pinned => {
            if let Some(target) = pinned_display_path.map(str::trim).filter(|s| !s.is_empty()) {
                if let Some(found) =
                    monitors.iter().enumerate().find(|(_, m)| m.display_path.eq_ignore_ascii_case(target))
                {
                    return Some(found);
                }
                if let Some(found) = monitors.iter().enumerate().find(|(_, m)| m.id.eq_ignore_ascii_case(target)) {
                    return Some(found);
                }
            }
            primary_fallback()
        }
        PlacementMode::Primary => primary_fallback(),
        PlacementMode::Cursor => {
            if let Some((cx, cy)) = cursor_pos
                && let Some(found) = monitors.iter().enumerate().find(|(_, m)| m.contains_point(cx, cy))
            {
                return Some(found);
            }
            primary_fallback()
        }
    }
}

/// Enumerates all active monitors, ordered with the primary monitor first.
///
/// # Errors
/// Fails only if `EnumDisplayMonitors` returns an OS failure or finds zero monitors.
pub fn list_monitors() -> Result<Vec<MonitorInfo>> {
    let mut raw_monitors: Vec<MonitorInfo> = Vec::new();
    let ptr = (&raw mut raw_monitors).cast::<()>() as isize;
    // SAFETY: `ptr` points to `raw_monitors` for the synchronous duration of `EnumDisplayMonitors`.
    let ok = unsafe { EnumDisplayMonitors(None, None, Some(enum_monitor_proc), LPARAM(ptr)) };
    if !ok.as_bool() && raw_monitors.is_empty() {
        return Err(windows::core::Error::from_thread());
    }
    if raw_monitors.is_empty() {
        // Fallback to primary monitor at origin if enumeration yielded nothing.
        // SAFETY: querying primary monitor at (0, 0) is always safe.
        let hmon =
            unsafe { MonitorFromPoint(windows::Win32::Foundation::POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        if let Some(info) = query_monitor(hmon) {
            raw_monitors.push(info);
        }
    }

    let display_meta = query_display_paths();
    for m in &mut raw_monitors {
        if let Some((path, friendly)) = display_meta.get(&m.id.to_ascii_uppercase()) {
            if !path.is_empty() {
                m.display_path = path.clone();
            }
            if !friendly.is_empty() {
                m.friendly_name = friendly.clone();
            }
        }
    }

    raw_monitors.sort_by_key(|m| (!m.is_primary, m.left, m.top));
    Ok(raw_monitors)
}

unsafe extern "system" fn enum_monitor_proc(hmonitor: HMONITOR, _hdc: HDC, _rect: *mut RECT, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` was constructed in `list_monitors` from `&raw mut raw_monitors`.
    let out = unsafe { &mut *(lparam.0 as *mut Vec<MonitorInfo>) };
    if let Some(info) = query_monitor(hmonitor) {
        out.push(info);
    }
    BOOL(1)
}

fn utf16z(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end]).trim().to_string()
}

fn query_display_paths() -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = Vec::new();
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = Vec::new();

    for _ in 0..4 {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;
        // SAFETY: out-pointers point to valid stack `u32` values.
        let sz_err =
            unsafe { GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &raw mut path_count, &raw mut mode_count) };
        if sz_err != ERROR_SUCCESS || path_count == 0 {
            return map;
        }
        paths.resize(path_count as usize, DISPLAYCONFIG_PATH_INFO::default());
        modes.resize(mode_count as usize, DISPLAYCONFIG_MODE_INFO::default());
        // SAFETY: `paths` and `modes` are sized according to `path_count` and `mode_count`.
        let err = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &raw mut path_count,
                paths.as_mut_ptr(),
                &raw mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };
        if err == ERROR_SUCCESS {
            paths.truncate(path_count as usize);
            break;
        }
        if err != ERROR_INSUFFICIENT_BUFFER {
            return map;
        }
    }

    for path in &paths {
        let mut src = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
        src.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
        src.header.size = u32::try_from(size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>()).unwrap_or(0);
        src.header.adapterId = path.sourceInfo.adapterId;
        src.header.id = path.sourceInfo.id;
        // SAFETY: `src.header` is initialized with the exact size and `GET_SOURCE_NAME` type.
        if unsafe { DisplayConfigGetDeviceInfo(&raw mut src.header) } != 0 {
            continue;
        }
        let gdi_name = utf16z(&src.viewGdiDeviceName);
        if gdi_name.is_empty() {
            continue;
        }

        let mut tgt = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
        tgt.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
        tgt.header.size = u32::try_from(size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>()).unwrap_or(0);
        tgt.header.adapterId = path.targetInfo.adapterId;
        tgt.header.id = path.targetInfo.id;
        // SAFETY: `tgt.header` is initialized with the exact size and `GET_TARGET_NAME` type.
        if unsafe { DisplayConfigGetDeviceInfo(&raw mut tgt.header) } != 0 {
            continue;
        }
        let device_path = utf16z(&tgt.monitorDevicePath);
        let friendly = utf16z(&tgt.monitorFriendlyDeviceName);
        map.entry(gdi_name.to_ascii_uppercase()).or_insert((device_path, friendly));
    }

    map
}

fn query_monitor(hmonitor: HMONITOR) -> Option<MonitorInfo> {
    let mut mi = MONITORINFOEXW::default();
    mi.monitorInfo.cbSize = u32::try_from(size_of::<MONITORINFOEXW>()).ok()?;
    // SAFETY: `MONITORINFOEXW` begins with `MONITORINFO` and has `cbSize` set to the full struct size.
    let ok = unsafe { GetMonitorInfoW(hmonitor, (&raw mut mi).cast::<MONITORINFO>()) };
    if !ok.as_bool() {
        return None;
    }

    let id = utf16z(&mi.szDevice);
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
        display_path: id.clone(),
        friendly_name: id.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_monitor(id: &str, path: &str, name: &str, is_primary: bool, left: i32) -> MonitorInfo {
        MonitorInfo {
            id: id.into(),
            display_path: path.into(),
            friendly_name: name.into(),
            is_primary,
            left,
            top: 0,
            width: 1920,
            height: 1080,
            work_left: left,
            work_top: 0,
            work_width: 1920,
            work_height: 1032,
            dpi_x: 96,
            scale_factor: 1.0,
            refresh_hz: if is_primary { 144 } else { 75 },
        }
    }

    #[test]
    fn pinned_mode_tracks_display_path_across_renumbered_gdi_ids_and_reconnects() {
        let primary = fake_monitor("\\\\.\\DISPLAY1", "\\\\?\\DISPLAY#DEL1000#1", "Primary Dell", true, 0);
        let side = fake_monitor("\\\\.\\DISPLAY2", "\\\\?\\DISPLAY#SAM2000#2", "Side Samsung", false, -1920);
        let initial = vec![primary.clone(), side.clone()];

        // Pinned to secondary monitor by display_path.
        let (idx, resolved) =
            resolve_target_monitor(&initial, PlacementMode::Pinned, Some("\\\\?\\display#sam2000#2"), None)
                .expect("resolved monitor");
        assert_eq!(idx, 1);
        assert_eq!(resolved.id, "\\\\.\\DISPLAY2");

        // Disconnected: temporarily falls back to primary.
        let disconnected = vec![primary.clone()];
        let (idx, resolved) =
            resolve_target_monitor(&disconnected, PlacementMode::Pinned, Some("\\\\?\\DISPLAY#SAM2000#2"), None)
                .expect("primary fallback");
        assert_eq!(idx, 0);
        assert!(resolved.is_primary);

        // Reconnected with swapped GDI session ID (DISPLAY4 instead of DISPLAY2): returns automatically.
        let reconnected =
            vec![primary, fake_monitor("\\\\.\\DISPLAY4", "\\\\?\\DISPLAY#SAM2000#2", "Side Samsung", false, -1920)];
        let (idx, resolved) =
            resolve_target_monitor(&reconnected, PlacementMode::Pinned, Some("\\\\?\\DISPLAY#SAM2000#2"), None)
                .expect("reconnected pinned monitor");
        assert_eq!(idx, 1);
        assert_eq!(resolved.display_path, "\\\\?\\DISPLAY#SAM2000#2");
    }

    #[test]
    fn primary_and_cursor_modes_resolve_cleanly() {
        let primary = fake_monitor("\\\\.\\DISPLAY1", "\\\\?\\DISPLAY#DEL1000#1", "Primary", true, 0);
        let left = fake_monitor("\\\\.\\DISPLAY2", "\\\\?\\DISPLAY#SAM2000#2", "Left", false, -1920);
        let list = vec![primary, left];

        let (p_idx, _) =
            resolve_target_monitor(&list, PlacementMode::Primary, Some("\\\\?\\DISPLAY#SAM2000#2"), Some((-500, 100)))
                .expect("primary");
        assert_eq!(p_idx, 0);

        // Cursor on negative-X secondary display selects index 1; missing cursor falls back to primary.
        let (c_idx, c_mon) =
            resolve_target_monitor(&list, PlacementMode::Cursor, None, Some((-500, 200))).expect("cursor monitor");
        assert_eq!(c_idx, 1);
        assert_eq!(c_mon.id, "\\\\.\\DISPLAY2");

        let (fb_idx, _) = resolve_target_monitor(&list, PlacementMode::Cursor, None, None).expect("fallback");
        assert_eq!(fb_idx, 0);
    }
}
