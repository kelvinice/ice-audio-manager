//! Launch at sign-in, shared by the tray check item and Settings → General.
//!
//! Read from the registry each time instead of cached, so the tray and Settings never disagree.

use ice_win::autostart::Autostart;

/// `Some(enabled)`, or `None` when this build can't change it (packaged: the MSIX `StartupTask` owns it) or the
/// state is unreadable.
pub(super) fn enabled() -> Option<bool> {
    match Autostart::current().and_then(Autostart::enabled) {
        Ok(state) => state,
        Err(error) => {
            tracing::warn!(%error, "Could not read launch-at-sign-in state");
            None
        }
    }
}

pub(super) fn set_enabled(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    Autostart::current().and_then(|startup| startup.set_enabled(enabled, &exe)).map_err(|error| error.to_string())
}
