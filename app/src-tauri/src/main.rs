#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

mod art;
mod engine_bridge;
mod ipc;
mod logging;
mod placement;
mod tray;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use ice_core::engine::{Command, EngineSnapshot, RoleSet, fake::FakeEngine};
use ice_win::desktop::InstanceGuard;
use ice_win::monitors::{MonitorInfo, list_monitors};
use ice_win::native_dot::{NativeDotHandle, NativePillTheme};
use ice_win::overlay_win;
use ice_win::proc_metrics::{TreeSnapshot, cpu_percent_between, current_pid, sample_pids};
use ice_win::webview2;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::ipc::{ActivityKind, HitRect, MonitorSummary, NotchDelta, NotchSnapshot, SpringPreset, ThemeKind};

const NOTCH_CANVAS_W: i32 = 420;
const NOTCH_CANVAS_H: i32 = 190;
const NOTCH_TOP_OFFSET: i32 = 0;

struct SharedRuntime {
    engine: engine_bridge::EngineSlot,
    bench_mode: bool,
    tray_prompts: Mutex<tray::Prompts>,
    cursor_poll_running: AtomicBool,
    notch_opening: AtomicBool,
    settings_opening: AtomicBool,
    notch_suspended: AtomicBool,
    want_suspended: AtomicBool,
    suspend_gen: AtomicU64,
    pending_wake_seq: AtomicU64,
    snapshot: Mutex<NotchSnapshot>,
    monitors: Mutex<Vec<MonitorInfo>>,
    placement: Mutex<placement::Placement>,
    preview_on_attach: AtomicBool,
    start_minimized: AtomicBool,
    hit_rect: Mutex<HitRect>,
    notch_hwnd: AtomicIsize,
    notch_visible: AtomicBool,
    allow_exit: AtomicBool,
    wv2_data_dir: PathBuf,
    settings_wv2_data_dir: PathBuf,
    known_wv2_pids: Mutex<HashMap<u32, String>>,
    first_paint_tx: Mutex<Option<mpsc::SyncSender<Instant>>>,
    settings_ready_tx: Mutex<Option<mpsc::SyncSender<Instant>>>,
    event_rendered_waiter: Mutex<Option<(u64, mpsc::SyncSender<Instant>)>>,
    native_dot: Mutex<Option<NativeDotHandle>>,
}

impl SharedRuntime {
    fn new(
        monitors: Vec<MonitorInfo>,
        wv2_data_dir: PathBuf,
        settings_wv2_data_dir: PathBuf,
        bench_mode: bool,
        engine: engine_bridge::EngineSlot,
        initial: &EngineSnapshot,
    ) -> Self {
        let summaries = placement::summaries(&monitors);
        let reduce_motion = overlay_win::reduce_motion_enabled();
        let mut snapshot = NotchSnapshot {
            revision: 1,
            event_seq: 0,
            activity: ActivityKind::Idle,
            volume_percent: 0,
            muted: false,
            mic_muted: false,
            active_device: String::new(),
            target_device: String::new(),
            spatial_badge: None,
            preset: SpringPreset::Smooth,
            system_reduce_motion: reduce_motion,
            theme: ThemeKind::Ice,
            expanded: false,
            suspended: false,
            hidden_for_bench: false,
            active_monitor: 0,
            monitors: summaries,
        };
        engine_bridge::project(initial, &mut snapshot);
        Self {
            engine,
            bench_mode,
            tray_prompts: Mutex::default(),
            cursor_poll_running: AtomicBool::new(false),
            notch_opening: AtomicBool::new(false),
            settings_opening: AtomicBool::new(false),
            notch_suspended: AtomicBool::new(false),
            want_suspended: AtomicBool::new(false),
            suspend_gen: AtomicU64::new(0),
            pending_wake_seq: AtomicU64::new(0),
            snapshot: Mutex::new(snapshot),
            monitors: Mutex::new(monitors),
            placement: Mutex::default(),
            preview_on_attach: AtomicBool::default(),
            start_minimized: AtomicBool::default(),
            hit_rect: Mutex::new(HitRect { x: 154.0, y: 6.0, width: 112.0, height: 28.0 }),
            notch_hwnd: AtomicIsize::new(0),
            notch_visible: AtomicBool::new(false),
            allow_exit: AtomicBool::new(false),
            wv2_data_dir,
            settings_wv2_data_dir,
            known_wv2_pids: Mutex::new(HashMap::new()),
            first_paint_tx: Mutex::new(None),
            settings_ready_tx: Mutex::new(None),
            event_rendered_waiter: Mutex::new(None),
            native_dot: Mutex::new(None),
        }
    }

    fn snapshot_clone(&self) -> NotchSnapshot {
        self.snapshot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    fn update_and_emit(&self, app: &AppHandle, mutator: impl FnOnce(&mut NotchSnapshot)) -> NotchSnapshot {
        let snap = {
            let mut guard = self.snapshot.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.revision = guard.revision.saturating_add(1);
            mutator(&mut guard);
            guard.clone()
        };
        let delta = NotchDelta::State { snapshot: snap.clone() };
        let _ = app.emit_to("notch", "notch://delta/v1", &delta);
        let _ = app.emit_to("settings", "notch://delta/v1", &delta);
        snap
    }

    fn placement(&self) -> std::sync::MutexGuard<'_, placement::Placement> {
        self.placement.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn active_monitor_info(&self) -> Option<MonitorInfo> {
        let idx = self.snapshot_clone().active_monitor;
        let guard = self.monitors.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.get(idx).or_else(|| guard.first()).cloned()
    }
}

#[tauri::command]
fn get_notch_snapshot(state: State<'_, Arc<SharedRuntime>>) -> NotchSnapshot {
    state.snapshot_clone()
}

#[tauri::command]
fn get_engine_snapshot(state: State<'_, Arc<SharedRuntime>>) -> EngineSnapshot {
    state.engine.snapshot()
}

#[tauri::command]
fn report_hit_rect(rect: HitRect, state: State<'_, Arc<SharedRuntime>>) {
    let mut guard = state.hit_rect.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = rect;
}

#[tauri::command]
fn report_first_paint(state: State<'_, Arc<SharedRuntime>>) {
    let now = Instant::now();
    if let Some(tx) = state.first_paint_tx.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take() {
        let _ = tx.send(now);
    }
    if !state.bench_mode
        && state.placement().allows_webview()
        && state.notch_visible.load(Ordering::Acquire)
        && let Some(dot) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref()
    {
        dot.set_standby_only(true);
        dot.hide();
    }
}

#[tauri::command]
fn report_settings_ready(state: State<'_, Arc<SharedRuntime>>) {
    let now = Instant::now();
    if let Some(tx) = state.settings_ready_tx.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take() {
        let _ = tx.send(now);
    }
}

#[tauri::command]
fn report_event_rendered(event_seq: u64, state: State<'_, Arc<SharedRuntime>>) {
    let now = Instant::now();
    let mut guard = state.event_rendered_waiter.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((expected, _)) = guard.as_ref()
        && event_seq >= *expected
        && let Some((_, tx)) = guard.take()
    {
        let _ = tx.send(now);
    }
    drop(guard);

    if !state.bench_mode && state.placement().allows_webview() {
        let pending = state.pending_wake_seq.load(Ordering::Acquire);
        if pending > 0 && event_seq >= pending && !state.want_suspended.load(Ordering::Acquire) {
            state.pending_wake_seq.store(0, Ordering::Release);
            state.notch_visible.store(true, Ordering::Release);
            let hwnd = state.notch_hwnd.load(Ordering::Acquire);
            if hwnd != 0 {
                let _ = overlay_win::ensure_topmost(hwnd);
            }
            if let Some(dot) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
                dot.set_standby_only(true);
                dot.hide();
            }
            spawn_cursor_poll(Arc::clone(&state), false);
        }
    }
}

#[tauri::command]
fn adjust_volume(delta: i8, state: State<'_, Arc<SharedRuntime>>) {
    state.engine.send(Command::StepVolume { delta });
}

#[tauri::command]
fn toggle_mute(state: State<'_, Arc<SharedRuntime>>) {
    state.engine.send(Command::ToggleMute);
}

#[tauri::command]
fn toggle_mic_mute(state: State<'_, Arc<SharedRuntime>>) {
    state.engine.send(Command::ToggleMicMute);
}

#[tauri::command]
fn trigger_switch(state: State<'_, Arc<SharedRuntime>>) {
    let snapshot = state.engine.snapshot();
    if let Some(target) = engine_bridge::preview_target(&snapshot) {
        state.engine.send(Command::SwitchTo { device_id: target.id.clone(), roles: RoleSet::Default });
    }
}

#[tauri::command]
fn switch_to_device(device_id: String, state: State<'_, Arc<SharedRuntime>>) {
    state.engine.send(Command::SwitchTo { device_id, roles: RoleSet::Default });
}

#[tauri::command]
fn toggle_expand(app: AppHandle, state: State<'_, Arc<SharedRuntime>>) {
    state.update_and_emit(&app, |s| {
        s.expanded = !s.expanded;
        s.event_seq = s.event_seq.saturating_add(1);
    });
}

#[tauri::command]
fn set_preset(preset: SpringPreset, app: AppHandle, state: State<'_, Arc<SharedRuntime>>) {
    state.update_and_emit(&app, |s| {
        s.preset = preset;
    });
}

const fn theme_to_native(theme: ThemeKind) -> NativePillTheme {
    match theme {
        ThemeKind::Ice => NativePillTheme::ice(),
        ThemeKind::Midnight => NativePillTheme::midnight(),
        ThemeKind::Fluent => NativePillTheme::fluent(),
        ThemeKind::Minimal => NativePillTheme::minimal(),
    }
}

#[tauri::command]
fn set_theme(theme: ThemeKind, app: AppHandle, state: State<'_, Arc<SharedRuntime>>) {
    if let Some(dot) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
        dot.set_theme(theme_to_native(theme));
    }
    state.update_and_emit(&app, |s| {
        s.theme = theme;
    });
}

#[tauri::command]
fn request_idle_suspend(app: AppHandle, state: State<'_, Arc<SharedRuntime>>) {
    if state.bench_mode || !state.placement().allows_webview() {
        return;
    }
    if state.snapshot_clone().expanded
        || !state.notch_visible.load(Ordering::Acquire)
        || state.want_suspended.load(Ordering::Acquire)
    {
        return;
    }
    let Some(win) = app.get_webview_window("notch") else {
        return;
    };

    let epoch = state.suspend_gen.fetch_add(1, Ordering::AcqRel).saturating_add(1);
    state.want_suspended.store(true, Ordering::Release);
    state.notch_visible.store(false, Ordering::Release);
    state.pending_wake_seq.store(0, Ordering::Release);

    if let Some(dot) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
        dot.set_standby_only(false);
        dot.show_idle();
    }

    state.update_and_emit(&app, |s| {
        s.suspended = true;
        s.activity = ActivityKind::Idle;
    });

    let app_cb = app.clone();
    let state_cb = Arc::clone(&state);
    if win.hide().is_err() {
        state.want_suspended.store(false, Ordering::Release);
        return;
    }
    let res = win.with_webview(move |wv| {
        let ctrl = wv.controller();
        if let Err(error) = webview2::try_suspend(&ctrl, move |done| match done {
            Ok(true) => {
                if state_cb.suspend_gen.load(Ordering::Acquire) == epoch
                    && state_cb.want_suspended.load(Ordering::Acquire)
                {
                    state_cb.notch_suspended.store(true, Ordering::Release);
                    tracing::debug!("Notch WebView suspended (TrySuspend)");
                } else if state_cb.placement().allows_webview()
                    && !state_cb.want_suspended.load(Ordering::Acquire)
                    && let Some(w) = app_cb.get_webview_window("notch")
                {
                    let _ = w.with_webview(|wv2| {
                        let _ = webview2::resume(&wv2.controller());
                    });
                }
            }
            Ok(false) => {
                tracing::debug!("TrySuspend returned false; keeping native idle dot active");
            }
            Err(error) => {
                tracing::warn!(%error, "TrySuspend callback failed; keeping native idle dot");
            }
        }) {
            tracing::warn!(%error, "TrySuspend call failed; keeping native idle dot");
        }
    });
    if let Err(error) = res {
        tracing::warn!(%error, "Could not access notch webview for TrySuspend");
    }
}

pub(crate) fn wake_notch_on_main(app: &AppHandle, state: &Arc<SharedRuntime>, wake_seq: u64) {
    if !state.placement().allows_webview() {
        return;
    }
    let Some(win) = app.get_webview_window("notch") else {
        return;
    };
    state.suspend_gen.fetch_add(1, Ordering::AcqRel);
    state.want_suspended.store(false, Ordering::Release);
    state.notch_suspended.store(false, Ordering::Release);
    state.pending_wake_seq.store(wake_seq.max(1), Ordering::Release);

    let resume_res = win.with_webview(|wv| {
        if let Err(error) = webview2::resume(&wv.controller()) {
            tracing::warn!(%error, "WebView2 Resume failed; keeping native feedback visible");
        }
    });
    if let Err(error) = resume_res {
        tracing::warn!(%error, "Could not resume notch WebView; keeping native feedback");
        return;
    }

    if let Some(mon) = state.active_monitor_info()
        && let Ok(hw) = win.hwnd()
    {
        let hwnd = hw.0 as isize;
        let _ = overlay_win::move_top_center(hwnd, &mon, NOTCH_CANVAS_W, NOTCH_CANVAS_H, NOTCH_TOP_OFFSET);
        let _ = win.show();
        let _ = overlay_win::ensure_topmost(hwnd);
        state.notch_hwnd.store(hwnd, Ordering::Release);
    } else {
        let _ = win.show();
    }
}

#[tauri::command]
fn cycle_monitor(app: AppHandle, state: State<'_, Arc<SharedRuntime>>) -> Result<(), String> {
    cycle_monitor_internal(&app, &state)
}

#[tauri::command]
fn open_settings(app: AppHandle, state: State<'_, Arc<SharedRuntime>>) {
    request_preview(&app, &state, Preview::Settings);
}

#[derive(Clone, Copy, Debug)]
enum Preview {
    Notch,
    Settings,
}

impl Preview {
    fn opening(self, state: &SharedRuntime) -> &AtomicBool {
        match self {
            Self::Notch => &state.notch_opening,
            Self::Settings => &state.settings_opening,
        }
    }
}

fn request_preview(app: &AppHandle, state: &Arc<SharedRuntime>, preview: Preview) {
    let generation = state.placement().generation;
    if matches!(preview, Preview::Notch) && !state.placement().allows_webview() {
        return;
    }
    if preview.opening(state).swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    let state = Arc::clone(state);
    // WebView2 creation in a synchronous menu callback can deadlock the Windows event loop.
    tauri::async_runtime::spawn_blocking(move || {
        let result = match preview {
            Preview::Notch => create_notch_window(&app, &state, generation).map(|_| {
                spawn_cursor_poll(Arc::clone(&state), false);
            }),
            Preview::Settings => create_settings_window(&app, &state),
        };
        preview.opening(&state).store(false, Ordering::Release);
        if let Err(error) = result {
            tracing::warn!(?preview, %error, "Could not open the preview window");
        }
    });
}

fn create_notch_window(app: &AppHandle, state: &Arc<SharedRuntime>, generation: u64) -> Result<isize, String> {
    if !state.placement().can_reveal(generation) {
        return Err("notch opening superseded".into());
    }
    if let Some(existing) = app.get_webview_window("notch") {
        let hwnd = existing.hwnd().map_err(|e| e.to_string())?.0 as isize;
        if !state.bench_mode && !state.notch_visible.load(Ordering::Acquire) {
            on_main_sync(app, {
                let st = Arc::clone(state);
                move |h| {
                    if !st.placement().can_reveal(generation) {
                        return Ok(());
                    }
                    let snap = st.update_and_emit(h, |s| {
                        s.suspended = false;
                        s.event_seq = s.event_seq.saturating_add(1);
                    });
                    wake_notch_on_main(h, &st, snap.event_seq);
                    Ok(())
                }
            })?;
        }
        return Ok(hwnd);
    }

    let monitor = state.active_monitor_info().ok_or_else(|| "no active monitor found".to_string())?;
    let phys_w = f64::from(monitor.logical_to_physical(NOTCH_CANVAS_W));
    let phys_x = f64::from(monitor.left) + (f64::from(monitor.width) - phys_w) * 0.5;
    let phys_y = f64::from(monitor.top + monitor.logical_to_physical(NOTCH_TOP_OFFSET));

    let win = WebviewWindowBuilder::new(app, "notch", WebviewUrl::App("index.html".into()))
        .title("Ice Audio Notch")
        .inner_size(f64::from(NOTCH_CANVAS_W), f64::from(NOTCH_CANVAS_H))
        .position(phys_x / monitor.scale_factor, phys_y / monitor.scale_factor)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .data_directory(state.wv2_data_dir.clone())
        .build()
        .map_err(|e| e.to_string())?;

    let state = Arc::clone(state);
    on_main_sync(app, move |_| {
        let monitor = state.active_monitor_info().unwrap_or(monitor);
        let configured = (|| {
            if !state.placement().can_reveal(generation) {
                return Err("notch opening superseded".into());
            }
            let hwnd = win.hwnd().map_err(|e| e.to_string())?.0 as isize;
            overlay_win::apply_overlay_styles(hwnd).map_err(|e| e.to_string())?;
            overlay_win::move_top_center(hwnd, &monitor, NOTCH_CANVAS_W, NOTCH_CANVAS_H, NOTCH_TOP_OFFSET)
                .map_err(|e| e.to_string())?;
            win.show().map_err(|e| e.to_string())?;
            overlay_win::ensure_topmost(hwnd).map_err(|e| e.to_string())?;
            state.notch_hwnd.store(hwnd, Ordering::Release);
            state.notch_visible.store(true, Ordering::Release);
            Ok(hwnd)
        })();
        if configured.is_err() {
            let _ = win.destroy();
        }
        configured
    })
}

fn create_settings_window(app: &AppHandle, state: &Arc<SharedRuntime>) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window("settings") {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }
    let win = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Ice Audio Manager — Settings")
        .inner_size(560.0, 420.0)
        .resizable(false)
        .visible(true)
        .data_directory(state.settings_wv2_data_dir.clone())
        .build()
        .map_err(|e| e.to_string())?;

    let hwnd = win.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let _ = overlay_win::apply_settings_mica(hwnd, true);
    Ok(())
}

fn spawn_cursor_poll(state: Arc<SharedRuntime>, benchmark: bool) {
    if state.cursor_poll_running.swap(true, Ordering::AcqRel) {
        return;
    }
    let poll_state = Arc::clone(&state);
    let worker = std::thread::Builder::new().name("ice-cursor-poll".into()).spawn(move || {
        let state = poll_state;
        let mut last_ignore: Option<bool> = None;
        loop {
            let hwnd = state.notch_hwnd.load(Ordering::Acquire);
            let visible = state.notch_visible.load(Ordering::Acquire);
            if hwnd == 0 || !visible {
                if !benchmark {
                    // Allow a newly opened preview to start its own worker, or keep this one.
                    state.cursor_poll_running.store(false, Ordering::Release);
                    if !state.notch_visible.load(Ordering::Acquire)
                        || state.cursor_poll_running.swap(true, Ordering::AcqRel)
                    {
                        break;
                    }
                }
                last_ignore = None;
                std::thread::sleep(Duration::from_millis(120));
                continue;
            }

            let Some((cx, cy)) = overlay_win::cursor_pos() else {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            };
            let Some((wl, wt, wr, wb)) = overlay_win::window_rect(hwnd) else {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            };

            let near = cx >= wl - 64 && cx <= wr + 64 && cy >= wt - 32 && cy <= wb + 64;
            if !near {
                if last_ignore != Some(true) {
                    let _ = overlay_win::set_click_through(hwnd, true);
                    last_ignore = Some(true);
                }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            let scale = state.active_monitor_info().map_or(1.0, |m| m.scale_factor);
            let hr = *state.hit_rect.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let pad = 6.0;
            let pl = wl + ((hr.x - pad) * scale).floor() as i32;
            let pt = wt + ((hr.y - pad) * scale).floor() as i32;
            let pr = wl + ((hr.x + hr.width + pad) * scale).ceil() as i32;
            let pb = wt + ((hr.y + hr.height + pad) * scale).ceil() as i32;

            let inside = cx >= pl && cx <= pr && cy >= pt && cy <= pb;
            let want_ignore = !inside;
            if last_ignore != Some(want_ignore) {
                let _ = overlay_win::set_click_through(hwnd, want_ignore);
                last_ignore = Some(want_ignore);
            }
            std::thread::sleep(Duration::from_millis(if benchmark { 16 } else { 34 }));
        }
    });
    if let Err(error) = worker {
        state.cursor_poll_running.store(false, Ordering::Release);
        tracing::warn!(%error, "Could not start the notch preview cursor worker");
    }
}

// ---------- Automated Phase 0.2 Benchmark Harness ----------

#[derive(Debug, Serialize)]
struct StateMemoryReport {
    state: String,
    sample_secs: u64,
    process_count: usize,
    wv2_process_count: usize,
    private_bytes_mb: f64,
    private_ws_mb: f64,
    working_set_mb: f64,
    avg_cpu_pct: f64,
    processes: Vec<ProcessReport>,
}

#[derive(Debug, Serialize)]
struct ProcessReport {
    pid: u32,
    kind: String,
    private_bytes_mb: f64,
    private_ws_mb: f64,
    working_set_mb: f64,
}

#[derive(Debug, Serialize)]
struct LatencyStatsMs {
    samples_ms: Vec<f64>,
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
}

#[derive(Debug, Serialize)]
struct BenchReport {
    monitors: Vec<MonitorInfoSer>,
    reduce_motion_flag: bool,
    focus_preserved_on_spawn: bool,
    focus_preserved_on_activity: bool,
    focus_preserved_on_monitor_move: bool,
    click_through_toggled_cleanly: bool,
    cold_first_paint_ms: f64,
    native_dot_pulse_ms: LatencyStatsMs,
    warm_event_to_render_ms: LatencyStatsMs,
    monitor_move_ms: LatencyStatsMs,
    try_suspend_resume_ms: LatencyStatsMs,
    balanced_cold_dot_react_ms: LatencyStatsMs,
    balanced_cold_wv2_paint_ms: LatencyStatsMs,
    states: Vec<StateMemoryReport>,
}

#[derive(Debug, Serialize)]
struct MonitorInfoSer {
    id: String,
    is_primary: bool,
    width: i32,
    height: i32,
    dpi_x: u32,
    scale_factor: f64,
    refresh_hz: u32,
}

fn on_main_sync<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(f(&handle));
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(20)).map_err(|e| format!("main-thread dispatch timeout: {e}"))?
}

fn collect_tree_pids(app: &AppHandle, state: &Arc<SharedRuntime>, window_label: &str) -> Vec<(u32, String)> {
    let host = (current_pid(), "host".to_string());
    let wv2_pids = on_main_sync(app, {
        let label = window_label.to_string();
        let st = Arc::clone(state);
        move |h| {
            let Some(win) = h.get_webview_window(&label) else {
                return Ok(Vec::new());
            };
            let (tx, rx) = mpsc::sync_channel(1);
            win.with_webview(move |wv| {
                let res = webview2::process_infos(&wv.environment()).map_err(|e| e.to_string());
                let _ = tx.send(res);
            })
            .map_err(|e| e.to_string())?;
            let list = rx.recv_timeout(Duration::from_secs(5)).map_err(|e| e.to_string())??;
            let mut known = st.known_wv2_pids.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            for (pid, kind) in &list {
                known.insert(*pid, kind.clone());
            }
            Ok(list)
        }
    })
    .unwrap_or_default();

    let mut out = vec![host];
    if wv2_pids.is_empty() {
        let known = state.known_wv2_pids.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        for (pid, kind) in known {
            out.push((pid, kind));
        }
    } else {
        out.extend(wv2_pids);
    }
    out
}

fn sample_state(
    app: &AppHandle,
    state: &Arc<SharedRuntime>,
    name: &str,
    window_label: &str,
    settle_secs: u64,
    sample_secs: u64,
) -> StateMemoryReport {
    std::thread::sleep(Duration::from_secs(settle_secs));
    let pids = collect_tree_pids(app, state, window_label);
    let before: TreeSnapshot = sample_pids(&pids);
    let t0 = Instant::now();
    std::thread::sleep(Duration::from_secs(sample_secs));
    let pids_after = collect_tree_pids(app, state, window_label);
    let after: TreeSnapshot = sample_pids(&pids_after);
    let avg_cpu_pct = cpu_percent_between(&before, &after, t0.elapsed());

    const MIB: f64 = 1024.0 * 1024.0;
    let wv2_process_count = after.processes.iter().filter(|p| p.kind != "host").count();
    let processes = after
        .processes
        .iter()
        .map(|p| ProcessReport {
            pid: p.pid,
            kind: p.kind.clone(),
            private_bytes_mb: round2(p.private_bytes as f64 / MIB),
            private_ws_mb: round2(p.private_working_set as f64 / MIB),
            working_set_mb: round2(p.working_set as f64 / MIB),
        })
        .collect();

    StateMemoryReport {
        state: name.to_string(),
        sample_secs,
        process_count: after.processes.len(),
        wv2_process_count,
        private_bytes_mb: round2(after.total_private_bytes as f64 / MIB),
        private_ws_mb: round2(after.total_private_ws as f64 / MIB),
        working_set_mb: round2(after.total_working_set as f64 / MIB),
        avg_cpu_pct: (avg_cpu_pct * 1000.0).round() / 1000.0,
        processes,
    }
}

fn compute_stats(samples: Vec<f64>) -> LatencyStatsMs {
    let mut sorted = samples.clone();
    sorted.sort_by(f64::total_cmp);
    let pct = |p: f64| -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
        sorted[idx.min(sorted.len() - 1)]
    };
    LatencyStatsMs {
        p50_ms: round2(pct(0.50)),
        p95_ms: round2(pct(0.95)),
        max_ms: round2(sorted.last().copied().unwrap_or(0.0)),
        samples_ms: samples.into_iter().map(round2).collect(),
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn wait_for_wv2_exit(state: &Arc<SharedRuntime>, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let known: Vec<(u32, String)> = state
            .known_wv2_pids
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        if known.is_empty() || sample_pids(&known).processes.is_empty() {
            state.known_wv2_pids.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clear();
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn spawn_notch_and_measure(app: &AppHandle, state: &Arc<SharedRuntime>) -> Result<(isize, f64), String> {
    let (tx, rx) = mpsc::sync_channel(1);
    *state.first_paint_tx.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(tx);

    let start = Instant::now();
    let monitor = state.active_monitor_info().ok_or_else(|| "no active monitor found".to_string())?;
    let phys_w = f64::from(monitor.logical_to_physical(NOTCH_CANVAS_W));
    let phys_x = f64::from(monitor.left) + (f64::from(monitor.width) - phys_w) * 0.5;
    let phys_y = f64::from(monitor.top + monitor.logical_to_physical(NOTCH_TOP_OFFSET));

    // Must be called from a non-main thread (`tauri-runtime-wry` dispatches window creation to main loop).
    let win = WebviewWindowBuilder::new(app, "notch", WebviewUrl::App("index.html".into()))
        .title("Ice Audio Notch")
        .inner_size(f64::from(NOTCH_CANVAS_W), f64::from(NOTCH_CANVAS_H))
        .position(phys_x / monitor.scale_factor, phys_y / monitor.scale_factor)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .data_directory(state.wv2_data_dir.clone())
        .build()
        .map_err(|e| e.to_string())?;

    let hwnd = on_main_sync(app, {
        let st = Arc::clone(state);
        let mon = monitor;
        move |_h| {
            let hwnd = win.hwnd().map_err(|e| e.to_string())?.0 as isize;
            overlay_win::apply_overlay_styles(hwnd).map_err(|e| e.to_string())?;
            overlay_win::move_top_center(hwnd, &mon, NOTCH_CANVAS_W, NOTCH_CANVAS_H, NOTCH_TOP_OFFSET)
                .map_err(|e| e.to_string())?;
            win.show().map_err(|e| e.to_string())?;
            overlay_win::ensure_topmost(hwnd).map_err(|e| e.to_string())?;
            st.notch_hwnd.store(hwnd, Ordering::Release);
            st.notch_visible.store(true, Ordering::Release);
            Ok(hwnd)
        }
    })?;

    let painted = rx.recv_timeout(Duration::from_secs(15)).map_err(|e| format!("first_paint timeout: {e}"))?;
    Ok((hwnd, painted.saturating_duration_since(start).as_secs_f64() * 1000.0))
}

fn destroy_notch_and_wait(app: &AppHandle, state: &Arc<SharedRuntime>) -> Result<(), String> {
    state.notch_hwnd.store(0, Ordering::Release);
    state.notch_visible.store(false, Ordering::Release);
    std::thread::sleep(Duration::from_millis(40));

    on_main_sync(app, {
        let st = Arc::clone(state);
        move |h| {
            if let Some(win) = h.get_webview_window("notch") {
                overlay_win::remove_overlay_subclass(win.hwnd().map_or(0, |hw| hw.0 as isize));
                win.destroy().map_err(|e| e.to_string())?;
            }
            if let Some(d) = st.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
                d.show_idle();
            }
            Ok(())
        }
    })?;

    let deadline = Instant::now() + Duration::from_secs(5);
    while app.get_webview_window("notch").is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    wait_for_wv2_exit(state, Duration::from_secs(10));
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

fn spawn_settings_and_wait(app: &AppHandle, state: &Arc<SharedRuntime>) -> Result<(), String> {
    let (tx, rx) = mpsc::sync_channel(1);
    *state.settings_ready_tx.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(tx);

    let win = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Ice Audio Manager — Settings")
        .inner_size(560.0, 420.0)
        .resizable(false)
        .visible(true)
        .data_directory(state.settings_wv2_data_dir.clone())
        .build()
        .map_err(|e| e.to_string())?;

    let hwnd = win.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let _ = overlay_win::apply_settings_mica(hwnd, true);
    let _ = rx.recv_timeout(Duration::from_secs(5));

    win.destroy().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(3);
    while app.get_webview_window("settings").is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(())
}

fn trigger_event_and_measure(
    app: &AppHandle,
    state: &Arc<SharedRuntime>,
    mutator: impl FnOnce(&mut NotchSnapshot) + Send + 'static,
) -> Result<f64, String> {
    let expected_seq = state.snapshot_clone().event_seq.saturating_add(1);
    let (tx, rx) = mpsc::sync_channel(1);
    *state.event_rendered_waiter.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some((expected_seq, tx));

    let start = Instant::now();
    let app_clone = app.clone();
    let st = Arc::clone(state);
    on_main_sync(&app_clone, move |h| {
        st.update_and_emit(h, mutator);
        Ok(())
    })?;
    let rendered = rx.recv_timeout(Duration::from_secs(5)).map_err(|e| format!("event_rendered timeout: {e}"))?;
    Ok(rendered.saturating_duration_since(start).as_secs_f64() * 1000.0)
}

fn suspend_notch(app: &AppHandle) -> Result<bool, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    on_main_sync(app, move |h| {
        let win = h.get_webview_window("notch").ok_or("notch missing")?;
        win.hide().map_err(|e| e.to_string())?;
        win.with_webview(move |wv| {
            let ctrl = wv.controller();
            let tx_err = tx.clone();
            if let Err(err) = webview2::try_suspend(&ctrl, move |res| {
                let _ = tx.send(res.map_err(|e| e.to_string()));
            }) {
                let _ = tx_err.send(Err(err.to_string()));
            }
        })
        .map_err(|e| e.to_string())
    })?;
    rx.recv_timeout(Duration::from_secs(10)).map_err(|e| format!("TrySuspend callback timeout: {e}"))?
}

fn resume_notch(app: &AppHandle, state: &Arc<SharedRuntime>) -> Result<(), String> {
    let st = Arc::clone(state);
    on_main_sync(app, move |h| {
        let win = h.get_webview_window("notch").ok_or("notch missing")?;
        win.with_webview(move |wv| {
            let _ = webview2::resume(&wv.controller());
        })
        .map_err(|e| e.to_string())?;
        win.show().map_err(|e| e.to_string())?;
        let hwnd = win.hwnd().map_err(|e| e.to_string())?.0 as isize;
        overlay_win::ensure_topmost(hwnd).map_err(|e| e.to_string())?;
        st.notch_visible.store(true, Ordering::Release);
        Ok(())
    })
}

fn run_benchmark(app: AppHandle, state: Arc<SharedRuntime>, sample_secs: u64, out_path: PathBuf) {
    let settle_secs = if sample_secs >= 20 { 3 } else { 1 };
    let res = run_benchmark_inner(&app, &state, settle_secs, sample_secs, &out_path);
    if let Err(err) = &res {
        eprintln!("Benchmark failed: {err}");
    }
    state.allow_exit.store(true, Ordering::Release);
    app.exit(i32::from(res.is_err()));
}

#[allow(clippy::too_many_lines)]
fn run_benchmark_inner(
    app: &AppHandle,
    state: &Arc<SharedRuntime>,
    settle_secs: u64,
    sample_secs: u64,
    out_path: &PathBuf,
) -> Result<(), String> {
    eprintln!("[bench] Starting Phase 0.2 benchmark (sample_secs={sample_secs}, settle={settle_secs}s)...");
    let monitors = state.monitors.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    let reduce_motion_flag = overlay_win::reduce_motion_enabled();
    let mut states = Vec::new();

    // 1. Native host only (zero WebView2, zero native dot)
    eprintln!("[bench] State 1/7: native_host_only");
    states.push(sample_state(app, state, "1_native_host_only", "notch", settle_secs, sample_secs));

    // 2. Native idle dot only (zero WebView2)
    eprintln!("[bench] State 2/7: native_dot_only");
    let primary = monitors.first().cloned().ok_or("no monitors")?;
    let dot = NativeDotHandle::spawn(primary).map_err(|e| e.to_string())?;
    dot.show_idle();
    let mut pulse_samples = Vec::new();
    for i in 0..10 {
        let t = Instant::now();
        dot.pulse_activity(Some((i as f32 + 1.0) / 10.0), 0x006b_c5ff);
        pulse_samples.push(t.elapsed().as_secs_f64() * 1000.0);
        std::thread::sleep(Duration::from_millis(20));
    }
    dot.show_idle();
    *state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(dot);
    states.push(sample_state(app, state, "2_native_dot_only", "notch", settle_secs, sample_secs));

    // 3. Visible mini pill (WebView2 warm)
    eprintln!("[bench] State 3/7: notch_visible_warm");
    let fg_before_spawn = overlay_win::foreground_hwnd();
    let (notch_hwnd, cold_first_paint_ms) = spawn_notch_and_measure(app, state)?;
    if let Some(d) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
        d.hide();
    }
    std::thread::sleep(Duration::from_millis(120));
    let focus_preserved_on_spawn = overlay_win::foreground_hwnd() == fg_before_spawn && fg_before_spawn != notch_hwnd;

    let mut warm_latencies = Vec::new();
    for i in 0..10u8 {
        let fg_before_act = overlay_win::foreground_hwnd();
        let ms = trigger_event_and_measure(app, state, move |s| {
            s.activity = ActivityKind::Volume;
            s.volume_percent = 30 + (i * 6);
            s.muted = false;
            s.event_seq = s.event_seq.saturating_add(1);
        })?;
        warm_latencies.push(ms);
        if i == 0 && overlay_win::foreground_hwnd() != fg_before_act {
            eprintln!("[warn] foreground changed on warm activity");
        }
        std::thread::sleep(Duration::from_millis(45));
    }
    let focus_preserved_on_activity = overlay_win::foreground_hwnd() != notch_hwnd;

    let mut move_latencies = Vec::new();
    let mut focus_preserved_on_monitor_move = true;
    if monitors.len() > 1 {
        for _ in 0..4 {
            let t = Instant::now();
            on_main_sync(app, {
                let st = Arc::clone(state);
                move |h| cycle_monitor_internal(h, &st)
            })?;
            move_latencies.push(t.elapsed().as_secs_f64() * 1000.0);
            if overlay_win::foreground_hwnd() == notch_hwnd {
                focus_preserved_on_monitor_move = false;
            }
            std::thread::sleep(Duration::from_millis(60));
        }
    }

    let click_through_toggled_cleanly = overlay_win::set_click_through(notch_hwnd, true).is_ok()
        && overlay_win::set_click_through(notch_hwnd, false).is_ok()
        && overlay_win::set_click_through(notch_hwnd, true).is_ok();

    on_main_sync(app, {
        let st = Arc::clone(state);
        move |h| {
            st.update_and_emit(h, |s| {
                s.activity = ActivityKind::Idle;
                s.expanded = false;
            });
            Ok(())
        }
    })?;
    states.push(sample_state(app, state, "3_notch_visible_warm", "notch", settle_secs, sample_secs));

    // 4. Hidden WebView2 (rAF paused)
    eprintln!("[bench] State 4/7: notch_hidden");
    on_main_sync(app, {
        let st = Arc::clone(state);
        move |h| {
            st.notch_visible.store(false, Ordering::Release);
            st.update_and_emit(h, |s| s.hidden_for_bench = true);
            if let Some(win) = h.get_webview_window("notch") {
                win.hide().map_err(|e| e.to_string())?;
            }
            Ok(())
        }
    })?;
    states.push(sample_state(app, state, "4_notch_hidden", "notch", settle_secs, sample_secs));

    // 5. TrySuspended WebView2
    eprintln!("[bench] State 5/7: notch_try_suspended");
    let suspended_ok = suspend_notch(app)?;
    eprintln!("[bench] TrySuspend result: {suspended_ok}");
    states.push(sample_state(app, state, "5_notch_try_suspended", "notch", settle_secs, sample_secs));

    let mut resume_latencies = Vec::new();
    for i in 0..3u8 {
        let t = Instant::now();
        resume_notch(app, state)?;
        let _ = trigger_event_and_measure(app, state, move |s| {
            s.hidden_for_bench = false;
            s.activity = ActivityKind::Volume;
            s.volume_percent = 50 + i * 10;
            s.event_seq = s.event_seq.saturating_add(1);
        })?;
        resume_latencies.push(t.elapsed().as_secs_f64() * 1000.0);
        if i < 2 {
            on_main_sync(app, {
                let st = Arc::clone(state);
                move |h| {
                    st.notch_visible.store(false, Ordering::Release);
                    st.update_and_emit(h, |s| s.hidden_for_bench = true);
                    Ok(())
                }
            })?;
            let _ = suspend_notch(app)?;
            std::thread::sleep(Duration::from_millis(300));
        }
    }

    // 6. Destroy WebView2 + show native dot (Balanced / Lite idle state)
    eprintln!("[bench] State 6/7: notch_destroyed_native_dot");
    destroy_notch_and_wait(app, state)?;
    eprintln!(
        "[bench] State 6/7: wv2 exited, remaining known_wv2_pids={}",
        state.known_wv2_pids.lock().unwrap_or_else(std::sync::PoisonError::into_inner).len()
    );
    states.push(sample_state(app, state, "6_notch_destroyed_native_dot", "notch", settle_secs, sample_secs));

    let mut cold_dot_latencies = Vec::new();
    let mut cold_wv2_latencies = Vec::new();
    for i in 0..5u8 {
        eprintln!("[bench] State 6/7: cold-wake cycle {}/5", i + 1);
        let t_dot = Instant::now();
        if let Some(d) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
            d.pulse_activity(Some(0.6 + (f32::from(i) * 0.06)), 0x006b_c5ff);
        }
        cold_dot_latencies.push(t_dot.elapsed().as_secs_f64() * 1000.0);

        on_main_sync(app, {
            let st = Arc::clone(state);
            move |h| {
                st.update_and_emit(h, |s| {
                    s.hidden_for_bench = false;
                    s.activity = ActivityKind::Volume;
                    s.volume_percent = 60 + i * 5;
                });
                Ok(())
            }
        })?;
        let (_, cold_ms) = spawn_notch_and_measure(app, state)?;
        cold_wv2_latencies.push(cold_ms);
        if let Some(d) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
            d.hide();
        }
        std::thread::sleep(Duration::from_millis(120));
        destroy_notch_and_wait(app, state)?;
    }

    // 7. Re-warm notch + 10x Settings open/destroy leak check
    eprintln!("[bench] State 7/7: after_10x_settings");
    on_main_sync(app, {
        let st = Arc::clone(state);
        move |h| {
            st.update_and_emit(h, |s| {
                s.hidden_for_bench = false;
                s.activity = ActivityKind::Idle;
            });
            Ok(())
        }
    })?;
    let _ = spawn_notch_and_measure(app, state)?;
    if let Some(d) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
        d.hide();
    }

    for i in 0..10 {
        eprintln!("[bench] State 7/7: settings cycle {}/10", i + 1);
        spawn_settings_and_wait(app, state)?;
        std::thread::sleep(Duration::from_millis(120));
    }
    states.push(sample_state(app, state, "7_after_10x_settings", "notch", settle_secs, sample_secs));

    let monitor_summaries = monitors
        .into_iter()
        .map(|m| MonitorInfoSer {
            id: m.id,
            is_primary: m.is_primary,
            width: m.width,
            height: m.height,
            dpi_x: m.dpi_x,
            scale_factor: m.scale_factor,
            refresh_hz: m.refresh_hz,
        })
        .collect();

    let report = BenchReport {
        monitors: monitor_summaries,
        reduce_motion_flag,
        focus_preserved_on_spawn,
        focus_preserved_on_activity,
        focus_preserved_on_monitor_move,
        click_through_toggled_cleanly,
        cold_first_paint_ms: round2(cold_first_paint_ms),
        native_dot_pulse_ms: compute_stats(pulse_samples),
        warm_event_to_render_ms: compute_stats(warm_latencies),
        monitor_move_ms: compute_stats(move_latencies),
        try_suspend_resume_ms: compute_stats(resume_latencies),
        balanced_cold_dot_react_ms: compute_stats(cold_dot_latencies),
        balanced_cold_wv2_paint_ms: compute_stats(cold_wv2_latencies),
        states,
    };

    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(out_path, &json).map_err(|e| e.to_string())?;

    println!("\n=== Phase 0.2 Overlay + Memory Benchmark ===");
    println!("Monitors: {} | SPI reduce_motion: {}", report.monitors.len(), report.reduce_motion_flag);
    println!(
        "Focus preserved: spawn={} activity={} monitor_move={} | click_through_ok={}",
        report.focus_preserved_on_spawn,
        report.focus_preserved_on_activity,
        report.focus_preserved_on_monitor_move,
        report.click_through_toggled_cleanly
    );
    println!(
        "Latencies (p50/p95 ms): native_dot={:.2}/{:.2} | warm_event={:.2}/{:.2} | resume={:.2}/{:.2} | cold_wv2={:.2}/{:.2}",
        report.native_dot_pulse_ms.p50_ms,
        report.native_dot_pulse_ms.p95_ms,
        report.warm_event_to_render_ms.p50_ms,
        report.warm_event_to_render_ms.p95_ms,
        report.try_suspend_resume_ms.p50_ms,
        report.try_suspend_resume_ms.p95_ms,
        report.balanced_cold_wv2_paint_ms.p50_ms,
        report.balanced_cold_wv2_paint_ms.p95_ms,
    );
    println!("\n| State | Procs (wv2) | Private Bytes (MB) | Private WS (MB) | Working Set (MB) | Avg CPU % |");
    println!("|---|---:|---:|---:|---:|---:|");
    for s in &report.states {
        println!(
            "| {} | {} ({}) | {:.2} | {:.2} | {:.2} | {:.3}% |",
            s.state,
            s.process_count,
            s.wv2_process_count,
            s.private_bytes_mb,
            s.private_ws_mb,
            s.working_set_mb,
            s.avg_cpu_pct
        );
    }
    println!("Wrote JSON report: {}", out_path.display());
    Ok(())
}

fn cycle_monitor_internal(app: &AppHandle, state: &Arc<SharedRuntime>) -> Result<(), String> {
    let count = state.monitors.lock().unwrap_or_else(std::sync::PoisonError::into_inner).len();
    if count <= 1 {
        return Ok(());
    }
    if !state.bench_mode {
        let next = (state.snapshot_clone().active_monitor + 1) % count;
        if let Some(monitor) = state.monitors.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(next) {
            state.engine.send(Command::SetPinnedMonitor { display_path: monitor.display_path.clone() });
        }
        return Ok(());
    }
    state.update_and_emit(app, |s| {
        s.active_monitor = (s.active_monitor + 1) % count;
    });
    if let Some(mon) = state.active_monitor_info() {
        let hwnd = state.notch_hwnd.load(Ordering::Acquire);
        if hwnd != 0 {
            overlay_win::move_top_center(hwnd, &mon, NOTCH_CANVAS_W, NOTCH_CANVAS_H, NOTCH_TOP_OFFSET)
                .map_err(|e| e.to_string())?;
        }
        if let Some(dot) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
            dot.move_to_monitor(mon);
        }
    }
    Ok(())
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}

fn main() {
    if let Err(error) = run() {
        tracing::error!(%error, "Ice Audio Manager could not start");
        eprintln!("Ice Audio Manager could not start: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let bench_mode = args.iter().any(|a| a == "--bench");
    let _instance = if bench_mode {
        None
    } else {
        let Some(guard) = InstanceGuard::acquire()? else {
            return Ok(());
        };
        Some(guard)
    };
    let sample_secs: u64 = flag_value(&args, "--sample-secs").and_then(|s| s.parse().ok()).unwrap_or(10);
    let out_path = PathBuf::from(flag_value(&args, "--out").unwrap_or_else(|| "app/out/bench.json".to_string()));

    let show_notch = args.iter().any(|arg| arg == "--show-notch") && !args.iter().any(|arg| arg == "--start-minimized");
    let app_data =
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is unavailable")?).join("IceAudioManager");
    if !bench_mode {
        logging::init(&app_data.join("logs"), args.iter().any(|arg| arg == "--verbose"))?;
    }

    let monitors = list_monitors().unwrap_or_default();
    let fake_engine = bench_mode || args.iter().any(|arg| arg == "--fake-engine");
    let engine = engine_bridge::EngineSlot::new(fake_engine.then(|| Arc::new(FakeEngine::new()) as _));
    let initial = engine.snapshot();
    tracing::debug!(revision = initial.revision, notices = ?initial.notices, "Initial audio engine snapshot");
    let wv2_data_dir = app_data.join(if bench_mode { "spike-overlay-wv2" } else { "webview" });
    let settings_wv2_data_dir =
        app_data.join(if bench_mode { "spike-overlay-wv2-settings" } else { "webview-settings" });
    let runtime =
        Arc::new(SharedRuntime::new(monitors, wv2_data_dir, settings_wv2_data_dir, bench_mode, engine, &initial));
    runtime.preview_on_attach.store(show_notch, Ordering::Release);
    runtime.start_minimized.store(args.iter().any(|arg| arg == "--start-minimized"), Ordering::Release);
    let setup_state = Arc::clone(&runtime);
    let exit_flag_ref = Arc::clone(&runtime);

    let app = tauri::Builder::default()
        .register_uri_scheme_protocol("iceart", |ctx, request| {
            let state = ctx.app_handle().state::<Arc<SharedRuntime>>();
            art::serve(&request, |id| state.engine.art(id))
        })
        .manage(Arc::clone(&runtime))
        .invoke_handler(tauri::generate_handler![
            get_notch_snapshot,
            get_engine_snapshot,
            report_hit_rect,
            report_first_paint,
            report_settings_ready,
            report_event_rendered,
            adjust_volume,
            toggle_mute,
            toggle_mic_mute,
            trigger_switch,
            switch_to_device,
            toggle_expand,
            set_preset,
            set_theme,
            request_idle_suspend,
            cycle_monitor,
            open_settings,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            if bench_mode {
                spawn_cursor_poll(Arc::clone(&setup_state), true);
                let st = Arc::clone(&setup_state);
                std::thread::Builder::new()
                    .name("ice-bench-driver".into())
                    .spawn(move || {
                        std::thread::sleep(Duration::from_millis(150));
                        run_benchmark(handle, st, sample_secs, out_path);
                    })
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            } else {
                tray::build(&handle, Arc::clone(&setup_state))?;
                tray::refresh_tooltip(&setup_state, &initial);
                if let Some(primary) =
                    setup_state.monitors.lock().unwrap_or_else(std::sync::PoisonError::into_inner).first().cloned()
                    && let Ok(dot) = NativeDotHandle::spawn(primary)
                {
                    dot.set_theme(NativePillTheme::ice());
                    dot.apply_snapshot(&initial);
                    dot.set_standby_only(false);
                    *setup_state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(dot);
                }
                placement::watch(&handle, &setup_state);
                engine_bridge::watch(handle.clone(), Arc::clone(&setup_state))?;
                let engine = if fake_engine { "fake" } else { "windows" };
                tracing::info!(engine, "Ice Audio Manager tray ready");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "notch" && matches!(event, tauri::WindowEvent::Destroyed) {
                let state = window.state::<Arc<SharedRuntime>>();
                state.notch_visible.store(false, Ordering::Release);
                state.notch_suspended.store(false, Ordering::Release);
                state.want_suspended.store(false, Ordering::Release);
                state.pending_wake_seq.store(0, Ordering::Release);
                state.notch_hwnd.store(0, Ordering::Release);
                if !state.bench_mode
                    && let Some(dot) =
                        state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref()
                {
                    dot.set_standby_only(state.placement().gaming);
                }
            }
        })
        .build(tauri::generate_context!())?;

    app.run(move |_app_handle, event| match event {
        tauri::RunEvent::Exit => {
            placement::stop(_app_handle);
            tray::close();
        }
        tauri::RunEvent::ExitRequested { api, .. } if !exit_flag_ref.allow_exit.load(Ordering::Acquire) => {
            api.prevent_exit();
        }
        _ => {}
    });
    Ok(())
}
