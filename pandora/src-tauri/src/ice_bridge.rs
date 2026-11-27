//! Dual-mode integration between Pandora and Ice Audio Manager:
//!
//! 1. **Embedded / Wrapped mode**: When standalone `ice-audio-manager.exe` is NOT running, Pandora
//!    wraps `ice_win::audio_engine::AudioEngine` directly inside Pandora (so device switching, hotkeys,
//!    Call Mode, spatial sound, and 14-band WASAPI meter work even without launching Ice separately).
//! 2. **Connected / Co-op mode**: When standalone `ice-audio-manager.exe` IS running (detected via
//!    `OpenMutexW("Local\\IceAudioManager.Desktop")` without holding the mutex!), Pandora drops its
//!    embedded engine so standalone Ice keeps exclusive ownership of `RegisterHotKey`, Call Mode, and
//!    the Notch overlay. Pandora still switches devices / volume / spatial directly via `ice_win` and
//!    syncs settings through `%APPDATA%\IceAudioManager\config.toml` (which standalone Ice hot-reloads).

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ice_core::appearance::{NotchStyle, ThemeKind};
use ice_core::companion::{self, DiscoveryFile, InstallHint};
use ice_core::config::ConfigStore;
use ice_core::device::{EndpointInfo, Flow, Role};
use ice_core::engine::{
    Command, DeviceSummary, Engine, EngineSnapshot, PerformanceProfile, RoleSet, SpatialMode, VolumeState,
};
use ice_core::meter::MeterSubscription;
use ice_core::resolve::normalize_name;
use ice_win::audio_engine::AudioEngine;
use ice_win::com::ComGuard;
use ice_win::endpoints::Endpoints;
use ice_win::meter::MeterHub;
use ice_win::spatial::{self, SpatialFormat};
use ice_win::{policy, volume};
use serde::Deserialize;
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_ACCESS_RIGHTS};
use windows::core::w;

use crate::models::{IceConfigPatch, IceStatus};

const ICE_MUTEX_NAME_SYNC: SYNCHRONIZATION_ACCESS_RIGHTS = SYNCHRONIZATION_ACCESS_RIGHTS(0x0010_0000);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeRemoteStatus {
    pid: u32,
    app_version: String,
    notch_expanded: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeReply {
    ok: bool,
    error: Option<String>,
    status: Option<BridgeRemoteStatus>,
    snapshot: Option<EngineSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveBridgeInfo {
    port: u16,
    pid: u32,
    app_version: String,
    notch_expanded: bool,
}

pub struct IceBridge {
    config_path: PathBuf,
    bridge_info_path: PathBuf,
    fake_engine: bool,
    pref: Mutex<String>,
    mode: Mutex<String>,
    last_error: Mutex<Option<String>>,
    embedded: Mutex<Option<Arc<dyn Engine>>>,
    live_bridge: Mutex<Option<LiveBridgeInfo>>,
    last_bridge_revision: AtomicU64,
    coop_meter: Arc<MeterHub>,
    meter_active: Arc<AtomicBool>,
    meter_generation: Mutex<u64>,
    app_handle: Mutex<Option<AppHandle>>,
}

impl IceBridge {
    pub fn new(initial_pref: String, fake_engine: bool) -> Arc<Self> {
        let config_path = default_ice_config_path();
        let bridge_info_path = default_ice_bridge_info_path();
        Arc::new(Self {
            config_path,
            bridge_info_path,
            fake_engine,
            pref: Mutex::new(initial_pref),
            mode: Mutex::new("unavailable".into()),
            last_error: Mutex::new(None),
            embedded: Mutex::new(None),
            live_bridge: Mutex::new(None),
            last_bridge_revision: AtomicU64::new(0),
            coop_meter: MeterHub::new(),
            meter_active: Arc::new(AtomicBool::new(false)),
            meter_generation: Mutex::new(0),
            app_handle: Mutex::new(None),
        })
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn bridge_info_path(&self) -> &Path {
        &self.bridge_info_path
    }

    pub fn attach_app(self: &Arc<Self>, app: AppHandle) {
        if let Ok(mut g) = self.app_handle.lock() {
            *g = Some(app);
        }
        self.reconcile();

        let bridge = Arc::clone(self);
        std::thread::Builder::new()
            .name("pandora-ice-watch".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(2));
                    bridge.reconcile();
                }
            })
            .ok();
    }

    pub fn set_mode_pref(self: &Arc<Self>, pref: &str) {
        if let Ok(mut g) = self.pref.lock() {
            *g = match pref {
                "embedded" | "connected" => pref.to_string(),
                _ => "auto".into(),
            };
        }
        self.reconcile();
    }

    pub fn status(&self) -> IceStatus {
        let mode = self.mode.lock().map(|g| g.clone()).unwrap_or_else(|_| "unavailable".into());
        let error = self.last_error.lock().ok().and_then(|g| g.clone());
        let bridge = self.live_bridge.lock().ok().and_then(|g| g.clone());
        IceStatus {
            mode,
            standalone_running: is_standalone_ice_running(),
            standalone_exe: find_standalone_ice_exe().map(|p| p.display().to_string()),
            config_path: self.config_path.display().to_string(),
            bridge_connected: bridge.is_some(),
            bridge_port: bridge.as_ref().map(|b| b.port),
            bridge_pid: bridge.as_ref().map(|b| b.pid),
            notch_expanded: bridge.as_ref().map(|b| b.notch_expanded),
            ice_version: bridge.as_ref().map(|b| b.app_version.clone()),
            bridge_info_path: self.bridge_info_path.display().to_string(),
            error,
        }
    }

    pub fn reconcile(self: &Arc<Self>) {
        let pref = self.pref.lock().map(|g| g.clone()).unwrap_or_else(|_| "auto".into());
        let standalone = is_standalone_ice_running();
        let want_embedded = if self.fake_engine {
            true
        } else {
            match pref.as_str() {
                "connected" => false,
                "embedded" => !standalone, // never conflict with standalone Ice
                _ => !standalone,
            }
        };

        let mut changed = false;
        let current_has_embedded = self.embedded.lock().map(|g| g.is_some()).unwrap_or(false);

        if want_embedded && !current_has_embedded {
            let _ = self.clear_live_bridge();
            let engine_res: Result<Arc<dyn Engine>, String> = if self.fake_engine {
                Ok(Arc::new(ice_core::engine::fake::FakeEngine::new()))
            } else {
                AudioEngine::start(ConfigStore::new(&self.config_path)).map(|e| Arc::new(e) as Arc<dyn Engine>)
            };
            match engine_res {
                Ok(eng) => {
                    if let Ok(mut g) = self.embedded.lock() {
                        *g = Some(Arc::clone(&eng));
                    }
                    if let Ok(mut m) = self.mode.lock() {
                        *m = if self.fake_engine { "fake".into() } else { "embedded".into() };
                    }
                    if let Ok(mut e) = self.last_error.lock() {
                        *e = None;
                    }
                    self.spawn_embedded_follower(eng);
                    changed = true;
                }
                Err(err) => {
                    if let Ok(mut m) = self.mode.lock() {
                        *m = "unavailable".into();
                    }
                    if let Ok(mut e) = self.last_error.lock() {
                        *e = Some(err);
                    }
                    changed = true;
                }
            }
        } else if !want_embedded {
            if current_has_embedded {
                if let Ok(mut g) = self.embedded.lock() {
                    *g = None;
                }
                changed = true;
            }
            let target_mode = "connected";
            let prev = self.mode.lock().map(|g| g.clone()).unwrap_or_default();
            if prev != target_mode {
                if let Ok(mut m) = self.mode.lock() {
                    *m = target_mode.into();
                }
                if let Ok(mut e) = self.last_error.lock() {
                    *e = None;
                }
                changed = true;
            }

            if standalone {
                let prev_bridge = self.live_bridge.lock().ok().and_then(|g| g.clone());
                let polled = self.poll_bridge_snapshot();
                let next_bridge = self.live_bridge.lock().ok().and_then(|g| g.clone());
                if prev_bridge != next_bridge {
                    changed = true;
                }
                if let Some(snap) = polled {
                    let prev_rev = self.last_bridge_revision.swap(snap.revision, Ordering::AcqRel);
                    if snap.revision != prev_rev && !changed {
                        self.emit_snapshot_value(&snap);
                    }
                }
            } else if self.clear_live_bridge() {
                changed = true;
            }
        }

        if changed {
            self.emit_status_and_snapshot();
        }
    }

    fn spawn_embedded_follower(self: &Arc<Self>, engine: Arc<dyn Engine>) {
        let rx = engine.subscribe();
        let bridge = Arc::clone(self);
        std::thread::Builder::new()
            .name("pandora-ice-events".into())
            .spawn(move || {
                bridge.emit_status_and_snapshot();
                let mut last_emit = Instant::now() - Duration::from_secs(1);
                while let Ok(_env) = rx.recv() {
                    // Drain bursts so the WebView receives the freshest coalesced snapshot.
                    while rx.try_recv().is_ok() {}
                    if last_emit.elapsed() >= Duration::from_millis(40) {
                        bridge.emit_snapshot_only(&engine);
                        last_emit = Instant::now();
                    } else {
                        bridge.emit_snapshot_only(&engine);
                    }
                }
            })
            .ok();
    }

    fn emit_snapshot_only(&self, engine: &Arc<dyn Engine>) {
        self.emit_snapshot_value(&engine.snapshot());
    }

    fn emit_snapshot_value(&self, snap: &EngineSnapshot) {
        if let Ok(guard) = self.app_handle.lock() {
            if let Some(app) = guard.as_ref() {
                let _ = app.emit("ice://snapshot/v1", snap);
            }
        }
    }

    fn emit_status_and_snapshot(&self) {
        if let Ok(guard) = self.app_handle.lock() {
            if let Some(app) = guard.as_ref() {
                let _ = app.emit("ice://status/v1", self.status());
                let _ = app.emit("ice://snapshot/v1", self.snapshot());
            }
        }
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        if let Some(eng) = self.embedded.lock().ok().and_then(|g| g.clone()) {
            return eng.snapshot();
        }
        if let Some(snap) = self.poll_bridge_snapshot() {
            self.last_bridge_revision.store(snap.revision, Ordering::Release);
            return snap;
        }
        self.coop_snapshot()
    }

    pub fn send_command(self: &Arc<Self>, cmd: Command) -> Result<(), String> {
        if let Some(eng) = self.embedded.lock().ok().and_then(|g| g.clone()) {
            eng.send(cmd);
            return Ok(());
        }
        if let Ok(reply) = self.call_bridge(serde_json::json!({ "action": "command", "command": &cmd })) {
            if let Some(snap) = reply.snapshot {
                self.last_bridge_revision.store(snap.revision, Ordering::Release);
                self.emit_status_value_and_snapshot(snap);
            } else {
                self.emit_status_and_snapshot();
            }
            return Ok(());
        }
        self.execute_coop_command(cmd)?;
        self.emit_status_and_snapshot();
        Ok(())
    }

    pub fn ping_bridge(self: &Arc<Self>) -> Result<IceStatus, String> {
        self.reconcile();
        if is_standalone_ice_running() {
            let reply = self.call_bridge(serde_json::json!({ "action": "ping" }))?;
            if let Some(st) = reply.status {
                tracing::debug!(pid = st.pid, version = %st.app_version, "Ice bridge ping ok");
            }
        }
        self.emit_status_and_snapshot();
        Ok(self.status())
    }

    pub fn toggle_notch(self: &Arc<Self>) -> Result<IceStatus, String> {
        if !is_standalone_ice_running() {
            return Err("Start standalone Ice Audio Manager to control its desktop Notch.".into());
        }
        self.call_bridge(serde_json::json!({ "action": "toggleNotch" }))?;
        std::thread::sleep(Duration::from_millis(40));
        let _ = self.call_bridge(serde_json::json!({ "action": "ping" }));
        self.emit_status_and_snapshot();
        Ok(self.status())
    }

    pub fn show_notch(self: &Arc<Self>) -> Result<IceStatus, String> {
        if !is_standalone_ice_running() {
            return Err("Start standalone Ice Audio Manager to show its desktop Notch.".into());
        }
        self.call_bridge(serde_json::json!({ "action": "showNotch" }))?;
        std::thread::sleep(Duration::from_millis(40));
        let _ = self.call_bridge(serde_json::json!({ "action": "ping" }));
        self.emit_status_and_snapshot();
        Ok(self.status())
    }

    /// The title bar's Ice button: start standalone Ice if it isn't running, then ask it to wake the notch from
    /// hibernation and show the card for `ms`. An Ice without `peekNotch` (older build) is woken and shown instead.
    pub fn wake_and_peek(self: &Arc<Self>, ms: u64) -> Result<IceStatus, String> {
        if !is_standalone_ice_running() {
            self.launch_standalone()?;
        }
        if self.call_bridge(serde_json::json!({ "action": "peekNotch", "ms": ms })).is_err() {
            let _ = self.patch_settings(IceConfigPatch { hibernate: Some(false), ..IceConfigPatch::default() });
            self.call_bridge(serde_json::json!({ "action": "showNotch" }))?;
        }
        std::thread::sleep(Duration::from_millis(40));
        let _ = self.call_bridge(serde_json::json!({ "action": "ping" }));
        self.emit_status_and_snapshot();
        Ok(self.status())
    }

    pub fn open_ice_settings(self: &Arc<Self>) -> Result<(), String> {
        if !is_standalone_ice_running() {
            return Err("Start standalone Ice Audio Manager to open Ice's Settings window.".into());
        }
        self.call_bridge(serde_json::json!({ "action": "openSettings" }))?;
        Ok(())
    }

    fn emit_status_value_and_snapshot(&self, snap: EngineSnapshot) {
        if let Ok(guard) = self.app_handle.lock() {
            if let Some(app) = guard.as_ref() {
                let _ = app.emit("ice://status/v1", self.status());
                let _ = app.emit("ice://snapshot/v1", snap);
            }
        }
    }

    fn clear_live_bridge(&self) -> bool {
        self.live_bridge
            .lock()
            .ok()
            .and_then(|mut g| g.take())
            .is_some()
    }

    fn poll_bridge_snapshot(&self) -> Option<EngineSnapshot> {
        self.call_bridge(serde_json::json!({ "action": "snapshot" }))
            .ok()
            .and_then(|r| r.snapshot)
    }

    fn call_bridge(&self, mut payload: serde_json::Value) -> Result<BridgeReply, String> {
        let info = read_bridge_discovery(&self.bridge_info_path)
            .ok_or_else(|| "Ice local bridge file not found (standalone Ice may be older or stopped)".to_string())?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("token".into(), serde_json::Value::String(info.token.clone()));
        }
        let reply = bridge_tcp_call(&info, &payload);
        match &reply {
            Ok(rep) if rep.ok => {
                if let Some(st) = &rep.status {
                    if let Ok(mut g) = self.live_bridge.lock() {
                        *g = Some(LiveBridgeInfo {
                            port: info.port,
                            pid: st.pid,
                            app_version: st.app_version.clone(),
                            notch_expanded: st.notch_expanded,
                        });
                    }
                }
            }
            _ => {
                self.clear_live_bridge();
            }
        }
        reply
    }

    pub fn patch_settings(self: &Arc<Self>, patch: IceConfigPatch) -> Result<EngineSnapshot, String> {
        let mut cmds = Vec::new();
        if let Some(t) = patch.theme.as_deref().and_then(parse_theme) {
            cmds.push(Command::SetTheme { theme: t });
        }
        if let Some(s) = patch.notch_style.as_deref().and_then(parse_notch_style) {
            cmds.push(Command::SetNotchStyle { style: s });
        }
        if let Some(h) = patch.hibernate {
            cmds.push(Command::SetHibernate { enabled: h });
        }
        if let Some(mb) = patch.moving_backdrop {
            cmds.push(Command::SetMovingBackdrop { enabled: mb });
        }
        if let Some(cm) = patch.call_mode_enabled {
            cmds.push(Command::SetCallMode {
                enabled: Some(cm),
                apps: None,
                duck_percent: None,
                hold_secs: None,
                sensitivity: None,
            });
        }
        if let Some(le) = patch.lyrics_enabled {
            cmds.push(Command::SetLyricsEnabled { enabled: le });
        }
        if let Some(lo) = patch.lyrics_online_lookup {
            cmds.push(Command::SetLyricsOnlineLookup { enabled: lo });
        }
        if let Some(rom) = patch.lyrics_romaji {
            cmds.push(Command::SetLyricsDisplay {
                show_on_pill: None,
                show_on_card: None,
                word_highlight: None,
                romaji: Some(rom),
            });
        }
        if let Some(ews) = patch.lyrics_estimated_word_sync {
            cmds.push(Command::SetLyricsEstimatedWordSync { enabled: ews });
        }
        if let Some(vs) = patch.visualizer_spectrum {
            cmds.push(Command::SetVisualizerSpectrum { enabled: vs });
        }
        if let Some(pp) = patch.performance_profile.as_deref().and_then(parse_profile) {
            cmds.push(Command::SetPerformanceProfile { profile: pp });
        }

        if let Some(eng) = self.embedded.lock().ok().and_then(|g| g.clone()) {
            for cmd in cmds {
                eng.send(cmd);
            }
            std::thread::sleep(Duration::from_millis(30));
            return Ok(eng.snapshot());
        }

        if is_standalone_ice_running() && read_bridge_discovery(&self.bridge_info_path).is_some() {
            let mut latest_snap = None;
            let mut bridge_ok = true;
            for cmd in &cmds {
                match self.call_bridge(serde_json::json!({ "action": "command", "command": cmd })) {
                    Ok(rep) => {
                        if rep.snapshot.is_some() {
                            latest_snap = rep.snapshot;
                        }
                    }
                    Err(_) => {
                        bridge_ok = false;
                        break;
                    }
                }
            }
            if bridge_ok {
                let snap = latest_snap.unwrap_or_else(|| self.snapshot());
                self.last_bridge_revision.store(snap.revision, Ordering::Release);
                self.emit_status_value_and_snapshot(snap.clone());
                return Ok(snap);
            }
        }

        for cmd in cmds {
            self.execute_coop_command(cmd)?;
        }
        let snap = self.coop_snapshot();
        self.emit_status_and_snapshot();
        Ok(snap)
    }

    pub fn set_meter_active(self: &Arc<Self>, active: bool) {
        let prev = self.meter_active.swap(active, Ordering::AcqRel);
        if !active || prev == active {
            return;
        }
        let epoch = {
            let mut g = self.meter_generation.lock().unwrap_or_else(|e| e.into_inner());
            *g += 1;
            *g
        };
        let bridge = Arc::clone(self);
        std::thread::Builder::new()
            .name("pandora-ice-meter".into())
            .spawn(move || {
                let sub: Option<MeterSubscription> = {
                    let embedded = bridge.embedded.lock().ok().and_then(|g| g.clone());
                    if let Some(eng) = embedded {
                        eng.meter()
                    } else {
                        bridge.coop_meter.subscribe()
                    }
                };
                let Some(rx) = sub else { return };
                while bridge.meter_active.load(Ordering::Acquire) {
                    let current_epoch = bridge.meter_generation.lock().map(|g| *g).unwrap_or(0);
                    if current_epoch != epoch {
                        break;
                    }
                    match rx.recv_timeout(Duration::from_millis(200)) {
                        Ok(mut frame) => {
                            while let Ok(newer) = rx.try_recv() {
                                frame = newer;
                            }
                            if let Ok(guard) = bridge.app_handle.lock() {
                                if let Some(app) = guard.as_ref() {
                                    let _ = app.emit("ice://meter/v1", frame);
                                }
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
            .ok();
    }

    pub fn launch_standalone(self: &Arc<Self>) -> Result<IceStatus, String> {
        if is_standalone_ice_running() {
            self.reconcile();
            return Ok(self.status());
        }
        let exe = find_standalone_ice_exe().ok_or_else(|| {
            "Could not find ice-audio-manager.exe. Build standalone Ice first (`build` in repo root).".to_string()
        })?;
        // Release embedded AudioEngine first so standalone Ice gets clean ownership of hotkeys and mutex!
        if let Ok(mut g) = self.embedded.lock() {
            *g = None;
        }
        if let Ok(mut m) = self.mode.lock() {
            *m = "connected".into();
        }
        std::thread::sleep(Duration::from_millis(120));

        ProcessCommand::new(&exe)
            .spawn()
            .map_err(|e| format!("Failed to start {}: {e}", exe.display()))?;

        // Wait up to 3s for Ice's single-instance mutex to come up, then wait briefly for bridge.json.
        for _ in 0..15 {
            if is_standalone_ice_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        for _ in 0..12 {
            if self.call_bridge(serde_json::json!({ "action": "ping" })).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        self.emit_status_and_snapshot();
        Ok(self.status())
    }

    pub fn stop_standalone(self: &Arc<Self>) -> Result<IceStatus, String> {
        let _ = ProcessCommand::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-Process ice-audio-manager -ErrorAction SilentlyContinue | Stop-Process -Force",
            ])
            .output();
        for _ in 0..15 {
            if !is_standalone_ice_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        self.set_mode_pref("auto");
        Ok(self.status())
    }

    fn coop_snapshot(&self) -> EngineSnapshot {
        let store = ConfigStore::new(&self.config_path);
        let cfg = store.read().ok().flatten().unwrap_or_else(|| store.load().config().clone());
        self.coop_meter.set_spectrum_enabled(cfg.visualizer_spectrum);

        let Ok(_com) = ComGuard::init_mta() else {
            return EngineSnapshot {
                settings: cfg.view(&[]),
                ..EngineSnapshot::default()
            };
        };
        let Ok(eps) = Endpoints::new() else {
            return EngineSnapshot {
                settings: cfg.view(&[]),
                ..EngineSnapshot::default()
            };
        };
        let endpoints = eps.list(false).unwrap_or_default();
        let to_summary = |e: &EndpointInfo| -> DeviceSummary {
            let sp = if e.flow == Flow::Render {
                spatial::active(&e.id).ok().flatten().and_then(|(f, _)| match f? {
                    SpatialFormat::WindowsSonic => Some(SpatialMode::WindowsSonic),
                    SpatialFormat::DolbyAtmosHeadphones => Some(SpatialMode::DolbyAtmosHeadphones),
                    SpatialFormat::DolbyAtmosHomeTheater => Some(SpatialMode::DolbyAtmosHomeTheater),
                    SpatialFormat::DolbyAtmosSpeakers => Some(SpatialMode::DolbyAtmosSpeakers),
                    SpatialFormat::DtsHeadphoneX => Some(SpatialMode::DtsHeadphoneX),
                    SpatialFormat::DtsXUltra => Some(SpatialMode::DtsXUltra),
                    SpatialFormat::DtsXHomeTheater => Some(SpatialMode::DtsXHomeTheater),
                })
            } else {
                None
            };
            DeviceSummary {
                id: e.id.clone(),
                name: cfg
                    .prefs_for(e)
                    .and_then(|p| p.alias.clone())
                    .unwrap_or_else(|| normalize_name(e.friendly_name.as_deref().unwrap_or(&e.id))),
                flow: e.flow,
                state: e.state,
                is_default: e.default_for.contains(&Role::Multimedia),
                is_default_communications: e.default_for.contains(&Role::Communications),
                form_factor: e.form_factor,
                is_virtual: e
                    .hardware_key
                    .as_deref()
                    .is_some_and(|k| k.starts_with("ROOT\\") || k.starts_with("SWD\\")),
                spatial: sp,
                lyrics_delay_ms: if e.flow == Flow::Render { cfg.lyrics_delay_for(e) } else { 0 },
            }
        };

        let playback = endpoints
            .iter()
            .filter(|e| e.flow == Flow::Render)
            .map(to_summary)
            .collect();
        let recording = endpoints
            .iter()
            .filter(|e| e.flow == Flow::Capture)
            .map(to_summary)
            .collect();

        let vol = eps
            .default_device(Flow::Render, Role::Multimedia)
            .ok()
            .and_then(|d| volume::get(&d).ok())
            .map(|(s, muted)| VolumeState {
                percent: (s * 100.0).round().clamp(0.0, 100.0) as u8,
                muted,
            });
        let mic = eps
            .default_device(Flow::Capture, Role::Communications)
            .ok()
            .and_then(|d| volume::get(&d).ok());

        EngineSnapshot {
            revision: 1,
            playback,
            recording,
            disconnected: Vec::new(),
            volume: vol,
            mic_muted: mic.map(|(_, m)| m),
            mic_volume: mic.map(|(s, _)| (s * 100.0).round().clamp(0.0, 100.0) as u8),
            notices: Vec::new(),
            settings: cfg.view(&endpoints),
            media: None,
            lyrics: Default::default(),
        }
    }

    fn execute_coop_command(&self, cmd: Command) -> Result<(), String> {
        let _com = ComGuard::init_mta().map_err(|e| format!("COM: {e}"))?;
        let eps = Endpoints::new().map_err(|e| format!("audio service: {e}"))?;

        if cmd.is_setting() {
            let store = ConfigStore::new(&self.config_path);
            let mut cfg = store
                .read()
                .ok()
                .flatten()
                .unwrap_or_else(|| store.load().config().clone());
            let endpoints = eps.list(false).unwrap_or_default();
            if cfg.apply_setting(&cmd, &endpoints)? {
                store.save(&cfg).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }

        match cmd {
            Command::SwitchTo { device_id, roles } => {
                let _ = policy::set_default(&device_id, roles.roles()).map_err(|e| e.to_string())?;
                self.coop_meter.rebind();
                Ok(())
            }
            Command::SetVolume { percent } => {
                let dev = eps
                    .default_device(Flow::Render, Role::Multimedia)
                    .map_err(|e| e.to_string())?;
                volume::set_scalar(&dev, f32::from(percent.min(100)) / 100.0).map_err(|e| e.to_string())
            }
            Command::ToggleMute => {
                let dev = eps
                    .default_device(Flow::Render, Role::Multimedia)
                    .map_err(|e| e.to_string())?;
                let (_, muted) = volume::get(&dev).map_err(|e| e.to_string())?;
                volume::set_mute(&dev, !muted).map_err(|e| e.to_string())
            }
            Command::ToggleMicMute => {
                let dev = eps
                    .default_device(Flow::Capture, Role::Communications)
                    .map_err(|e| e.to_string())?;
                let (_, muted) = volume::get(&dev).map_err(|e| e.to_string())?;
                volume::set_mute(&dev, !muted).map_err(|e| e.to_string())
            }
            Command::SetSpatial { device_id, mode } => {
                let fmt = match mode {
                    SpatialMode::Keep => return Ok(()),
                    SpatialMode::Off => None,
                    SpatialMode::WindowsSonic => Some(SpatialFormat::WindowsSonic),
                    SpatialMode::DolbyAtmosHeadphones => Some(SpatialFormat::DolbyAtmosHeadphones),
                    SpatialMode::DolbyAtmosHomeTheater => Some(SpatialFormat::DolbyAtmosHomeTheater),
                    SpatialMode::DolbyAtmosSpeakers => Some(SpatialFormat::DolbyAtmosSpeakers),
                    SpatialMode::DtsHeadphoneX => Some(SpatialFormat::DtsHeadphoneX),
                    SpatialMode::DtsXUltra => Some(SpatialFormat::DtsXUltra),
                    SpatialMode::DtsXHomeTheater => Some(SpatialFormat::DtsXHomeTheater),
                };
                spatial::set_default_within(&device_id, fmt, Duration::from_secs(3)).map(|_| ())
            }
            Command::Refresh => Ok(()),
            _ => Err("Command requires standalone Ice or Embedded engine mode".into()),
        }
    }
}

pub fn is_standalone_ice_running() -> bool {
    // SAFETY: read-only probe of Ice's session-local mutex; immediately closed without acquiring ownership.
    let handle = unsafe { OpenMutexW(ICE_MUTEX_NAME_SYNC, false, w!("Local\\IceAudioManager.Desktop")) };
    if let Ok(h) = handle {
        // SAFETY: valid handle just returned by OpenMutexW.
        unsafe {
            let _ = CloseHandle(h);
        }
        true
    } else {
        false
    }
}

fn default_ice_config_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("IceAudioManager").join("config.toml")
}

fn default_ice_bridge_info_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(companion::ICE_DIR).join(companion::DISCOVERY_FILE)
}

fn read_bridge_discovery(path: &Path) -> Option<DiscoveryFile> {
    let text = std::fs::read_to_string(path).ok()?;
    let info: DiscoveryFile = serde_json::from_str(&text).ok()?;
    if info.version == companion::PROTOCOL_VERSION && info.port > 0 && !info.token.is_empty() && info.pid > 0 {
        Some(info)
    } else {
        None
    }
}

fn bridge_tcp_call(info: &DiscoveryFile, payload: &serde_json::Value) -> Result<BridgeReply, String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], info.port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(350))
        .map_err(|e| format!("Ice bridge unreachable on port {}: {e}", info.port))?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1200)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(1200)));

    let mut req_line = serde_json::to_string(payload).map_err(|e| e.to_string())?;
    req_line.push('\n');
    stream
        .write_all(req_line.as_bytes())
        .map_err(|e| format!("Ice bridge write error: {e}"))?;
    stream.flush().map_err(|e| format!("Ice bridge flush error: {e}"))?;

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader
        .read_line(&mut line)
        .map_err(|e| format!("Ice bridge read error: {e}"))?;

    let reply: BridgeReply =
        serde_json::from_str(line.trim_end()).map_err(|e| format!("Invalid Ice bridge reply: {e}"))?;
    if !reply.ok {
        return Err(reply.error.unwrap_or_else(|| "Ice bridge returned an error".into()));
    }
    Ok(reply)
}

/// Where to start Ice from: the `install.json` Ice writes at every start, then a source checkout next to Pandora.
fn find_standalone_ice_exe() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = ice_install_hint().into_iter().collect();
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().take(6) {
            candidates.push(ancestor.join("target").join("release").join("ice-audio-manager.exe"));
            candidates.push(ancestor.join("target").join("debug").join("ice-audio-manager.exe"));
            candidates.push(ancestor.join("ice-audio-manager.exe"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors().take(4) {
            candidates.push(ancestor.join("target").join("release").join("ice-audio-manager.exe"));
            candidates.push(ancestor.join("target").join("debug").join("ice-audio-manager.exe"));
        }
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        candidates.push(local.join("IceAudioManager").join("ice-audio-manager.exe"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn parse_theme(s: &str) -> Option<ThemeKind> {
    match s {
        "ice" => Some(ThemeKind::Ice),
        "ice_light" => Some(ThemeKind::IceLight),
        "midnight" => Some(ThemeKind::Midnight),
        "fluent" => Some(ThemeKind::Fluent),
        "minimal" => Some(ThemeKind::Minimal),
        _ => None,
    }
}

fn parse_notch_style(s: &str) -> Option<NotchStyle> {
    match s {
        "floating" => Some(NotchStyle::Floating),
        "nook" => Some(NotchStyle::Nook),
        _ => None,
    }
}

fn parse_profile(s: &str) -> Option<PerformanceProfile> {
    match s {
        "instant" => Some(PerformanceProfile::Instant),
        "balanced" => Some(PerformanceProfile::Balanced),
        "lite" => Some(PerformanceProfile::Lite),
        _ => None,
    }
}

pub fn apply_window_backdrop(win: &tauri::WebviewWindow) {
    if let Ok(hwnd) = win.hwnd() {
        let _ = ice_win::overlay_win::apply_settings_mica(hwnd.0 as isize, true);
    }
}

#[allow(dead_code)]
fn _role_default() -> RoleSet {
    RoleSet::Default
}

/// `%LOCALAPPDATA%\IceAudioManager\install.json`, if it names an absolute `.exe`.
fn ice_install_hint() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join(companion::ICE_DIR);
    let bytes = std::fs::read(dir.join(companion::INSTALL_FILE)).ok()?;
    serde_json::from_slice::<InstallHint>(&bytes).ok()?.exe()
}
