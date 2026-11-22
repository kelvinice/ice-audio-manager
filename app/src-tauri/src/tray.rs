use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use ice_core::device::DeviceState;
use ice_core::engine::{Command, DeviceSummary, EngineSnapshot, Notice, RoleSet};
use ice_win::autostart::Autostart;
use tauri::AppHandle;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::window::WindowBuilder;

use super::{Preview, SharedRuntime, cycle_monitor_internal, request_preview};

const TRAY_ID: &str = "ice-audio-tray";
const DEVICE_PREFIX: &str = "switch/";

const CHOICE_PREFIX: &str = "choice/";
const DISMISS_PREFIX: &str = "dismiss/";

#[derive(Clone, Default)]
pub(super) struct Prompts {
    choice: Option<Choice>,
    disconnected: Option<String>,
}

#[derive(Clone)]
struct Choice {
    request_id: u64,
    name: String,
    candidates: Vec<DeviceSummary>,
}

impl Choice {
    fn answer(&self, action: &str) -> Option<Command> {
        let (request, selection) = action.split_once('/')?;
        if request.parse::<u64>().ok()? != self.request_id {
            return None;
        }
        let device_id = if selection == "cancel" {
            None
        } else {
            Some(self.candidates.get(selection.parse::<usize>().ok()?)?.id.clone())
        };
        Some(Command::ResolveChoice { request_id: self.request_id, device_id })
    }
}

fn prompts(state: &SharedRuntime) -> MutexGuard<'_, Prompts> {
    state.tray_prompts.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn show_notice(state: &SharedRuntime, notice: &Notice) {
    match notice {
        Notice::NotConnected { name } => prompts(state).disconnected = Some(name.clone()),
        Notice::ChooseDevice { request_id, name, candidates } => {
            let choice = Choice { request_id: *request_id, name: name.clone(), candidates: candidates.clone() };
            let previous = prompts(state).choice.replace(choice);
            if let Some(previous) = previous {
                state.engine.send(Command::ResolveChoice { request_id: previous.request_id, device_id: None });
            }
        }
        _ => {}
    }
}

pub(super) fn refresh_tooltip(app: &AppHandle, state: &SharedRuntime, snapshot: &EngineSnapshot) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let prompts = prompts(state);
        let status = if prompts.choice.is_some() {
            Some("Left-click to choose a device")
        } else if prompts.disconnected.is_some() || !snapshot.notices.is_empty() {
            Some("Left-click to see audio notices")
        } else {
            snapshot.playback.iter().find(|device| device.is_default).map(|device| device.name.as_str())
        };
        let tooltip =
            status.map_or_else(|| "Ice Audio Manager".to_owned(), |status| format!("Ice Audio Manager · {status}"));
        drop(prompts);
        let tooltip: String = tooltip.chars().take(60).collect();
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

fn handle_prompt_action(app: &AppHandle, state: &SharedRuntime, id: &str) -> bool {
    if let Some(action) = id.strip_prefix(CHOICE_PREFIX) {
        let command = {
            let mut prompts = prompts(state);
            let command = prompts.choice.as_ref().and_then(|choice| choice.answer(action));
            if command.is_some() {
                prompts.choice = None;
            }
            command
        };
        if let Some(command) = command {
            state.engine.send(command);
            refresh_tooltip(app, state, &state.engine.snapshot());
        }
    } else if let Some(name) = id.strip_prefix(DISMISS_PREFIX) {
        let dismissed = {
            let mut prompts = prompts(state);
            let same_notice = prompts.disconnected.as_deref() == Some(name);
            if same_notice {
                prompts.disconnected = None;
            }
            same_notice
        };
        if dismissed {
            refresh_tooltip(app, state, &state.engine.snapshot());
        }
    } else {
        return false;
    }
    true
}

pub(super) fn build(app: &AppHandle, state: Arc<SharedRuntime>) -> tauri::Result<()> {
    let popup_owner =
        WindowBuilder::new(app, "tray-menu-owner").visible(false).skip_taskbar(true).decorations(false).build()?;
    let owner_hwnd = popup_owner.hwnd()?.0 as isize;
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
        .tooltip("Ice Audio Manager")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            if handle_prompt_action(app, &menu_state, event.id.as_ref()) {
                return;
            }
            if let Some(device_id) = event.id.as_ref().strip_prefix(DEVICE_PREFIX) {
                menu_state.engine.send(Command::SwitchTo { device_id: device_id.to_owned(), roles: RoleSet::Default });
                return;
            }
            let operation = match event.id.as_ref() {
                "settings" => {
                    request_preview(app, &menu_state, Preview::Settings);
                    Ok(())
                }
                "notch" => {
                    request_preview(app, &menu_state, Preview::Notch);
                    Ok(())
                }
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
                let visible_prompts = prompts(&state).clone();
                let snapshot = state.engine.snapshot();
                tracing::debug!(revision = snapshot.revision, "Opening audio tray menu");
                let result =
                    quick_menu(app, &snapshot, &visible_prompts).and_then(|menu| popup_owner.popup_menu(&menu));
                if let Err(error) = ice_win::desktop::finish_tray_menu(owner_hwnd) {
                    tracing::debug!(%error, "Could not signal the tray popup closed");
                }
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

fn quick_menu(app: &AppHandle, snapshot: &EngineSnapshot, prompts: &Prompts) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    if let Some(choice) = &prompts.choice {
        let picker = Submenu::new(app, menu_text(&format!("Choose device for {}", choice.name)), true)?;
        for (index, device) in choice.candidates.iter().enumerate() {
            let label = if choice.candidates.iter().any(|other| other.id != device.id && other.name == device.name) {
                let suffix: String = device.id.chars().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect();
                format!("{} (…{suffix})", device.name)
            } else {
                device.name.clone()
            };
            let id = format!("{CHOICE_PREFIX}{}/{index}", choice.request_id);
            picker.append(&item(app, &id, &menu_text(&label))?)?;
        }
        let cancel = format!("{CHOICE_PREFIX}{}/cancel", choice.request_id);
        picker.append(&item(app, &cancel, "Cancel")?)?;
        menu.append(&picker)?;
    }
    if let Some(name) = &prompts.disconnected {
        let unavailable = Submenu::new(app, menu_text(&format!("{name} was not connected")), true)?;
        unavailable.append(&item(app, &format!("{DISMISS_PREFIX}{name}"), "Dismiss")?)?;
        menu.append(&unavailable)?;
    }
    for (index, notice) in snapshot.notices.iter().enumerate() {
        if let Some(label) = persistent_notice_text(notice) {
            append_status(app, &menu, &format!("notice/{index}"), &label)?;
        }
    }
    if prompts.choice.is_some() || prompts.disconnected.is_some() || !snapshot.notices.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    append_devices(app, &menu, "playback", "Playback devices", &snapshot.playback)?;
    if let Some(volume) = snapshot.volume {
        append_status(app, &menu, "playback-volume", &volume_text("Playback volume", volume.percent, volume.muted))?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    append_devices(app, &menu, "recording", "Recording devices", &snapshot.recording)?;
    if let Some(percent) = snapshot.mic_volume {
        let text = volume_text("Communications microphone", percent, snapshot.mic_muted.unwrap_or(false));
        append_status(app, &menu, "mic-volume", &text)?;
    }
    Ok(menu)
}

fn menu_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").replace('&', "&&")
}

fn append_status(app: &AppHandle, menu: &Menu<tauri::Wry>, id: &str, label: &str) -> tauri::Result<()> {
    menu.append(&MenuItem::with_id(app, id, menu_text(label), false, None::<&str>)?)
}

fn volume_text(device: &str, percent: u8, muted: bool) -> String {
    let status = if muted { " (muted)" } else { "" };
    format!("{device}: {percent}%{status}")
}

fn persistent_notice_text(notice: &Notice) -> Option<String> {
    match notice {
        Notice::HotkeyConflict { binding_id, keys } => Some(format!("{keys} is already in use ({binding_id})")),
        Notice::ConfigReplaced { backup, .. } => Some(format!("Invalid config. Defaults loaded. Backup: {backup}")),
        Notice::ConfigInvalid { error } => Some(format!("Config edit could not be applied: {error}")),
        Notice::ConfigSaveFailed { error } => Some(format!("Config could not be saved: {error}")),
        _ => None,
    }
}

fn append_devices(
    app: &AppHandle,
    menu: &Menu<tauri::Wry>,
    heading_id: &str,
    heading: &str,
    devices: &[DeviceSummary],
) -> tauri::Result<()> {
    append_status(app, menu, heading_id, heading)?;
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

#[cfg(test)]
mod tests {
    use ice_core::engine::{Engine, fake::FakeEngine};

    use super::*;

    #[test]
    fn picker_answers_use_the_current_request_and_offered_candidates() {
        let candidates = FakeEngine::new().snapshot().playback;
        let id = candidates[1].id.clone();
        let choice = Choice { request_id: 41, name: "Headphones".to_owned(), candidates };

        assert_eq!(choice.answer("40/1"), None);
        assert_eq!(choice.answer("41/1000"), None);
        assert_eq!(choice.answer("41/1"), Some(Command::ResolveChoice { request_id: 41, device_id: Some(id) }));
        assert_eq!(choice.answer("41/cancel"), Some(Command::ResolveChoice { request_id: 41, device_id: None }));
    }
}
