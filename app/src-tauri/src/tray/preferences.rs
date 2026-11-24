use std::sync::PoisonError;

use ice_core::appearance::{Appearance, IdleStyle, NowPlayingPill, SpringPreset, ThemeKind};
use ice_core::device::Flow;
use ice_core::engine::{Command, DeviceSummary, EngineSnapshot, PerformanceProfile, SavedDevice, SettingsView};
use tauri::AppHandle;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, Submenu};

use super::{SharedRuntime, checked_command, menu_text};

pub(super) const FAVOURITE_PREFIX: &str = "favourite/";

fn matches_device(saved: &SavedDevice, id: &str) -> bool {
    saved.device_id.as_ref().is_some_and(|live| live.eq_ignore_ascii_case(id))
}

pub(super) fn favourite_item(
    app: &AppHandle,
    settings: &SettingsView,
    device: &DeviceSummary,
) -> tauri::Result<CheckMenuItem<tauri::Wry>> {
    let checked = settings.favourites_playback.iter().any(|saved| matches_device(saved, &device.id));
    let action = if checked { "remove" } else { "add" };
    let id = format!("{FAVOURITE_PREFIX}{action}/{}", device.id);
    CheckMenuItem::with_id(app, id, "Show in notch", true, checked, None::<&str>)
}

// Re-read favourites when the click arrives. A config reload may have reordered them while the popup was open.
pub(super) fn edit_favourite(snapshot: &EngineSnapshot, action: &str) -> Option<Command> {
    let (operation, id) = action.split_once('/')?;
    let target = snapshot.playback.iter().find(|device| device.id.eq_ignore_ascii_case(id))?;
    let saved = &snapshot.settings.favourites_playback;
    let included = saved.iter().any(|device| matches_device(device, id));
    let mut device_ids: Vec<_> =
        saved.iter().filter(|device| !matches_device(device, id)).map(|device| device.saved_id.clone()).collect();
    match (operation, included) {
        ("add", false) => device_ids.push(target.id.clone()),
        ("remove", true) => {}
        _ => return None,
    }
    Some(Command::SetFavourites { flow: Flow::Render, device_ids })
}

pub(super) fn append_disconnected(
    app: &AppHandle,
    menu: &Menu<tauri::Wry>,
    snapshot: &EngineSnapshot,
    flow: Flow,
) -> tauri::Result<()> {
    if snapshot.settings.show_disconnected {
        for device in snapshot.disconnected.iter().filter(|device| device.flow == flow) {
            let label = menu_text(&format!("{} (disconnected)", device.name));
            menu.append(&MenuItem::new(app, label, false, None::<&str>)?)?;
        }
    }
    Ok(())
}

fn choices(
    app: &AppHandle,
    title: &str,
    enabled: bool,
    current: Command,
    options: &[(&str, Command)],
) -> tauri::Result<Submenu<tauri::Wry>> {
    let menu = Submenu::new(app, title, enabled)?;
    for (label, command) in options {
        menu.append(&checked_command(app, label, command, command == &current)?)?;
    }
    Ok(menu)
}

pub(super) fn appearance_menus(
    app: &AppHandle,
    current: Appearance,
    available: bool,
) -> tauri::Result<[Submenu<tauri::Wry>; 4]> {
    use Command::{SetIdleStyle, SetNowPlayingPill, SetPreset, SetTheme};
    Ok([
        choices(
            app,
            "Theme",
            available,
            SetTheme { theme: current.theme },
            &[
                ("Ice", SetTheme { theme: ThemeKind::Ice }),
                ("Midnight", SetTheme { theme: ThemeKind::Midnight }),
                ("Fluent", SetTheme { theme: ThemeKind::Fluent }),
                ("Minimal", SetTheme { theme: ThemeKind::Minimal }),
            ],
        )?,
        choices(
            app,
            "Animation",
            available,
            SetPreset { preset: current.preset },
            &[
                ("Snappy", SetPreset { preset: SpringPreset::Snappy }),
                ("Smooth", SetPreset { preset: SpringPreset::Smooth }),
                ("Bouncy", SetPreset { preset: SpringPreset::Bouncy }),
                ("Off", SetPreset { preset: SpringPreset::Off }),
            ],
        )?,
        choices(
            app,
            "Idle notch",
            available,
            SetIdleStyle { style: current.idle_style },
            &[
                ("Pill", SetIdleStyle { style: IdleStyle::MiniPill }),
                ("Dot", SetIdleStyle { style: IdleStyle::Dot }),
                ("Hidden", SetIdleStyle { style: IdleStyle::Hidden }),
            ],
        )?,
        choices(
            app,
            "Now playing pill",
            available,
            SetNowPlayingPill { mode: current.now_playing },
            &[
                ("Brief, then small (default)", SetNowPlayingPill { mode: NowPlayingPill::Brief }),
                ("While playing", SetNowPlayingPill { mode: NowPlayingPill::WhilePlaying }),
                ("Off", SetNowPlayingPill { mode: NowPlayingPill::Off }),
            ],
        )?,
    ])
}

pub(super) fn placement_menus(
    app: &AppHandle,
    state: &SharedRuntime,
    settings: &SettingsView,
    available: bool,
) -> tauri::Result<(Submenu<tauri::Wry>, Submenu<tauri::Wry>)> {
    use Command::SetPerformanceProfile;
    let profiles = choices(
        app,
        "Performance profile",
        available,
        SetPerformanceProfile { profile: settings.performance_profile },
        &[
            ("Instant", SetPerformanceProfile { profile: PerformanceProfile::Instant }),
            ("Balanced", SetPerformanceProfile { profile: PerformanceProfile::Balanced }),
            ("Lite", SetPerformanceProfile { profile: PerformanceProfile::Lite }),
        ],
    )?;
    let pinned = Submenu::new(app, "Pinned monitor", available)?;
    for m in state.monitors.lock().unwrap_or_else(PoisonError::into_inner).iter() {
        let selected = settings.pinned_monitor.as_ref().is_some_and(|pin| pin.eq_ignore_ascii_case(&m.display_path));
        pinned.append(&checked_command(
            app,
            &menu_text(&m.friendly_name),
            &Command::SetPinnedMonitor { display_path: m.display_path.clone() },
            selected,
        )?)?;
    }
    Ok((pinned, profiles))
}
