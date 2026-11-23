//! The Settings window (Phase 4, F19): created when opened and destroyed when closed (code-practices §2), with a Mica
//! backdrop that follows Windows' light/dark mode. Its commands can only change settings (`Command::is_setting`) and
//! launch at sign-in; switching devices or controlling media stays with the tray and the notch.

use std::sync::Arc;

use ice_core::engine::Command;
use ice_win::overlay_win;
use tauri::{AppHandle, Manager, State, Theme, WebviewUrl, WebviewWindowBuilder};

use super::{SharedRuntime, placement, startup};

pub(super) const LABEL: &str = "settings";
/// Logical size: a navigation rail plus a content column that fits a 600 px card at 150 % on a 1080p screen.
const WIDTH: i32 = 920;
const HEIGHT: i32 = 640;
const MIN_WIDTH: f64 = 720.0;
const MIN_HEIGHT: f64 = 480.0;

pub(super) fn create_window(app: &AppHandle, state: &Arc<SharedRuntime>) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(LABEL) {
        let _ = existing.unminimize();
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }
    // Built hidden, then moved to the notch's monitor before it shows (owner request: Settings opens where the notch is).
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("settings.html".into()))
        .title("Ice Audio Manager Settings")
        .inner_size(f64::from(WIDTH), f64::from(HEIGHT))
        .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
        // A transparent page lets the Mica backdrop show through (theming.md §1).
        .transparent(true)
        .visible(false)
        .data_directory(state.settings_wv2_data_dir.clone())
        .build()
        .map_err(|e| e.to_string())?;
    if let Some(monitor) = state.active_monitor_info() {
        let (x, y) = placement::centred_in_work_area(&monitor, WIDTH, HEIGHT);
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        // Re-apply the logical size after the move, so a DPI change between monitors keeps it centred.
        let _ = win.set_size(tauri::LogicalSize::new(f64::from(WIDTH), f64::from(HEIGHT)));
    }
    let hwnd = win.hwnd().map_err(|e| e.to_string())?.0 as isize;
    apply_backdrop(hwnd, win.theme().unwrap_or(Theme::Dark));
    let _ = win.show();
    let _ = win.set_focus();
    Ok(())
}

/// Mica plus a title bar that matches the Windows app mode. Called again on `WindowEvent::ThemeChanged`.
pub(super) fn apply_backdrop(hwnd: isize, theme: Theme) {
    if let Err(error) = overlay_win::apply_settings_mica(hwnd, theme == Theme::Dark) {
        tracing::debug!(%error, "Mica backdrop unavailable; Settings uses its solid fallback colours");
    }
}

#[tauri::command]
pub(super) fn apply_setting(command: Command, state: State<'_, Arc<SharedRuntime>>) -> Result<(), String> {
    if !command.is_setting() {
        return Err("The Settings window can only change settings.".into());
    }
    state.engine.send(command);
    Ok(())
}

/// `null` when this build can't change it (see [`startup::enabled`]).
#[tauri::command]
pub(super) fn get_launch_at_sign_in() -> Option<bool> {
    startup::enabled()
}

#[tauri::command]
pub(super) fn set_launch_at_sign_in(enabled: bool) -> Result<Option<bool>, String> {
    startup::set_enabled(enabled)?;
    Ok(startup::enabled())
}
