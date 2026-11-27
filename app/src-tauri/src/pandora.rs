//! Pandora companion (experimental, owner 2026-11-27): the notch widget's Pandora buttons.
//!
//! Finds Pandora through its `install.json`, starts it when it isn't running, and sends one request over Pandora's
//! loopback bridge (`ice_core::companion`). One job at a time, on a short-lived `ice-pandora` thread that ends with the
//! job. Nothing here talks to the network: Pandora does the recognition lookup itself. Also writes Ice's own
//! `install.json`, so Pandora can start Ice.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ice_core::companion::{
    self as proto, DiscoveryFile, InstallHint, PandoraAction, PandoraEnvelope, PandoraReply, PandoraRequest,
    RecognizedSong,
};
use tauri::{AppHandle, State};

use super::{SharedRuntime, bridge, tray};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(400);
const QUICK_TIMEOUT: Duration = Duration::from_secs(3);
/// Pandora listens for up to ~10 s and then looks the fingerprint up.
const RECOGNIZE_TIMEOUT: Duration = Duration::from_secs(30);
/// How long a freshly started Pandora gets to open its bridge, checked every `LAUNCH_POLL`.
const LAUNCH_WAIT: Duration = Duration::from_secs(15);
const LAUNCH_POLL: Duration = Duration::from_millis(250);

/// Set while an `ice-pandora` job runs; a second press meanwhile is refused.
static BUSY: AtomicBool = AtomicBool::new(false);

/// Records where this Ice lives, for Pandora's "Launch Ice" (`%LOCALAPPDATA%\IceAudioManager\install.json`).
pub(super) fn write_install_hint(app_data: &Path) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let hint = InstallHint {
        version: proto::PROTOCOL_VERSION,
        exe_path: exe.display().to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    if let Err(error) = bridge::write_json_atomic(&app_data.join(proto::INSTALL_FILE), &hint) {
        tracing::debug!(%error, "Could not write Ice's install hint");
    }
}

#[tauri::command]
pub(super) fn pandora_action(
    window: tauri::WebviewWindow,
    action: PandoraAction,
    app: AppHandle,
    state: State<'_, Arc<SharedRuntime>>,
) -> Result<(), String> {
    if window.label() != "notch" {
        return Err("Only the notch can use the Pandora buttons".into());
    }
    start(&app, &state, action)
}

fn start(app: &AppHandle, state: &Arc<SharedRuntime>, action: PandoraAction) -> Result<(), String> {
    if BUSY.swap(true, Ordering::AcqRel) {
        return Err("Pandora is still busy".into());
    }
    state.update_and_emit(app, |s| s.pandora.busy = Some(action));
    let (job_app, job_state) = (app.clone(), Arc::clone(state));
    let spawned = std::thread::Builder::new().name("ice-pandora".into()).spawn(move || {
        let outcome = run(action);
        BUSY.store(false, Ordering::Release);
        finish(&job_app, &job_state, action, outcome);
    });
    spawned.map(|_| ()).map_err(|error| {
        BUSY.store(false, Ordering::Release);
        state.update_and_emit(app, |s| s.pandora.busy = None);
        error.to_string()
    })
}

fn run(action: PandoraAction) -> Result<Option<RecognizedSong>, String> {
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(|d| PathBuf::from(d).join(proto::PANDORA_DIR))
        .ok_or("LOCALAPPDATA is unavailable")?;
    let (request, timeout, background) = match action {
        PandoraAction::Recognize => (PandoraRequest::Recognize, RECOGNIZE_TIMEOUT, true),
        PandoraAction::Open => (PandoraRequest::Show, QUICK_TIMEOUT, false),
    };
    let info = connect_or_launch(&dir, background)?;
    let reply = call(&info, request, timeout)?;
    if !reply.ok {
        return Err(reply.error.unwrap_or_else(|| "Pandora couldn't do that".into()));
    }
    Ok(reply.song)
}

fn connect_or_launch(dir: &Path, background: bool) -> Result<DiscoveryFile, String> {
    if let Some(info) = live_bridge(dir) {
        return Ok(info);
    }
    let exe =
        find_pandora_exe(dir).ok_or("Pandora isn't installed, or hasn't run yet. Start it once so Ice can find it.")?;
    let mut command = std::process::Command::new(&exe);
    if background {
        command.arg(proto::BACKGROUND_ARG);
    }
    command.spawn().map_err(|e| format!("Could not start Pandora: {e}"))?;
    let deadline = Instant::now() + LAUNCH_WAIT;
    while Instant::now() < deadline {
        std::thread::sleep(LAUNCH_POLL);
        if let Some(info) = live_bridge(dir) {
            return Ok(info);
        }
    }
    Err("Pandora started but didn't answer in time.".into())
}

/// Pandora's discovery file, if Pandora answers a ping on it (a crash can leave a stale file behind).
fn live_bridge(dir: &Path) -> Option<DiscoveryFile> {
    let bytes = std::fs::read(dir.join(proto::DISCOVERY_FILE)).ok()?;
    let info: DiscoveryFile = serde_json::from_slice(&bytes).ok()?;
    call(&info, PandoraRequest::Ping, QUICK_TIMEOUT).ok().filter(|r| r.ok).map(|_| info)
}

fn find_pandora_exe(dir: &Path) -> Option<PathBuf> {
    let hinted = std::fs::read(dir.join(proto::INSTALL_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<InstallHint>(&bytes).ok())
        .and_then(|hint| hint.exe());
    hinted.into_iter().chain(checkout_candidates()).find(|p| p.is_file())
}

/// A source checkout before Pandora's first run: Ice in `target\<profile>`, Pandora in `pandora\src-tauri\target`.
fn checkout_candidates() -> Vec<PathBuf> {
    let Ok(exe) = std::env::current_exe() else {
        return Vec::new();
    };
    exe.ancestors()
        .take(4)
        .flat_map(|root| {
            ["release", "debug"]
                .map(|profile| root.join("pandora").join("src-tauri").join("target").join(profile).join("pandora.exe"))
        })
        .collect()
}

fn call(info: &DiscoveryFile, request: PandoraRequest, timeout: Duration) -> Result<PandoraReply, String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], info.port));
    let mut stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(timeout)).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(QUICK_TIMEOUT)).map_err(|e| e.to_string())?;
    let envelope = PandoraEnvelope { token: info.token.clone(), request };
    let mut line = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
    line.push('\n');
    stream.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    let mut answer = String::new();
    BufReader::new(stream.take(proto::MAX_LINE_BYTES)).read_line(&mut answer).map_err(|e| e.to_string())?;
    serde_json::from_str(answer.trim_end()).map_err(|e| format!("Pandora sent an unreadable reply: {e}"))
}

fn finish(
    app: &AppHandle,
    state: &Arc<SharedRuntime>,
    action: PandoraAction,
    outcome: Result<Option<RecognizedSong>, String>,
) {
    let notice = match (&outcome, action) {
        (Ok(Some(song)), _) => Some(("Pandora found the song", format!("{} — {}", song.title, song.artist))),
        (Ok(None), PandoraAction::Recognize) => Some(("Pandora", "No match this time.".to_string())),
        (Ok(None), PandoraAction::Open) => None,
        (Err(error), _) => Some(("Pandora", error.clone())),
    };
    tracing::debug!(?action, ok = outcome.is_ok(), "Pandora job finished");
    let last = notice.as_ref().map(|(_, text)| text.clone());
    state.update_and_emit(app, |s| {
        s.pandora.busy = None;
        if last.is_some() {
            s.pandora.last_result.clone_from(&last);
        }
    });
    if let Some((title, message)) = notice {
        // The tray icon lives on the main thread (`tray::TRAY` is thread-local).
        let st = Arc::clone(state);
        let _ = app.run_on_main_thread(move || tray::notify(&st, title, &message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_candidates_look_in_pandoras_own_target() {
        let all = checkout_candidates();
        assert!(!all.is_empty());
        assert!(all.iter().all(|p| p.ends_with("pandora.exe") && p.to_string_lossy().contains("src-tauri")));
    }

    #[test]
    fn a_missing_or_unanswered_discovery_file_is_not_live() {
        let dir = std::env::temp_dir().join(format!("ice-pandora-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(live_bridge(&dir).is_none());
        std::fs::create_dir_all(&dir).unwrap();
        // Port 9 (discard) on loopback never runs a Pandora bridge.
        let stale = DiscoveryFile { version: 1, port: 9, token: "t".into(), pid: 1 };
        std::fs::write(dir.join(proto::DISCOVERY_FILE), serde_json::to_vec(&stale).unwrap()).unwrap();
        assert!(live_bridge(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
