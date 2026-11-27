//! Authenticated localhost JSON-line bridge (`127.0.0.1:0`) for companion tools such as Pandora.
//!
//! Discovery metadata (`{ version, port, token, pid }`) is written atomically to
//! `%LOCALAPPDATA%\IceAudioManager\bridge.json` on startup and removed cleanly on exit. If nothing
//! connects, the listener thread sleeps blocked in `TcpListener::accept` with zero CPU usage.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use ice_core::companion::{DiscoveryFile, new_token, tokens_match};
use ice_core::engine::{Command, EngineSnapshot, PerformanceProfile};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::{SharedRuntime, settings, show_notch};

const BRIDGE_VERSION: u32 = ice_core::companion::PROTOCOL_VERSION;
const MAX_REQUEST_BYTES: u64 = ice_core::companion::MAX_LINE_BYTES;
const IO_TIMEOUT: Duration = Duration::from_secs(2);

static ACTIVE: Mutex<Option<BridgeStop>> = Mutex::new(None);

struct BridgeStop {
    stop: Arc<AtomicBool>,
    port: u16,
    info_path: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeRequest {
    token: String,
    #[serde(flatten)]
    action: BridgeAction,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum BridgeAction {
    Ping,
    Snapshot,
    Command {
        command: Command,
    },
    ShowNotch,
    ToggleNotch,
    /// Wake from hibernation, open the card, close it after `ms` (default 2 s) unless the user took it over.
    PeekNotch {
        #[serde(default = "default_peek_ms")]
        ms: u64,
    },
    OpenSettings,
}

const fn default_peek_ms() -> u64 {
    2_000
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BridgeStatus {
    version: u32,
    app_version: String,
    pid: u32,
    notch_expanded: bool,
    hibernate: bool,
    gaming: bool,
    performance_profile: String,
    active_device: Option<String>,
    volume_percent: Option<u8>,
    muted: Option<bool>,
    mic_muted: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BridgeResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<BridgeStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<EngineSnapshot>,
}

impl BridgeResponse {
    fn err(msg: impl Into<String>) -> Self {
        Self { ok: false, error: Some(msg.into()), status: None, snapshot: None }
    }

    fn ok_status(status: BridgeStatus, snapshot: Option<EngineSnapshot>) -> Self {
        Self { ok: true, error: None, status: Some(status), snapshot }
    }
}

pub(super) fn start(app: &AppHandle, state: &Arc<SharedRuntime>, info_path: PathBuf) {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(error) => {
            tracing::warn!(%error, "Ice local bridge failed to bind");
            return;
        }
    };
    let Ok(addr) = listener.local_addr() else {
        tracing::warn!("Ice local bridge could not read local address");
        return;
    };
    let port = addr.port();
    let token = new_token();
    let info = DiscoveryFile { version: BRIDGE_VERSION, port, token: token.clone(), pid: std::process::id() };
    if let Err(error) = write_json_atomic(&info_path, &info) {
        tracing::warn!(%error, path = %info_path.display(), "Ice local bridge could not write discovery file");
    }

    let stop = Arc::new(AtomicBool::new(false));
    {
        let mut active = ACTIVE.lock().unwrap_or_else(PoisonError::into_inner);
        *active = Some(BridgeStop { stop: Arc::clone(&stop), port, info_path });
    }

    let app_handle = app.clone();
    let runtime = Arc::clone(state);
    if let Err(error) = std::thread::Builder::new().name("ice-local-bridge".into()).spawn(move || {
        for conn in listener.incoming() {
            if stop.load(Ordering::Acquire) {
                break;
            }
            match conn {
                Ok(stream) => {
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    handle_client(stream, &app_handle, &runtime, &token);
                }
                Err(error) => {
                    tracing::debug!(%error, "Ice local bridge accept error");
                    break;
                }
            }
        }
    }) {
        tracing::warn!(%error, "Ice local bridge thread failed to spawn");
        return;
    }

    tracing::info!(port, "Ice local bridge ready");
}

pub(super) fn stop() {
    let prev = ACTIVE.lock().unwrap_or_else(PoisonError::into_inner).take();
    let Some(entry) = prev else {
        return;
    };
    entry.stop.store(true, Ordering::Release);
    let _ = std::fs::remove_file(&entry.info_path);
    let addr = SocketAddr::from(([127, 0, 0, 1], entry.port));
    let _ = TcpStream::connect_timeout(&addr, Duration::from_millis(100));
}

fn handle_client(mut stream: TcpStream, app: &AppHandle, state: &Arc<SharedRuntime>, expected_token: &str) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));

    let mut line = String::new();
    let read_res = {
        let mut limited = BufReader::new(Read::by_ref(&mut stream).take(MAX_REQUEST_BYTES));
        limited.read_line(&mut line)
    };
    let response = match read_res {
        Ok(0) => return,
        Ok(_) => match serde_json::from_str::<BridgeRequest>(line.trim_end()) {
            Ok(req) => dispatch_request(req, app, state, expected_token),
            Err(e) => BridgeResponse::err(format!("invalid request: {e}")),
        },
        Err(e) => BridgeResponse::err(format!("read error: {e}")),
    };

    if let Ok(mut json) = serde_json::to_string(&response) {
        json.push('\n');
        let _ = stream.write_all(json.as_bytes());
        let _ = stream.flush();
    }
}

fn dispatch_request(
    req: BridgeRequest,
    app: &AppHandle,
    state: &Arc<SharedRuntime>,
    expected_token: &str,
) -> BridgeResponse {
    if !tokens_match(&req.token, expected_token) {
        return BridgeResponse::err("unauthorized");
    }

    match req.action {
        BridgeAction::Ping => {
            let snap = state.engine.snapshot();
            BridgeResponse::ok_status(build_status(state, &snap), None)
        }
        BridgeAction::Snapshot => {
            let snap = state.engine.snapshot();
            BridgeResponse::ok_status(build_status(state, &snap), Some(snap))
        }
        BridgeAction::Command { command } => {
            state.engine.send(command);
            std::thread::sleep(Duration::from_millis(25));
            let snap = state.engine.snapshot();
            BridgeResponse::ok_status(build_status(state, &snap), Some(snap))
        }
        BridgeAction::ShowNotch => {
            let (h, st) = (app.clone(), Arc::clone(state));
            let _ = app.run_on_main_thread(move || show_notch::show(&h, &st, show_notch::Open::Show));
            let snap = state.engine.snapshot();
            BridgeResponse::ok_status(build_status(state, &snap), None)
        }
        BridgeAction::ToggleNotch => {
            let (h, st) = (app.clone(), Arc::clone(state));
            let _ = app.run_on_main_thread(move || show_notch::show(&h, &st, show_notch::Open::Toggle));
            let snap = state.engine.snapshot();
            BridgeResponse::ok_status(build_status(state, &snap), None)
        }
        BridgeAction::PeekNotch { ms } => {
            let (h, st) = (app.clone(), Arc::clone(state));
            let _ = app.run_on_main_thread(move || show_notch::peek(&h, &st, ms));
            let snap = state.engine.snapshot();
            BridgeResponse::ok_status(build_status(state, &snap), None)
        }
        BridgeAction::OpenSettings => {
            let (h, st) = (app.clone(), Arc::clone(state));
            let _ = app.run_on_main_thread(move || {
                let _ = settings::create_window(&h, &st);
            });
            let snap = state.engine.snapshot();
            BridgeResponse::ok_status(build_status(state, &snap), None)
        }
    }
}

fn build_status(state: &SharedRuntime, snap: &EngineSnapshot) -> BridgeStatus {
    let notch_expanded = state.snapshot_clone().expanded;
    let gaming = state.placement().gaming;
    let active_device = snap.playback.iter().find(|d| d.is_default).map(|d| d.name.clone());
    let profile = match snap.settings.performance_profile {
        PerformanceProfile::Instant => "instant",
        PerformanceProfile::Balanced => "balanced",
        PerformanceProfile::Lite => "lite",
    };

    BridgeStatus {
        version: BRIDGE_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        pid: std::process::id(),
        notch_expanded,
        hibernate: snap.settings.appearance.hibernate,
        gaming,
        performance_profile: profile.to_string(),
        active_device,
        volume_percent: snap.volume.as_ref().map(|v| v.percent),
        muted: snap.volume.as_ref().map(|v| v.muted),
        mic_muted: snap.mic_muted,
    }
}

/// Writes `value` as pretty JSON through a temp file and a rename, so a reader never sees half a file.
pub(super) fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json)?;
    if std::fs::rename(&tmp, path).is_err() {
        std::fs::write(path, &json)?;
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_deserializes_all_bridge_actions() {
        let ping: BridgeRequest = serde_json::from_str(r#"{"token":"abc","action":"ping"}"#).unwrap();
        assert!(matches!(ping.action, BridgeAction::Ping));

        let snap: BridgeRequest = serde_json::from_str(r#"{"token":"abc","action":"snapshot"}"#).unwrap();
        assert!(matches!(snap.action, BridgeAction::Snapshot));

        let show: BridgeRequest = serde_json::from_str(r#"{"token":"abc","action":"showNotch"}"#).unwrap();
        assert!(matches!(show.action, BridgeAction::ShowNotch));

        let peek: BridgeRequest = serde_json::from_str(r#"{"token":"abc","action":"peekNotch","ms":1500}"#).unwrap();
        assert!(matches!(peek.action, BridgeAction::PeekNotch { ms: 1500 }));
        let peek_default: BridgeRequest = serde_json::from_str(r#"{"token":"abc","action":"peekNotch"}"#).unwrap();
        assert!(matches!(peek_default.action, BridgeAction::PeekNotch { ms: 2_000 }));

        let toggle: BridgeRequest = serde_json::from_str(r#"{"token":"abc","action":"toggleNotch"}"#).unwrap();
        assert!(matches!(toggle.action, BridgeAction::ToggleNotch));

        let settings: BridgeRequest = serde_json::from_str(r#"{"token":"abc","action":"openSettings"}"#).unwrap();
        assert!(matches!(settings.action, BridgeAction::OpenSettings));

        let cmd: BridgeRequest =
            serde_json::from_str(r#"{"token":"abc","action":"command","command":{"type":"toggleMute"}}"#).unwrap();
        assert!(matches!(cmd.action, BridgeAction::Command { command: Command::ToggleMute }));
    }
}
