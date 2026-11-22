//! Now playing via Global System Media Transport Controls (GSMTC). Phase 0.4 / Phase 3.

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::core::Result;

pub use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as MediaManager;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NowPlaying {
    pub app: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub status: &'static str,
    /// Size of the album-art stream in bytes, if the app provides one.
    pub thumbnail_bytes: Option<u64>,
}

/// Requests the session manager (blocks until Windows answers).
///
/// # Errors
/// Fails if GSMTC is unavailable.
pub fn manager() -> Result<Manager> {
    Manager::RequestAsync()?.join()
}

/// The session Windows considers current, if any.
///
/// # Errors
/// Fails if reading the session's properties fails.
pub fn current(mgr: &Manager) -> Result<Option<NowPlaying>> {
    match mgr.GetCurrentSession() {
        Ok(s) => describe(&s).map(Some),
        Err(_) => Ok(None),
    }
}

/// Every media session Windows knows about.
///
/// # Errors
/// Fails if the session list can't be read.
pub fn sessions(mgr: &Manager) -> Result<Vec<NowPlaying>> {
    mgr.GetSessions()?.into_iter().map(|s| describe(&s)).collect()
}

/// Toggles play/pause on the current session.
///
/// # Errors
/// Fails if there is no current session or the app rejects the command.
pub fn toggle_play_pause(mgr: &Manager) -> Result<bool> {
    mgr.GetCurrentSession()?.TryTogglePlayPauseAsync()?.join()
}

fn describe(s: &Session) -> Result<NowPlaying> {
    let props = s.TryGetMediaPropertiesAsync()?.join()?;
    let thumbnail_bytes = props
        .Thumbnail()
        .ok()
        .and_then(|t| t.OpenReadAsync().ok())
        .and_then(|op| op.join().ok())
        .and_then(|stream| stream.Size().ok());
    let status = match s.GetPlaybackInfo()?.PlaybackStatus()? {
        st if st == Status::Playing => "playing",
        st if st == Status::Paused => "paused",
        st if st == Status::Stopped => "stopped",
        st if st == Status::Changing => "changing",
        st if st == Status::Opened => "opened",
        _ => "closed",
    };
    Ok(NowPlaying {
        app: s.SourceAppUserModelId()?.to_string(),
        title: props.Title()?.to_string(),
        artist: props.Artist()?.to_string(),
        album: props.AlbumTitle()?.to_string(),
        status,
        thumbnail_bytes,
    })
}
