//! Background `ice-lyrics` worker thread: local `.lrc` overrides (Option B), bounded LRU cache,
//! opt-in LRCLIB lookups (Option A), and offline Japanese → Romaji enrichment.

pub mod cache;
pub mod lrclib;
pub mod romaji;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

use ice_core::lyrics::{
    LyricsConfig, LyricsDocument, LyricsProvider, LyricsState, LyricsStatus, LyricsStorageStats, TrackQuery, track_key,
};
use ice_core::media::{NowPlaying, PlaybackStatus};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR};

use crate::com::ComGuard;

use self::cache::{
    CacheLookup, clear_cache as clear_disk_cache, default_lyrics_root, enforce_limit, ensure_custom_dir,
    load_local_lrc, read_cache, storage_stats as read_storage_stats, write_cache,
};
use self::lrclib::LrcLibProvider;
use self::romaji::JapaneseRomaji;

enum In {
    Track(Option<NowPlaying>),
    Config(LyricsConfig),
    ClearCache,
    Quit,
}

/// Handle to the `ice-lyrics` thread. Dropping sends `Quit` and joins cleanly.
pub struct LyricsService {
    tx: Sender<In>,
    join: Option<JoinHandle<()>>,
}

impl LyricsService {
    /// Starts the dedicated `ice-lyrics` worker. `on_state` must be non-blocking (e.g. a channel send).
    ///
    /// # Errors
    /// Fails only if the OS refuses to spawn the thread.
    pub fn start(
        initial_config: LyricsConfig,
        on_state: impl Fn(LyricsState) + Send + 'static,
    ) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let root = default_lyrics_root();
        let join = std::thread::Builder::new()
            .name("ice-lyrics".into())
            .spawn(move || run(&rx, root, initial_config, &on_state))?;
        Ok(Self { tx, join: Some(join) })
    }

    pub fn update_track(&self, media: Option<NowPlaying>) {
        let _ = self.tx.send(In::Track(media));
    }

    pub fn update_config(&self, config: LyricsConfig) {
        let _ = self.tx.send(In::Config(config));
    }

    pub fn clear_cache(&self) {
        let _ = self.tx.send(In::ClearCache);
    }
}

impl Drop for LyricsService {
    fn drop(&mut self) {
        let _ = self.tx.send(In::Quit);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// Returns current disk usage of the lyrics cache and local `.lrc` overrides.
#[must_use]
pub fn storage_stats(cache_max_mb: u16) -> LyricsStorageStats {
    LyricsStorageStats {
        romaji_reads_kanji: romaji_reads_kanji(),
        ..read_storage_stats(&default_lyrics_root(), cache_max_mb)
    }
}

/// Deletes all managed cache files (leaving user `custom\` `.lrc` files untouched).
#[must_use]
pub fn clear_lyrics_cache(cache_max_mb: u16) -> LyricsStorageStats {
    LyricsStorageStats {
        romaji_reads_kanji: romaji_reads_kanji(),
        ..clear_disk_cache(&default_lyrics_root(), cache_max_mb)
    }
}

/// Whether a Windows backend reads kanji here. Probed on a short-lived STA thread (both backends are STA classes),
/// only when the Settings window asks; installing Japanese in Windows takes effect without restarting the app.
#[must_use]
pub fn romaji_reads_kanji() -> bool {
    std::thread::Builder::new()
        .name("ice-romaji-probe".into())
        .spawn(|| {
            let _com = ComGuard::init_sta().ok();
            JapaneseRomaji::new().backend() != romaji::RomajiBackend::KanaOnly
        })
        .ok()
        .and_then(|h| h.join().ok())
        .unwrap_or(false)
}

/// Ensures `%LOCALAPPDATA%\IceAudioManager\lyrics\custom\` exists and opens it in File Explorer.
///
/// # Errors
/// Fails if the directory cannot be created.
pub fn open_custom_folder() -> Result<(), String> {
    let dir = ensure_custom_dir(&default_lyrics_root())?;
    let verb = HSTRING::from("open");
    let target = HSTRING::from(dir.as_os_str());
    // SAFETY: valid null-terminated UTF-16 verb and directory path; ShellExecuteW launches Explorer asynchronously.
    let rc = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if (rc.0 as isize) <= 32 {
        return Err("could not open lyrics folder in Explorer".into());
    }
    Ok(())
}

fn run(rx: &Receiver<In>, root: PathBuf, mut config: LyricsConfig, out: &dyn Fn(LyricsState)) {
    // STA, not MTA: both Romaji backends (`JapanesePhoneticAnalyzer`, `IFELanguage`) are single-threaded classes and
    // fail from an MTA (no proxy for `IFELanguage`; the analyzer refuses to activate). Nothing here needs a message
    // pump: every COM call is synchronous on this thread, and WinHTTP and file I/O aren't COM.
    let _com = ComGuard::init_sta().ok();
    let romaji = JapaneseRomaji::new();
    let lrclib = LrcLibProvider::new();

    let mut current_media: Option<NowPlaying> = None;
    let mut last_resolved_key: Option<String> = None;
    let mut last_state = LyricsState::default();

    let mut emit = |next: LyricsState| {
        if next != last_state {
            last_state = next.clone();
            out(next);
        }
    };

    while let Ok(first) = rx.recv() {
        // Coalesce burst updates so rapid skips only query the latest track.
        let mut latest = first;
        let mut clear_requested = matches!(latest, In::ClearCache);
        while let Ok(more) = rx.try_recv() {
            match more {
                In::Quit => return,
                In::ClearCache => clear_requested = true,
                In::Config(c) => {
                    let prev_max = config.cache_max_mb;
                    let re_resolve = c.enabled != config.enabled
                        || c.online_lookup != config.online_lookup
                        || c.romaji != config.romaji;
                    config = c;
                    if config.cache_max_mb < prev_max {
                        enforce_limit(&root, config.cache_max_mb);
                    }
                    if re_resolve {
                        last_resolved_key = None;
                    }
                }
                In::Track(m) => {
                    current_media = m;
                }
            }
            latest = In::Track(current_media.clone());
        }

        match latest {
            In::Quit => break,
            In::ClearCache => clear_requested = true,
            In::Config(c) => {
                let prev_max = config.cache_max_mb;
                let re_resolve =
                    c.enabled != config.enabled || c.online_lookup != config.online_lookup || c.romaji != config.romaji;
                config = c;
                if config.cache_max_mb < prev_max {
                    enforce_limit(&root, config.cache_max_mb);
                }
                if re_resolve {
                    last_resolved_key = None;
                }
            }
            In::Track(m) => current_media = m,
        }

        if clear_requested {
            let _ = clear_disk_cache(&root, config.cache_max_mb);
            last_resolved_key = None;
        }

        if !config.enabled {
            last_resolved_key = None;
            emit(LyricsState { status: LyricsStatus::Disabled, track_key: None, document: None });
            continue;
        }

        let Some(query) = current_media.as_ref().and_then(active_track_query) else {
            last_resolved_key = None;
            emit(LyricsState { status: LyricsStatus::Idle, track_key: None, document: None });
            continue;
        };

        if last_resolved_key.as_deref() == Some(query.track_key.as_str()) {
            continue;
        }
        last_resolved_key = Some(query.track_key.clone());

        // 1. Option B: local custom `.lrc` override (always offline, checked first).
        if let Some(mut doc) = load_local_lrc(&root, &query) {
            apply_romaji(&mut doc, config.romaji, &romaji);
            emit(LyricsState { status: LyricsStatus::Ready, track_key: Some(query.track_key), document: Some(doc) });
            continue;
        }

        // 2. Option B: managed LRU disk cache.
        match read_cache(&root, &query) {
            CacheLookup::Hit(mut doc) => {
                apply_romaji(&mut doc, config.romaji, &romaji);
                emit(LyricsState {
                    status: LyricsStatus::Ready,
                    track_key: Some(query.track_key),
                    document: Some(doc),
                });
                continue;
            }
            CacheLookup::NegativeHit => {
                emit(LyricsState { status: LyricsStatus::NotFound, track_key: Some(query.track_key), document: None });
                continue;
            }
            CacheLookup::Miss => {}
        }

        // 3. Option A: opt-in LRCLIB (`https://lrclib.net`), only when explicitly enabled in Settings → Privacy.
        if !config.online_lookup {
            emit(LyricsState { status: LyricsStatus::NotFound, track_key: Some(query.track_key), document: None });
            continue;
        }

        emit(LyricsState { status: LyricsStatus::Loading, track_key: Some(query.track_key.clone()), document: None });

        match lrclib.fetch(&query) {
            Ok(Some(mut doc)) => {
                apply_romaji(&mut doc, config.romaji, &romaji);
                write_cache(&root, &query, Some(&doc), config.cache_max_mb);
                let pending = drain_pending(rx, &mut current_media, &mut config, &mut last_resolved_key);
                if pending.quit {
                    break;
                }
                if pending.changed {
                    continue;
                }
                emit(LyricsState {
                    status: LyricsStatus::Ready,
                    track_key: Some(query.track_key),
                    document: Some(doc),
                });
            }
            Ok(None) => {
                write_cache(&root, &query, None, config.cache_max_mb);
                let pending = drain_pending(rx, &mut current_media, &mut config, &mut last_resolved_key);
                if pending.quit {
                    break;
                }
                if pending.changed {
                    continue;
                }
                emit(LyricsState { status: LyricsStatus::NotFound, track_key: Some(query.track_key), document: None });
            }
            Err(e) => {
                tracing::debug!(error = %e, "lyrics lookup skipped/failed");
                emit(LyricsState { status: LyricsStatus::NotFound, track_key: Some(query.track_key), document: None });
            }
        }
    }
}

struct PendingOutcome {
    changed: bool,
    quit: bool,
}

fn drain_pending(
    rx: &Receiver<In>,
    current_media: &mut Option<NowPlaying>,
    config: &mut LyricsConfig,
    last_resolved_key: &mut Option<String>,
) -> PendingOutcome {
    let mut out = PendingOutcome { changed: false, quit: false };
    while let Ok(msg) = rx.try_recv() {
        match msg {
            In::Track(m) => {
                let next_key = m.as_ref().and_then(active_track_query).map(|q| q.track_key);
                if next_key != *last_resolved_key {
                    out.changed = true;
                    *last_resolved_key = None;
                }
                *current_media = m;
            }
            In::Config(c) => {
                if c != *config {
                    *config = c;
                    *last_resolved_key = None;
                    out.changed = true;
                }
            }
            In::ClearCache => {
                *last_resolved_key = None;
                out.changed = true;
            }
            In::Quit => {
                out.quit = true;
                return out;
            }
        }
    }
    out
}

fn active_track_query(m: &NowPlaying) -> Option<TrackQuery> {
    if matches!(m.status, PlaybackStatus::Closed | PlaybackStatus::Stopped) {
        return None;
    }
    if m.title.trim().is_empty() && m.artist.trim().is_empty() {
        return None;
    }
    Some(TrackQuery {
        track_key: track_key(&m.app_id, &m.title, &m.artist, &m.album),
        title: m.title.clone(),
        artist: m.artist.clone(),
        album: m.album.clone(),
        duration_ms: m.duration_ms,
    })
}

/// Fills `LyricLine::romaji` (never `subtitle`, which is the source's own paired line) once per loaded song.
/// The cache never stores it (`cache::write_cache`), so it's regenerated on load and generator fixes apply everywhere.
fn apply_romaji(doc: &mut LyricsDocument, enabled: bool, romaji: &JapaneseRomaji) {
    for line in &mut doc.lines {
        line.romaji = (enabled && line.subtitle.is_none()).then(|| romaji.line_to_romaji(&line.text)).flatten();
    }
}
