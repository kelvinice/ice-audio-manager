use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use ice_core::device::DeviceState;
use ice_core::engine::{Command, DeviceSummary, EngineSnapshot, RoleSet};
use ice_win::autostart::Autostart;
use tauri::AppHandle;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::window::WindowBuilder;

use super::{SharedRuntime, create_notch_window, create_settings_window, cycle_monitor_internal, spawn_cursor_poll};

const TRAY_ID: &str = "ice-audio-tray";
const DEVICE_PREFIX: &str = "switch/";

pub(super) fn refresh_tooltip(app: &AppHandle, snapshot: &EngineSnapshot) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let device = snapshot.playback.iter().find(|device| device.is_default);
        let tooltip = device.map_or_else(
            || "Ice Audio Manager (preview)".to_owned(),
            |device| format!("Ice Audio Manager (preview) · {}", device.name),
        );
        let tooltip: String = tooltip.chars().take(60).collect();
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

pub(super) fn build(app: &AppHandle, state: Arc<SharedRuntime>) -> tauri::Result<()> {
    let popup_owner =
        WindowBuilder::new(app, "tray-menu-owner").visible(false).skip_taskbar(true).decorations(false).build()?;
    let startup = Autostart::current().and_then(|mode| Ok(mode.enabled()?.map(|enabled| (mode, enabled))));
    let startup = match startup {
        Ok(startup) => startup,
        Err(error) => {
            tracing::warn!(%error, "Could not read launch-at-sign-in state");
            None
        }
    };
    let startup_enabled = Mutex::new(startup.is_some_and(|(_, enabled)| enabled));
    let launch = CheckMenuItem::with_id(
        app,
        "launch",
        "Launch at sign-in",
        startup.is_some(),
        startup.is_some_and(|(_, enabled)| enabled),
        None::<&str>,
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &item(app, "settings", "Settings")?,
            &item(app, "notch", "Show notch preview")?,
            &item(app, "monitor", "Move notch to next monitor")?,
            &launch,
            &PredefinedMenuItem::separator(app)?,
            &item(app, "quit", "Quit Ice Audio Manager")?,
        ],
    )?;

    let menu_state = Arc::clone(&state);
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Ice Audio Manager (preview)")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            if let Some(device_id) = event.id.as_ref().strip_prefix(DEVICE_PREFIX) {
                menu_state.engine.send(Command::SwitchTo { device_id: device_id.to_owned(), roles: RoleSet::Default });
                return;
            }
            let operation = match event.id.as_ref() {
                "settings" => create_settings_window(app, &menu_state),
                "notch" => create_notch_window(app, &menu_state).map(|_| {
                    spawn_cursor_poll(Arc::clone(&menu_state), false);
                }),
                "monitor" => cycle_monitor_internal(app, &menu_state),
                "launch" => {
                    if let Some((startup, _)) = startup {
                        let mut enabled = startup_enabled.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                        let next = !*enabled;
                        let result = std::env::current_exe()
                            .map_err(|error| error.to_string())
                            .and_then(|exe| startup.set_enabled(next, &exe).map_err(|error| error.to_string()));
                        if result.is_ok() {
                            *enabled = next;
                        }
                        let _ = launch.set_checked(*enabled);
                        if result.is_err() {
                            let _ = launch.set_text("Launch at sign-in (change failed)");
                        } else {
                            let _ = launch.set_text("Launch at sign-in");
                        }
                        result
                    } else {
                        Ok(())
                    }
                }
                "quit" => {
                    menu_state.allow_exit.store(true, Ordering::Release);
                    app.exit(0);
                    Ok(())
                }
                _ => Ok(()),
            };
            if let Err(error) = operation {
                tracing::warn!(action = %event.id.as_ref(), %error, "Tray action failed");
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                let result = quick_menu(app, &state.engine.snapshot()).and_then(|menu| popup_owner.popup_menu(&menu));
                if let Err(error) = result {
                    tracing::warn!(%error, "Could not open the audio device menu");
                }
            }
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

fn item(app: &AppHandle, id: &str, text: &str) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(app, id, text, true, None::<&str>)
}

fn quick_menu(app: &AppHandle, snapshot: &EngineSnapshot) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    append_devices(app, &menu, "playback", "Playback devices", &snapshot.playback)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    append_devices(app, &menu, "recording", "Recording devices", &snapshot.recording)?;
    Ok(menu)
}

fn append_devices(
    app: &AppHandle,
    menu: &Menu<tauri::Wry>,
    heading_id: &str,
    heading: &str,
    devices: &[DeviceSummary],
) -> tauri::Result<()> {
    menu.append(&MenuItem::with_id(app, heading_id, heading, false, None::<&str>)?)?;
    for device in devices.iter().filter(|device| device.state == DeviceState::Active) {
        let id = format!("{DEVICE_PREFIX}{}", device.id);
        let mut label = device.name.replace('&', "&&");
        if device.is_default_communications {
            label.push_str(" (communications)");
        }
        menu.append(&CheckMenuItem::with_id(app, id, label, true, device.is_default, None::<&str>)?)?;
    }
    Ok(())
}
