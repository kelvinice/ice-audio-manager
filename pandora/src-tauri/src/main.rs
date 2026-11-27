#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod artcache;
mod companion;
mod http;
mod ice_bridge;
mod library;
mod lyrics;
mod models;
mod playlist_import;
mod proxy;
mod recognition;
mod recommend;
mod ytm_feed;
mod ytmusic;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use ice_core::engine::{Command, EngineSnapshot, RoleSet, SpatialMode};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};

use crate::artcache::ArtCache;
use crate::ice_bridge::IceBridge;
use crate::library::LibraryStore;
use crate::models::{
    AppPaths, CollectionPage, EqState, HomeFeed, IceConfigPatch, IceStatus, LibraryState, PandoraLyrics, PandoraSettings,
    PlaylistImportResult, RecognizedTrack, SearchResponse, Track,
};
use crate::ytmusic::YtMusicClient;

struct AppState {
    proxy_port: u16,
    demo_mode: bool,
    ytm: Arc<YtMusicClient>,
    library: Arc<LibraryStore>,
    art: Arc<ArtCache>,
    ice: Arc<IceBridge>,
    recognition_cancel: Arc<AtomicBool>,
    /// Set while a recognition listens (the Recognize view and Ice share one capture).
    recognizing: AtomicBool,
    import_cancel: Arc<AtomicBool>,
    quitting: Arc<AtomicBool>,
}

/// Leaves for real (the close button only hides the window when "Minimize to tray" is on).
fn quit(app: &AppHandle) {
    app.state::<Arc<AppState>>().quitting.store(true, std::sync::atomic::Ordering::Release);
    app.exit(0);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    quit(&app);
}

#[tauri::command]
fn get_proxy_port(state: State<'_, Arc<AppState>>) -> u16 {
    state.proxy_port
}

#[tauri::command]
fn is_demo_mode(state: State<'_, Arc<AppState>>) -> bool {
    state.demo_mode
}

#[tauri::command]
async fn ytm_home(state: State<'_, Arc<AppState>>) -> Result<HomeFeed, String> {
    let ytm = Arc::clone(&state.ytm);
    tauri::async_runtime::spawn_blocking(move || ytm.home())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn ytm_search(
    query: String,
    filter: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<SearchResponse, String> {
    let ytm = Arc::clone(&state.ytm);
    let flt = filter.unwrap_or_else(|| "songs".into());
    tauri::async_runtime::spawn_blocking(move || ytm.search(&query, &flt))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn ytm_related(video_id: String, state: State<'_, Arc<AppState>>) -> Result<Vec<Track>, String> {
    let ytm = Arc::clone(&state.ytm);
    tauri::async_runtime::spawn_blocking(move || ytm.related(&video_id))
        .await
        .map_err(|e| e.to_string())
}

/// Songs to add to a playlist, from YouTube Music radios of its songs (`recommend.rs`).
#[tauri::command]
async fn ytm_recommend(
    tracks: Vec<Track>,
    round: u32,
    skip: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Track>, String> {
    let ytm = Arc::clone(&state.ytm);
    tauri::async_runtime::spawn_blocking(move || recommend::recommend(&ytm, &tracks, round, &skip))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn fetch_lyrics(
    title: String,
    artist: String,
    album: String,
    duration_ms: Option<u64>,
    lrc_override: Option<String>,
) -> Result<PandoraLyrics, String> {
    tauri::async_runtime::spawn_blocking(move || {
        lyrics::fetch_lyrics(&title, &artist, &album, duration_ms, lrc_override.as_deref())
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_library(state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.snapshot()
}

#[tauri::command]
fn toggle_like(track: Track, state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.toggle_like(track)
}

#[tauri::command]
fn record_history(track: Track, state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.record_history(track)
}

#[tauri::command]
fn create_playlist(
    name: String,
    initial_track: Option<Track>,
    state: State<'_, Arc<AppState>>,
) -> Result<LibraryState, String> {
    state.library.create_playlist(&name, initial_track)
}

#[tauri::command]
fn rename_playlist(
    playlist_id: String,
    name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<LibraryState, String> {
    state.library.rename_playlist(&playlist_id, &name)
}

#[tauri::command]
fn delete_playlist(playlist_id: String, state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.delete_playlist(&playlist_id)
}

#[tauri::command]
fn add_to_playlist(
    playlist_id: String,
    track: Track,
    state: State<'_, Arc<AppState>>,
) -> Result<LibraryState, String> {
    state.library.add_to_playlist(&playlist_id, track)
}

#[tauri::command]
fn remove_from_playlist(
    playlist_id: String,
    track_id: String,
    state: State<'_, Arc<AppState>>,
) -> LibraryState {
    state.library.remove_from_playlist(&playlist_id, &track_id)
}

#[tauri::command]
fn save_eq(eq: EqState, state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.save_eq(eq)
}

#[tauri::command]
fn save_settings(settings: PandoraSettings, state: State<'_, Arc<AppState>>) -> LibraryState {
    state.art.set_budget_mb(settings.artwork_cache_mb);
    state.library.save_settings(settings)
}

#[tauri::command]
fn artwork_cache_bytes(state: State<'_, Arc<AppState>>) -> u64 {
    state.art.used_bytes()
}

#[tauri::command]
async fn clear_artwork_cache(state: State<'_, Arc<AppState>>) -> Result<u64, String> {
    let art = Arc::clone(&state.art);
    tauri::async_runtime::spawn_blocking(move || art.clear()).await.map_err(|e| e.to_string())
}

/// Saves an opened YouTube Music collection as a library playlist; returns the library and the new playlist id.
#[tauri::command]
fn save_collection(
    name: String,
    tracks: Vec<Track>,
    state: State<'_, Arc<AppState>>,
) -> Result<(LibraryState, String), String> {
    state.library.create_playlist_with_tracks(&name, tracks).map(|(library, playlist)| (library, playlist.id))
}

#[tauri::command]
async fn ytm_collection(browse_id: String, state: State<'_, Arc<AppState>>) -> Result<CollectionPage, String> {
    let ytm = Arc::clone(&state.ytm);
    tauri::async_runtime::spawn_blocking(move || ytm.collection(&browse_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn reset_settings(state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.reset_settings()
}

#[tauri::command]
fn get_app_paths(state: State<'_, Arc<AppState>>) -> AppPaths {
    AppPaths {
        library_path: state.library.db_path().display().to_string(),
        cache_dir: state.library.cache_dir().display().to_string(),
        artwork_dir: state.art.dir().display().to_string(),
        ice_config_path: state.ice.config_path().display().to_string(),
        ice_bridge_path: state.ice.bridge_info_path().display().to_string(),
    }
}

#[tauri::command]
async fn add_local_folder(path: String, state: State<'_, Arc<AppState>>) -> Result<LibraryState, String> {
    let lib = Arc::clone(&state.library);
    tauri::async_runtime::spawn_blocking(move || lib.add_local_folder(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn remove_local_folder(path: String, state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.remove_local_folder(&path)
}

#[tauri::command]
async fn rescan_local(state: State<'_, Arc<AppState>>) -> Result<LibraryState, String> {
    let lib = Arc::clone(&state.library);
    tauri::async_runtime::spawn_blocking(move || lib.rescan_local())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn download_track(track: Track, state: State<'_, Arc<AppState>>) -> Result<LibraryState, String> {
    let lib = Arc::clone(&state.library);
    let ytm = Arc::clone(&state.ytm);
    tauri::async_runtime::spawn_blocking(move || lib.download_track(track, &ytm))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn delete_cached_track(track_id: String, state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.delete_cached_track(&track_id)
}

#[tauri::command]
fn clear_offline_cache(state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.clear_offline_cache()
}

#[tauri::command]
fn ice_status(state: State<'_, Arc<AppState>>) -> IceStatus {
    state.ice.status()
}

#[tauri::command]
fn ice_snapshot(state: State<'_, Arc<AppState>>) -> EngineSnapshot {
    state.ice.snapshot()
}

#[tauri::command]
fn ice_set_mode_pref(pref: String, state: State<'_, Arc<AppState>>) -> Result<IceStatus, String> {
    state.library.set_ice_mode_pref(&pref);
    state.ice.set_mode_pref(&pref);
    Ok(state.ice.status())
}

#[tauri::command]
fn ice_switch_device(device_id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.ice.send_command(Command::SwitchTo { device_id, roles: RoleSet::Default })
}

#[tauri::command]
fn ice_set_volume(percent: u8, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.ice.send_command(Command::SetVolume { percent })
}

#[tauri::command]
fn ice_toggle_mute(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.ice.send_command(Command::ToggleMute)
}

#[tauri::command]
fn ice_toggle_mic_mute(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.ice.send_command(Command::ToggleMicMute)
}

#[tauri::command]
fn ice_set_spatial(
    device_id: String,
    mode: SpatialMode,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.ice.send_command(Command::SetSpatial { device_id, mode })
}

#[tauri::command]
fn ice_patch_settings(
    patch: IceConfigPatch,
    state: State<'_, Arc<AppState>>,
) -> Result<EngineSnapshot, String> {
    state.ice.patch_settings(patch)
}

#[tauri::command]
fn ice_set_meter_active(active: bool, state: State<'_, Arc<AppState>>) {
    state.ice.set_meter_active(active);
}

#[tauri::command]
async fn ice_launch_standalone(state: State<'_, Arc<AppState>>) -> Result<IceStatus, String> {
    let ice = Arc::clone(&state.ice);
    tauri::async_runtime::spawn_blocking(move || ice.launch_standalone())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn ice_stop_standalone(state: State<'_, Arc<AppState>>) -> Result<IceStatus, String> {
    let ice = Arc::clone(&state.ice);
    tauri::async_runtime::spawn_blocking(move || ice.stop_standalone())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn ice_ping_bridge(state: State<'_, Arc<AppState>>) -> Result<IceStatus, String> {
    let ice = Arc::clone(&state.ice);
    tauri::async_runtime::spawn_blocking(move || ice.ping_bridge())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn ice_toggle_notch(state: State<'_, Arc<AppState>>) -> Result<IceStatus, String> {
    let ice = Arc::clone(&state.ice);
    tauri::async_runtime::spawn_blocking(move || ice.toggle_notch())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn ice_show_notch(state: State<'_, Arc<AppState>>) -> Result<IceStatus, String> {
    let ice = Arc::clone(&state.ice);
    tauri::async_runtime::spawn_blocking(move || ice.show_notch())
        .await
        .map_err(|e| e.to_string())?
}

/// How long the title bar's Ice button shows the notch card.
const ICE_PEEK_MS: u64 = 2_000;

#[tauri::command]
async fn ice_wake_and_peek(state: State<'_, Arc<AppState>>) -> Result<IceStatus, String> {
    let ice = Arc::clone(&state.ice);
    tauri::async_runtime::spawn_blocking(move || ice.wake_and_peek(ICE_PEEK_MS))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn ice_open_settings(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let ice = Arc::clone(&state.ice);
    tauri::async_runtime::spawn_blocking(move || ice.open_ice_settings())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn recognize_music(source: String, app: AppHandle) -> Result<RecognizedTrack, String> {
    let cap_src = recognition::CaptureSource::from_str(&source);
    tauri::async_runtime::spawn_blocking(move || recognize_now(&app, cap_src))
        .await
        .map_err(|e| e.to_string())?
}

/// One recognition at a time, from the Recognize view or from Ice's notch widget (`companion.rs`). Blocks up to ~10 s.
/// Saves the match in the history and tells the UI, whichever side asked.
fn recognize_now(app: &AppHandle, source: recognition::CaptureSource) -> Result<RecognizedTrack, String> {
    use std::sync::atomic::Ordering;
    let state = app.state::<Arc<AppState>>();
    if state.recognizing.swap(true, Ordering::AcqRel) {
        return Err("Pandora is already listening.".into());
    }
    let result = recognition::recognize_from_audio(source, &state.recognition_cancel, app, &state.ytm);
    state.recognizing.store(false, Ordering::Release);
    let matched = result?;
    let library = state.library.add_recognition_history(matched.clone());
    let _ = app.emit("library://changed/v1", &library);
    Ok(matched)
}

#[tauri::command]
fn cancel_recognition(state: State<'_, Arc<AppState>>) {
    state
        .recognition_cancel
        .store(true, std::sync::atomic::Ordering::Release);
}

#[tauri::command]
fn delete_recognition_item(id: String, state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.delete_recognition_item(&id)
}

#[tauri::command]
fn clear_recognition_history(state: State<'_, Arc<AppState>>) -> LibraryState {
    state.library.clear_recognition_history()
}

#[tauri::command]
async fn import_playlist(
    input: String,
    name_override: Option<String>,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<PlaylistImportResult, String> {
    let ytm = Arc::clone(&state.ytm);
    let lib = Arc::clone(&state.library);
    let cancel = Arc::clone(&state.import_cancel);
    tauri::async_runtime::spawn_blocking(move || {
        playlist_import::import_playlist(&input, name_override.as_deref(), &ytm, &lib, &cancel, &app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn cancel_playlist_import(state: State<'_, Arc<AppState>>) {
    state
        .import_cancel
        .store(true, std::sync::atomic::Ordering::Release);
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let visible = win.is_visible().unwrap_or(false);
        let minimized = win.is_minimized().unwrap_or(false);
        if visible && !minimized {
            let _ = win.hide();
        } else {
            let _ = win.unminimize();
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let fake_engine = std::env::args().any(|a| a == "--fake-engine");
    let demo_mode = std::env::args().any(|a| a == "--demo");
    // Started by Ice for a background job (recognition): stay in the tray until the user opens the window.
    let background = std::env::args().any(|a| a == ice_core::companion::BACKGROUND_ARG);
    let library = Arc::new(LibraryStore::load());
    let ytm = Arc::new(YtMusicClient::new());
    let initial_lib = library.snapshot();
    let art_dir = library.cache_dir().parent().map_or_else(|| library.cache_dir().join("artwork"), |p| p.join("artwork"));
    let art = Arc::new(ArtCache::open(art_dir, initial_lib.settings.artwork_cache_mb));
    let pref = initial_lib.ice_mode_pref;
    let auto_launch_ice = initial_lib.settings.auto_launch_ice_notch && !fake_engine;
    let ice = IceBridge::new(pref, fake_engine);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let proxy_port = rt
        .block_on(proxy::start(Arc::clone(&ytm), Arc::clone(&library), Arc::clone(&art)))
        .expect("local media proxy failed to bind");
    tracing::info!(proxy_port, demo_mode, "Pandora local audio/art proxy ready");

    let state = Arc::new(AppState {
        proxy_port,
        demo_mode,
        ytm,
        library,
        art,
        ice: Arc::clone(&ice),
        recognition_cancel: Arc::new(AtomicBool::new(false)),
        recognizing: AtomicBool::new(false),
        import_cancel: Arc::new(AtomicBool::new(false)),
        quitting: Arc::new(AtomicBool::new(false)),
    });

    tauri::Builder::default()
        .manage(state)
        .setup(move |app| {
            ice.attach_app(app.handle().clone());
            companion::start(app.handle());
            if auto_launch_ice {
                let ice_launch = Arc::clone(&ice);
                std::thread::Builder::new()
                    .name("pandora-ice-autolaunch".into())
                    .spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        let _ = ice_launch.launch_standalone();
                    })
                    .ok();
            }
            if let Some(win) = app.get_webview_window("main") {
                ice_bridge::apply_window_backdrop(&win);
                // The window starts hidden (tauri.conf.json) so a background start never flashes it.
                if !background {
                    let _ = win.show();
                }
            }
            let show_item = MenuItem::with_id(app, "tray-show", "Show Pandora", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "tray-quit", "Quit Pandora", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &sep, &quit_item])?;
            let mut tray_builder = TrayIconBuilder::with_id("pandora-tray")
                .tooltip("Pandora")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "tray-show" => show_main_window(app),
                    "tray-quit" => quit(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                    | TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    } => toggle_main_window(tray.app_handle()),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }
            if let Err(e) = tray_builder.build(app) {
                tracing::warn!(error = %e, "Failed to initialize Pandora system tray icon");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let state = window.state::<Arc<AppState>>();
                    if !state.quitting.load(std::sync::atomic::Ordering::Acquire)
                        && state.library.close_to_tray()
                    {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_proxy_port,
            quit_app,
            is_demo_mode,
            ytm_home,
            ytm_collection,
            save_collection,
            artwork_cache_bytes,
            clear_artwork_cache,
            ytm_search,
            ytm_related,
            ytm_recommend,
            fetch_lyrics,
            get_library,
            toggle_like,
            record_history,
            create_playlist,
            rename_playlist,
            delete_playlist,
            add_to_playlist,
            remove_from_playlist,
            save_eq,
            save_settings,
            reset_settings,
            get_app_paths,
            add_local_folder,
            remove_local_folder,
            rescan_local,
            download_track,
            delete_cached_track,
            clear_offline_cache,
            ice_status,
            ice_snapshot,
            ice_set_mode_pref,
            ice_switch_device,
            ice_set_volume,
            ice_toggle_mute,
            ice_toggle_mic_mute,
            ice_set_spatial,
            ice_patch_settings,
            ice_set_meter_active,
            ice_launch_standalone,
            ice_stop_standalone,
            ice_ping_bridge,
            ice_toggle_notch,
            ice_show_notch,
            ice_wake_and_peek,
            ice_open_settings,
            recognize_music,
            cancel_recognition,
            delete_recognition_item,
            clear_recognition_history,
            import_playlist,
            cancel_playlist_import,
        ])
        .build(tauri::generate_context!())
        .expect("error running Pandora")
        .run(|_, event| {
            if let RunEvent::Exit = event {
                companion::stop();
            }
        });
}
