mod preferences;

use std::cell::RefCell;
use std::sync::atomic::Ordering;
use std::sync::{Arc, MutexGuard, PoisonError};

use ice_core::device::{DeviceState, Flow};
use ice_core::engine::{
    Command, DeviceSummary, EngineSnapshot, Notice, RoleSet, SavedDevice, SettingsView, SpatialMode,
};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::window::WindowBuilder;
use tauri::{AppHandle, Manager};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use super::{Preview, SharedRuntime, request_preview, startup};

const TRAY_ID: &str = "ice-audio-tray";
const COMMAND_PREFIX: &str = "audio/";
const CHOICE_PREFIX: &str = "choice/";
const DISMISS_ID: &str = "dismiss-feedback";
/// Hidden window that owns the tray popups (menus need an owner window).
const POPUP_OWNER: &str = "tray-menu-owner";

const SPATIAL_OPTIONS: [(SpatialMode, &str); 4] = [
    (SpatialMode::Keep, "Keep"),
    (SpatialMode::Off, "Off"),
    (SpatialMode::WindowsSonic, "Windows Sonic"),
    (SpatialMode::DolbyAtmosHeadphones, "Dolby Atmos for Headphones"),
];

struct RegisteredTray {
    icon: TrayIcon,
    guid: u128,
}

thread_local! {
    // The tray icon belongs to Tauri's Windows event-loop thread.
    static TRAY: RefCell<Option<RegisteredTray>> = const { RefCell::new(None) };
}

pub(super) fn close() {
    TRAY.with_borrow_mut(|tray| *tray = None);
}

fn tray_guid() -> std::io::Result<u128> {
    // Windows binds a tray GUID to an executable path. Hash it so side-by-side worktrees
    // can each register their own icon while upgrades at the same path keep their identity.
    let executable = std::env::current_exe()?;
    let path = executable.canonicalize().unwrap_or(executable);
    let hash = path
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3));
    Ok(0x4046_6f6f_327c_4f4a_8000_0000_0000_0000 | u128::from(hash & 0x3fff_ffff_ffff_ffff))
}

#[derive(Clone, Default)]
pub(super) struct Prompts {
    choice: Option<Choice>,
    feedback: Option<String>,
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
        Notice::NotConnected { name } => notify(state, "Device not connected", &format!("{name} was not connected")),
        Notice::SettingRejected { error } => notify(state, "Setting could not be saved", error),
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

fn notify(state: &SharedRuntime, title: &str, message: &str) {
    prompts(state).feedback = Some(format!("{title} · {message}"));
    let accepted = TRAY.with_borrow(|tray| {
        tray.as_ref().is_some_and(|tray| {
            ice_win::desktop::notify_tray(tray.icon.window_handle() as isize, tray.guid, title, message)
        })
    });
    tracing::debug!(title, accepted, "Windows tray notification requested");
}

pub(super) fn refresh_tooltip(state: &SharedRuntime, snapshot: &EngineSnapshot) {
    let status = {
        let prompts = prompts(state);
        if state.engine.unavailable().is_some() {
            Some("Audio unavailable")
        } else if prompts.choice.is_some() {
            Some("Left-click to choose a device")
        } else if prompts.feedback.is_some() || !snapshot.notices.is_empty() {
            Some("Left-click to see audio notices")
        } else {
            snapshot.playback.iter().find(|device| device.is_default).map(|device| device.name.as_str())
        }
    };
    let tooltip =
        status.map_or_else(|| "Ice Audio Manager".to_owned(), |status| format!("Ice Audio Manager · {status}"));
    let tooltip: String = tooltip.chars().take(60).collect();
    TRAY.with_borrow(|tray| {
        if let Some(tray) = tray {
            let _ = tray.icon.set_tooltip(Some(tooltip));
        }
    });
}

fn handle_prompt_action(state: &SharedRuntime, id: &str) -> bool {
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
            // Clearing the picker changes the tooltip even when Cancel produces no engine event.
            refresh_tooltip(state, &state.engine.snapshot());
            state.engine.send(command);
        }
    } else if id == DISMISS_ID {
        prompts(state).feedback = None;
        refresh_tooltip(state, &state.engine.snapshot());
    } else {
        return false;
    }
    true
}

pub(super) fn build(app: &AppHandle, state: Arc<SharedRuntime>) -> tauri::Result<()> {
    let popup_owner =
        WindowBuilder::new(app, POPUP_OWNER).visible(false).skip_taskbar(true).decorations(false).build()?;
    let owner_hwnd = popup_owner.hwnd()?.0 as isize;
    let launch_state = startup::enabled();
    let launch = CheckMenuItem::with_id(
        app,
        "launch",
        "Launch at sign-in",
        launch_state.is_some(),
        launch_state == Some(true),
        None::<&str>,
    )?;
    let menu_launch = launch.clone();
    let context_menu = move |app: &AppHandle, state: &SharedRuntime| {
        // Settings → General can change it too, so re-read it whenever the menu opens.
        let _ = menu_launch.set_checked(startup::enabled() == Some(true));
        let snapshot = state.engine.snapshot();
        let available = state.engine.unavailable().is_none();
        let (pinned, profiles) = preferences::placement_menus(app, state, &snapshot.settings, available)?;
        let [theme, notch_style, animation, idle, now_playing] =
            preferences::appearance_menus(app, snapshot.settings.appearance, available)?;
        Menu::with_items(
            app,
            &[
                &item(app, "settings", "Settings")?,
                &item(app, "notch", "Show notch preview")?,
                &pinned,
                &profiles,
                &theme,
                &notch_style,
                &animation,
                &idle,
                &now_playing,
                &menu_launch,
                &PredefinedMenuItem::separator(app)?,
                &item(app, "quit", "Quit Ice Audio Manager")?,
            ],
        )
    };

    let menu_state = Arc::clone(&state);
    app.on_menu_event(move |app, event| {
        let id = event.id.as_ref();
        if handle_prompt_action(&menu_state, id) {
            return;
        }
        if let Some(action) = id.strip_prefix(preferences::FAVOURITE_PREFIX) {
            if let Some(command) = preferences::edit_favourite(&menu_state.engine.snapshot(), action) {
                menu_state.engine.send(command);
            }
            return;
        }
        if let Some(command) = id.strip_prefix(COMMAND_PREFIX) {
            match serde_json::from_str::<Command>(command) {
                Ok(command) => {
                    tracing::debug!(?command, "Audio tray command");
                    menu_state.engine.send(command);
                }
                Err(error) => tracing::warn!(%error, "Invalid audio tray action"),
            }
            return;
        }
        let operation = match id {
            "settings" => {
                request_preview(app, &menu_state, Preview::Settings);
                Ok(())
            }
            "notch" => {
                request_preview(app, &menu_state, Preview::Notch);
                Ok(())
            }
            "launch" => match startup::enabled() {
                Some(current) => {
                    let result = startup::set_enabled(!current);
                    let _ = launch.set_checked(startup::enabled() == Some(true));
                    let _ = launch.set_text(if result.is_err() {
                        "Launch at sign-in (change failed)"
                    } else {
                        "Launch at sign-in"
                    });
                    result
                }
                None => Ok(()),
            },
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
    });
    app.on_tray_icon_event(move |app, event| {
        if event.id().as_ref() != TRAY_ID {
            return;
        }
        let TrayIconEvent::Click { button, button_state: MouseButtonState::Up, .. } = event else { return };
        match button {
            MouseButton::Left => open_quick_menu(app, &state),
            MouseButton::Right => {
                let result = context_menu(app, &state).and_then(|menu| popup_owner.popup_menu(&menu));
                if let Err(error) = ice_win::desktop::finish_tray_menu(owner_hwnd) {
                    tracing::debug!(%error, "Could not signal the tray popup closed");
                }
                if let Err(error) = result {
                    tracing::warn!(%error, "Could not open the tray menu");
                }
            }
            _ => {}
        }
    });

    let guid = tray_guid()?;
    let mut builder = TrayIconBuilder::new().with_id(TRAY_ID).with_guid(guid).with_tooltip("Ice Audio Manager");
    if let Some(image) = app.default_window_icon() {
        let icon = Icon::from_rgba(image.rgba().to_vec(), image.width(), image.height())
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        builder = builder.with_icon(icon);
    }
    let icon = builder.build()?;
    TRAY.with_borrow_mut(|tray| *tray = Some(RegisteredTray { icon, guid }));
    Ok(())
}

/// The left-click quick-switch menu at the cursor. Also opened by the native pill and the "show notch" hotkey in
/// `Lite`, which has no notch to show (D-19). Main thread only.
pub(super) fn open_quick_menu(app: &AppHandle, state: &SharedRuntime) {
    let Some(owner) = app.get_window(POPUP_OWNER) else {
        tracing::warn!("Could not open the quick-switch menu: tray not ready");
        return;
    };
    let visible_prompts = prompts(state).clone();
    let snapshot = state.engine.snapshot();
    tracing::debug!(revision = snapshot.revision, "Opening audio tray menu");
    let unavailable = state.engine.unavailable();
    let result =
        quick_menu(app, &snapshot, &visible_prompts, unavailable.as_deref()).and_then(|menu| owner.popup_menu(&menu));
    if let Ok(hwnd) = owner.hwnd()
        && let Err(error) = ice_win::desktop::finish_tray_menu(hwnd.0 as isize)
    {
        tracing::debug!(%error, "Could not signal the tray popup closed");
    }
    if let Err(error) = result {
        tracing::warn!(%error, "Could not open the tray menu");
    }
}

fn item(app: &AppHandle, id: &str, text: &str) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(app, id, text, true, None::<&str>)
}

fn quick_menu(
    app: &AppHandle,
    snapshot: &EngineSnapshot,
    prompts: &Prompts,
    unavailable: Option<&str>,
) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    if let Some(reason) = unavailable {
        append_status(app, &menu, "audio-unavailable", "Audio unavailable")?;
        append_status(app, &menu, "audio-unavailable-reason", reason)?;
        return Ok(menu);
    }
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
    if let Some(message) = &prompts.feedback {
        let feedback = Submenu::new(app, menu_text(message), true)?;
        feedback.append(&item(app, DISMISS_ID, "Dismiss")?)?;
        menu.append(&feedback)?;
    }
    for (index, notice) in snapshot.notices.iter().enumerate() {
        if let Some(label) = persistent_notice_text(notice) {
            append_status(app, &menu, &format!("notice/{index}"), &label)?;
        }
    }
    if prompts.choice.is_some() || prompts.feedback.is_some() || !snapshot.notices.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    append_devices(app, &menu, "playback", "Playback devices", &snapshot.playback, &snapshot.settings)?;
    preferences::append_disconnected(app, &menu, snapshot, Flow::Render)?;
    if let Some(volume) = snapshot.volume {
        append_status(app, &menu, "playback-volume", &volume_text("Playback volume", volume.percent, volume.muted))?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    append_devices(app, &menu, "recording", "Recording devices", &snapshot.recording, &snapshot.settings)?;
    preferences::append_disconnected(app, &menu, snapshot, Flow::Capture)?;
    if let Some(percent) = snapshot.mic_volume {
        let text = volume_text("Communications microphone", percent, snapshot.mic_muted.unwrap_or(false));
        append_status(app, &menu, "mic-volume", &text)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    if !snapshot.settings.toggle_pairs.is_empty() {
        let pairs = Submenu::new(app, "Toggle pairs", true)?;
        for pair in &snapshot.settings.toggle_pairs {
            let primary = saved_device_text(&pair.primary);
            let secondary = saved_device_text(&pair.secondary);
            let hotkey = pair.hotkey.as_deref().unwrap_or("No hotkey");
            let label = menu_text(&format!("{primary} ↔ {secondary} · {hotkey}"));
            pairs.append(&MenuItem::new(app, label, false, None::<&str>)?)?;
        }
        menu.append(&pairs)?;
    }
    let show = snapshot.settings.show_disconnected;
    menu.append(&checked_command(
        app,
        "Show disconnected devices",
        &Command::SetShowDisconnected { enabled: !show },
        show,
    )?)?;
    let enabled = snapshot.settings.knob_gestures;
    menu.append(&checked_command(app, "Knob gestures", &Command::SetKnobGestures { enabled: !enabled }, enabled)?)?;
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

fn command_id(command: &Command) -> tauri::Result<String> {
    Ok(format!("{COMMAND_PREFIX}{}", serde_json::to_string(command)?))
}

fn checked_command(
    app: &AppHandle,
    label: &str,
    command: &Command,
    checked: bool,
) -> tauri::Result<CheckMenuItem<tauri::Wry>> {
    CheckMenuItem::with_id(app, command_id(command)?, label, true, checked, None::<&str>)
}

fn saved_device_text(device: &SavedDevice) -> String {
    let suffix = if device.device_id.is_none() { " (not connected)" } else { "" };
    format!("{}{suffix}", device.name)
}

fn append_devices(
    app: &AppHandle,
    menu: &Menu<tauri::Wry>,
    heading_id: &str,
    heading: &str,
    devices: &[DeviceSummary],
    settings: &SettingsView,
) -> tauri::Result<()> {
    append_status(app, menu, heading_id, heading)?;
    let current = devices.iter().find(|device| device.is_default && device.state == DeviceState::Active);
    for device in devices.iter().filter(|device| device.state == DeviceState::Active) {
        let suffix = match (device.is_default, device.is_default_communications) {
            (true, true) => " (default, communications)",
            (true, false) => " (default)",
            (false, true) => " (communications)",
            (false, false) => "",
        };
        let label = menu_text(&format!("{}{suffix}", device.name));
        let device_menu = Submenu::new(app, label, true)?;
        let id = command_id(&Command::SwitchTo { device_id: device.id.clone(), roles: RoleSet::Default })?;
        device_menu.append(&CheckMenuItem::with_id(app, id, "Make default", true, device.is_default, None::<&str>)?)?;
        if let Some(current) = current.filter(|current| current.id != device.id) {
            let id = command_id(&Command::SetTogglePair {
                pair_id: "main".into(),
                primary_id: device.id.clone(),
                secondary_id: current.id.clone(),
                hotkey: Some("Ctrl+Alt+F12".into()),
                roles: None,
            })?;
            device_menu.append(&item(app, &id, "Toggle with the current device…")?)?;
        }
        if device.flow == Flow::Render {
            device_menu.append(&preferences::favourite_item(app, settings, device)?)?;
            let preference = settings
                .devices
                .iter()
                .find(|pref| pref.device.device_id.as_deref() == Some(&device.id))
                .map_or(SpatialMode::Keep, |pref| pref.spatial);
            let spatial_menu = Submenu::new(app, "Spatial sound", true)?;
            for (spatial, text) in SPATIAL_OPTIONS {
                let id = command_id(&Command::SetDevicePrefs {
                    device_id: device.id.clone(),
                    spatial: Some(spatial),
                    alias: None,
                    hidden: None,
                })?;
                spatial_menu.append(&CheckMenuItem::with_id(
                    app,
                    id,
                    text,
                    true,
                    spatial == preference,
                    None::<&str>,
                )?)?;
            }
            device_menu.append(&spatial_menu)?;
        }
        menu.append(&device_menu)?;
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

    #[test]
    fn native_device_menu_updates_pair_spatial_and_knob_settings() {
        let app = tauri::Builder::default().any_thread().build(tauri::generate_context!()).unwrap();
        let engine = FakeEngine::new();
        let snapshot = engine.snapshot();
        let default = snapshot.playback.iter().find(|device| device.is_default).unwrap();
        let other = snapshot.playback.iter().find(|device| !device.is_default).unwrap();
        let menu = quick_menu(app.handle(), &snapshot, &Prompts::default(), None).unwrap();
        let items = menu.items().unwrap();
        let device_menu = items
            .iter()
            .filter_map(|item| item.as_submenu())
            .find(|item| item.text().unwrap() == menu_text(&other.name))
            .unwrap();
        let actions = device_menu.items().unwrap();
        let pair = actions
            .iter()
            .filter_map(|item| item.as_menuitem())
            .find(|item| item.text().unwrap() == "Toggle with the current device…")
            .unwrap();
        let command =
            serde_json::from_str::<Command>(pair.id().as_ref().strip_prefix(COMMAND_PREFIX).unwrap()).unwrap();
        assert_eq!(
            command,
            Command::SetTogglePair {
                pair_id: "main".into(),
                primary_id: other.id.clone(),
                secondary_id: default.id.clone(),
                hotkey: Some("Ctrl+Alt+F12".into()),
                roles: None,
            }
        );
        engine.send(command);

        let spatial = actions
            .iter()
            .filter_map(|item| item.as_submenu())
            .find(|item| item.text().unwrap() == "Spatial sound")
            .unwrap();
        let options = spatial.items().unwrap();
        let off = options
            .iter()
            .filter_map(|item| item.as_check_menuitem())
            .find(|item| item.text().unwrap() == "Off")
            .unwrap();
        let command = serde_json::from_str::<Command>(off.id().as_ref().strip_prefix(COMMAND_PREFIX).unwrap()).unwrap();
        engine.send(command);

        let knob = items
            .iter()
            .filter_map(|item| item.as_check_menuitem())
            .find(|item| item.text().unwrap() == "Knob gestures")
            .unwrap();
        assert_eq!(knob.is_checked().unwrap(), snapshot.settings.knob_gestures);
        let command =
            serde_json::from_str::<Command>(knob.id().as_ref().strip_prefix(COMMAND_PREFIX).unwrap()).unwrap();
        engine.send(command);

        let settings = engine.snapshot().settings;
        let saved_pair = settings.toggle_pairs.iter().find(|pair| pair.id == "main").unwrap();
        assert_eq!(saved_pair.primary.device_id.as_ref(), Some(&other.id));
        assert_eq!(saved_pair.secondary.device_id.as_ref(), Some(&default.id));
        assert_eq!(saved_pair.hotkey.as_deref(), Some("Ctrl+Alt+F12"));
        assert_eq!(settings.knob_gestures, !snapshot.settings.knob_gestures);
        assert!(
            settings
                .devices
                .iter()
                .any(|pref| pref.device.device_id.as_ref() == Some(&other.id) && pref.spatial == SpatialMode::Off)
        );
    }
}
