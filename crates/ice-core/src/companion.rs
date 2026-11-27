//! The companion protocol between Ice and the experimental Pandora player (owner, 2026-11-27). Types only: each app does
//! its own I/O.
//!
//! - **Finding each other:** each app keeps `install.json` ([`InstallHint`]) in its `%LOCALAPPDATA%` folder, rewritten
//!   at every start and kept after exit, so the other app can launch it.
//! - **Talking:** while running, an app that accepts requests writes `bridge.json` ([`DiscoveryFile`]: loopback port +
//!   per-run token) and listens on `127.0.0.1` for one JSON line per connection, answering with one JSON line. Ice's
//!   listener lives in `app/src-tauri/src/bridge.rs`; Pandora's in `pandora/src-tauri/src/companion.rs`.
//! - Nothing leaves the PC: both ends bind loopback only, and every request carries the token from the discovery file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Bumped when a request or reply changes incompatibly.
pub const PROTOCOL_VERSION: u32 = 1;
/// Ice's folder under `%LOCALAPPDATA%`.
pub const ICE_DIR: &str = "IceAudioManager";
/// Pandora's folder under `%LOCALAPPDATA%`.
pub const PANDORA_DIR: &str = "Pandora";
/// Written while the app runs and deleted on exit.
pub const DISCOVERY_FILE: &str = "bridge.json";
/// Kept after exit so the other app can start this one.
pub const INSTALL_FILE: &str = "install.json";
/// Ice and Pandora both accept requests up to this size.
pub const MAX_LINE_BYTES: u64 = 65_536;
/// Pandora started by Ice for a background job (recognition) keeps its window hidden.
pub const BACKGROUND_ARG: &str = "--background";

/// `bridge.json`: where a running app listens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryFile {
    pub version: u32,
    pub port: u16,
    pub token: String,
    pub pid: u32,
}

/// `install.json`: how to start the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallHint {
    pub version: u32,
    pub exe_path: String,
    pub app_version: String,
}

impl InstallHint {
    /// The executable, if the hint names an absolute `.exe` path (the caller still checks that it exists).
    #[must_use]
    pub fn exe(&self) -> Option<PathBuf> {
        let path = Path::new(&self.exe_path);
        let is_exe = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"));
        (path.is_absolute() && is_exe).then(|| path.to_path_buf())
    }
}

/// What Ice may ask Pandora to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum PandoraRequest {
    /// Is Pandora there? Answers with its version.
    Ping,
    /// Bring Pandora's window to the front.
    Show,
    /// Listen to the PC's playback (WASAPI loopback, never the microphone) and name the song. Blocks up to ~12 s.
    Recognize,
}

/// One request line: the per-run token plus the request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PandoraEnvelope {
    pub token: String,
    #[serde(flatten)]
    pub request: PandoraRequest,
}

/// A song Pandora recognised.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognizedSong {
    pub title: String,
    pub artist: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
}

/// One reply line.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PandoraReply {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub song: Option<RecognizedSong>,
}

impl PandoraReply {
    #[must_use]
    pub fn failed(error: impl Into<String>) -> Self {
        Self { ok: false, error: Some(error.into()), ..Self::default() }
    }
}

/// Notch → shell: a Pandora button in the widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum PandoraAction {
    Recognize,
    Open,
}

/// A fresh per-run token: 32 hex characters from two randomly keyed std hashers (no extra dependency). It only has to
/// keep other local processes from guessing it during one run.
#[must_use]
pub fn new_token() -> String {
    use std::hash::BuildHasher;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let pid = u128::from(std::process::id());
    let hi = std::collections::hash_map::RandomState::new().hash_one((nanos, pid, 0x9e37_79b9_7f4a_7c15_u64));
    let lo =
        std::collections::hash_map::RandomState::new().hash_one((pid, nanos.wrapping_mul(6_364_136_223_846_793), hi));
    format!("{hi:016x}{lo:016x}")
}

/// Compares a request token with the expected one without stopping at the first different byte.
#[must_use]
pub fn tokens_match(candidate: &str, expected: &str) -> bool {
    let (a, b) = (candidate.as_bytes(), expected.as_bytes());
    if a.len() != b.len() || b.is_empty() {
        return false;
    }
    a.iter().zip(b).fold(0_u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trips_with_a_flat_action_tag() {
        let line = r#"{"token":"abc","action":"recognize"}"#;
        let env: PandoraEnvelope = serde_json::from_str(line).unwrap();
        assert_eq!(env.request, PandoraRequest::Recognize);
        assert_eq!(serde_json::to_string(&env).unwrap(), line);
    }

    #[test]
    fn reply_omits_empty_fields() {
        let reply = PandoraReply::failed("busy");
        assert_eq!(serde_json::to_string(&reply).unwrap(), r#"{"ok":false,"error":"busy"}"#);
        let back: PandoraReply = serde_json::from_str(r#"{"ok":true}"#).unwrap();
        assert!(back.ok && back.song.is_none());
    }

    #[test]
    fn install_hint_needs_an_absolute_exe() {
        let hint = |p: &str| InstallHint { version: 1, exe_path: p.into(), app_version: "0".into() };
        assert!(hint(r"C:\Apps\Pandora\pandora.exe").exe().is_some());
        assert!(hint(r"C:\Apps\Pandora\PANDORA.EXE").exe().is_some());
        assert!(hint(r"pandora.exe").exe().is_none());
        assert!(hint(r"C:\Apps\Pandora\pandora.bat").exe().is_none());
    }

    #[test]
    fn new_tokens_are_32_hex_chars_and_differ() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 32);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_ne!(a, b);
    }

    #[test]
    fn tokens_must_match_exactly_and_never_be_empty() {
        assert!(tokens_match("deadbeef", "deadbeef"));
        assert!(!tokens_match("deadbeee", "deadbeef"));
        assert!(!tokens_match("dead", "deadbeef"));
        assert!(!tokens_match("", ""));
    }
}
