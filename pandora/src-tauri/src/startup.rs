//! Launch at sign-in, shared by Settings → Playback and the tray check item (same registry value Ice uses for itself,
//! under Pandora's own name). Read from the registry each time so the two never disagree.
//!
//! Only release builds may change it: a debug build would register `target\debug\pandora.exe`, which then starts at
//! every sign-in until someone notices.

use ice_win::autostart::{Autostart, StartupEntry};
use serde::Serialize;

/// Starts in the tray, like a start from Ice (`--background`).
const PANDORA_STARTUP: StartupEntry = StartupEntry { value: "Pandora", args: ice_core::companion::BACKGROUND_ARG };

/// `true` in debug builds, where changing it is refused.
pub const LOCKED_IN_DEBUG: bool = cfg!(debug_assertions);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchAtSignIn {
    /// Whether Pandora's Run value exists (`false` when unreadable).
    pub enabled: bool,
    /// Whether this build may change it.
    pub available: bool,
    /// Why it can't be changed, for the UI.
    pub reason: Option<String>,
}

pub fn state() -> LaunchAtSignIn {
    let read = Autostart::current().and_then(|startup| startup.enabled_as(PANDORA_STARTUP));
    let (enabled, mut reason) = match read {
        Ok(Some(enabled)) => (enabled, None),
        Ok(None) => (false, Some("Windows manages startup for this version (Settings → Apps → Startup).".to_string())),
        Err(error) => {
            tracing::warn!(%error, "Could not read Pandora's launch-at-sign-in state");
            (false, Some("Couldn’t read the Windows startup setting.".to_string()))
        }
    };
    if LOCKED_IN_DEBUG {
        reason = Some("Only release builds can launch at sign-in, so a debug build never registers itself.".into());
    }
    LaunchAtSignIn { enabled, available: reason.is_none(), reason }
}

pub fn set(enabled: bool) -> Result<LaunchAtSignIn, String> {
    if LOCKED_IN_DEBUG {
        return Err("Launch at sign-in is locked in debug builds.".into());
    }
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    Autostart::current()
        .and_then(|startup| startup.set_enabled_as(PANDORA_STARTUP, enabled, &exe))
        .map_err(|error| error.to_string())?;
    Ok(state())
}
