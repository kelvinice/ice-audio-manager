//! Ice → Pandora loopback bridge (experimental, owner 2026-11-27), the reverse of `ice_bridge.rs`.
//!
//! Lets Ice's notch widget drive Pandora: `ping`, `show` (bring the window up) and `recognize` (listen to the PC's
//! playback and name the song). Speaks `ice_core::companion`: one token-checked JSON line in, one out, on
//! `127.0.0.1` only. Writes `%LOCALAPPDATA%\Pandora\bridge.json` while running (removed on exit) and `install.json`
//! (kept) so Ice can start Pandora. Each connection gets its own short thread, because a recognition blocks for
//! ~10 s and a ping must still answer meanwhile; at most `MAX_CLIENTS` run at once.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use ice_core::companion::{
    self as proto, DiscoveryFile, InstallHint, PandoraEnvelope, PandoraReply, PandoraRequest, RecognizedSong,
};
use tauri::{AppHandle, Emitter};

use crate::recognition::CaptureSource;

const IO_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_CLIENTS: usize = 4;

static ACTIVE: Mutex<Option<Listening>> = Mutex::new(None);
static CLIENTS: AtomicUsize = AtomicUsize::new(0);

struct Listening {
    stop: Arc<AtomicBool>,
    port: u16,
    discovery: PathBuf,
}

/// `%LOCALAPPDATA%\Pandora`.
fn data_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join(proto::PANDORA_DIR))
}

/// Writes the install hint, binds the listener and publishes `bridge.json`. Failures only log: Pandora works without.
pub fn start(app: &AppHandle) {
    let Some(dir) = data_dir() else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(&dir) {
        tracing::warn!(%error, "Pandora companion folder unavailable");
        return;
    }
    write_install_hint(&dir);

    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(error) => {
            tracing::warn!(%error, "Pandora companion bridge failed to bind");
            return;
        }
    };
    let Ok(port) = listener.local_addr().map(|a| a.port()) else {
        return;
    };
    let token = proto::new_token();
    let discovery = dir.join(proto::DISCOVERY_FILE);
    let info = DiscoveryFile { version: proto::PROTOCOL_VERSION, port, token: token.clone(), pid: std::process::id() };
    if let Err(error) = write_json(&discovery, &info) {
        tracing::warn!(%error, "Pandora companion bridge could not publish bridge.json");
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    *ACTIVE.lock().unwrap_or_else(PoisonError::into_inner) =
        Some(Listening { stop: Arc::clone(&stop), port, discovery });

    let app = app.clone();
    let spawned = std::thread::Builder::new().name("pandora-companion".into()).spawn(move || {
        for conn in listener.incoming() {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let Ok(stream) = conn else { continue };
            if CLIENTS.fetch_add(1, Ordering::AcqRel) >= MAX_CLIENTS {
                CLIENTS.fetch_sub(1, Ordering::AcqRel);
                reply(stream, &PandoraReply::failed("busy"));
                continue;
            }
            let (app, token) = (app.clone(), token.clone());
            let client = std::thread::Builder::new().name("pandora-companion-client".into()).spawn(move || {
                handle(stream, &app, &token);
                CLIENTS.fetch_sub(1, Ordering::AcqRel);
            });
            if client.is_err() {
                CLIENTS.fetch_sub(1, Ordering::AcqRel);
            }
        }
    });
    if let Err(error) = spawned {
        tracing::warn!(%error, "Pandora companion thread failed to spawn");
        return;
    }
    tracing::info!(port, "Pandora companion bridge ready");
}

/// Removes `bridge.json` and wakes the accept loop so its thread ends.
pub fn stop() {
    let Some(entry) = ACTIVE.lock().unwrap_or_else(PoisonError::into_inner).take() else {
        return;
    };
    entry.stop.store(true, Ordering::Release);
    let _ = std::fs::remove_file(&entry.discovery);
    let _ = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], entry.port)), Duration::from_millis(100));
}

fn write_install_hint(dir: &std::path::Path) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let hint = InstallHint {
        version: proto::PROTOCOL_VERSION,
        exe_path: exe.display().to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    if let Err(error) = write_json(&dir.join(proto::INSTALL_FILE), &hint) {
        tracing::debug!(%error, "Pandora install hint not written");
    }
}

fn write_json<T: serde::Serialize>(path: &std::path::Path, value: &T) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

fn handle(mut stream: TcpStream, app: &AppHandle, token: &str) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let mut line = String::new();
    let read = BufReader::new(Read::by_ref(&mut stream).take(proto::MAX_LINE_BYTES)).read_line(&mut line);
    let answer = match read {
        Ok(0) => return,
        Ok(_) => match serde_json::from_str::<PandoraEnvelope>(line.trim_end()) {
            Ok(env) if proto::tokens_match(&env.token, token) => dispatch(env.request, app),
            Ok(_) => PandoraReply::failed("unauthorized"),
            Err(error) => PandoraReply::failed(format!("invalid request: {error}")),
        },
        Err(error) => PandoraReply::failed(format!("read error: {error}")),
    };
    reply(stream, &answer);
}

fn dispatch(request: PandoraRequest, app: &AppHandle) -> PandoraReply {
    let ok = PandoraReply { ok: true, app_version: Some(env!("CARGO_PKG_VERSION").into()), ..PandoraReply::default() };
    match request {
        PandoraRequest::Ping => ok,
        PandoraRequest::Show => {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || crate::show_main_window(&handle));
            ok
        }
        PandoraRequest::Recognize => match crate::recognize_now(app, CaptureSource::SystemAudio) {
            Ok(track) => {
                let _ = app.emit("companion://recognized/v1", &track);
                PandoraReply {
                    song: Some(RecognizedSong { title: track.title, artist: track.artist, album: track.album }),
                    ..ok
                }
            }
            Err(error) => PandoraReply::failed(error),
        },
    }
}

fn reply(mut stream: TcpStream, answer: &PandoraReply) {
    if let Ok(mut json) = serde_json::to_string(answer) {
        json.push('\n');
        let _ = stream.write_all(json.as_bytes());
        let _ = stream.flush();
    }
}
