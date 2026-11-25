//! Background `ice-lyrics` worker thread: local `.lrc` overrides (Option B), bounded LRU cache,
//! opt-in non-blocking LRCLIB lookups (Option A) on short-lived `ice-lyrics-net` threads, and offline
//! Japanese → Romaji enrichment.

pub mod cache;
pub mod lrclib;
pub mod romaji;
pub mod words;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ice_core::lyrics::{
    LyricsConfig, LyricsDocument, LyricsState, LyricsStatus, LyricsStorageStats, LyricsSyncKind, TrackQuery,
    contains_kanji, estimate_word_timings, track_key,
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

/// Wait before starting an online LRCLIB request so rapid track skips only query the settled track.
const ONLINE_DEBOUNCE: Duration = Duration::from_millis(250);
/// Upper bound on concurrent `ice-lyrics-net` worker threads.
const MAX_INFLIGHT_HTTP: usize = 2;
/// Backoff before checking slot availability again when `MAX_INFLIGHT_HTTP` fetches are already running.
const INFLIGHT_RETRY_DELAY: Duration = Duration::from_millis(120);

type FetchFn =
    Arc<dyn Fn(&TrackQuery, &(dyn Fn() -> bool + Sync)) -> Result<Option<LyricsDocument>, String> + Send + Sync>;

enum In {
    Track(Option<NowPlaying>),
    Config(LyricsConfig),
    ClearCache,
    FetchDone { epoch: u64, query: TrackQuery, result: Result<Option<LyricsDocument>, String> },
    Quit,
}

/// Handle to the `ice-lyrics` thread. Dropping aborts any in-flight work, sends `Quit`, and joins cleanly.
pub struct LyricsService {
    tx: Sender<In>,
    active_gen: Arc<AtomicU64>,
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
        let provider = Arc::new(LrcLibProvider::new());
        let fetcher: FetchFn = Arc::new(move |query, cancelled| provider.fetch_cancellable(query, cancelled));
        Self::start_with(initial_config, default_lyrics_root(), fetcher, ONLINE_DEBOUNCE, on_state)
    }

    fn start_with(
        initial_config: LyricsConfig,
        root: PathBuf,
        fetcher: FetchFn,
        online_debounce: Duration,
        on_state: impl Fn(LyricsState) + Send + 'static,
    ) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let worker_tx = tx.clone();
        let active_gen = Arc::new(AtomicU64::new(0));
        let worker_gen = Arc::clone(&active_gen);
        let join = std::thread::Builder::new().name("ice-lyrics".into()).spawn(move || {
            run(&rx, &worker_tx, root, initial_config, worker_gen, fetcher, online_debounce, &on_state)
        })?;
        Ok(Self { tx, active_gen, join: Some(join) })
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
        self.active_gen.fetch_add(1, Ordering::AcqRel);
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
    shell_open(&HSTRING::from(dir.as_os_str())).map_err(|()| "could not open lyrics folder in Explorer".into())
}

/// Windows' Language & region settings page, where the user adds Japanese so Romaji can read kanji
/// ([`romaji::JapaneseRomaji::recheck`] picks it up on the next song). A fixed target: the app never installs anything
/// itself (no admin, code-practices §1).
///
/// # Errors
/// Fails if Windows can't open the Settings page.
pub fn open_language_settings() -> Result<(), String> {
    shell_open(&HSTRING::from("ms-settings:regionlanguage"))
        .map_err(|()| "could not open Windows language settings".into())
}

fn shell_open(target: &HSTRING) -> Result<(), ()> {
    let verb = HSTRING::from("open");
    // SAFETY: valid null-terminated UTF-16 verb and target; ShellExecuteW opens it asynchronously.
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
    // ShellExecute reports success as a value above 32.
    if (rc.0 as isize) <= 32 { Err(()) } else { Ok(()) }
}

struct InflightGuard(Arc<AtomicUsize>);

impl Drop for InflightGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn try_acquire_slot(inflight: &Arc<AtomicUsize>, limit: usize) -> Option<InflightGuard> {
    let mut cur = inflight.load(Ordering::Acquire);
    while cur < limit {
        match inflight.compare_exchange_weak(cur, cur + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return Some(InflightGuard(Arc::clone(inflight))),
            Err(actual) => cur = actual,
        }
    }
    None
}

fn spawn_online_fetch(
    tx: &Sender<In>,
    active_gen: &Arc<AtomicU64>,
    inflight: &Arc<AtomicUsize>,
    fetcher: &FetchFn,
    epoch: u64,
    query: TrackQuery,
) -> bool {
    let Some(guard) = try_acquire_slot(inflight, MAX_INFLIGHT_HTTP) else {
        return false;
    };
    let tx = tx.clone();
    let active_gen = Arc::clone(active_gen);
    let fetcher = Arc::clone(fetcher);
    std::thread::Builder::new()
        .name("ice-lyrics-net".into())
        .spawn(move || {
            let _guard = guard;
            let result = fetcher(&query, &|| active_gen.load(Ordering::Acquire) != epoch);
            let _ = tx.send(In::FetchDone { epoch, query, result });
        })
        .is_ok()
}

#[expect(clippy::too_many_arguments, reason = "single worker entry keeps state local to the STA thread")]
fn run(
    rx: &Receiver<In>,
    tx: &Sender<In>,
    root: PathBuf,
    mut config: LyricsConfig,
    active_gen: Arc<AtomicU64>,
    fetcher: FetchFn,
    online_debounce: Duration,
    out: &dyn Fn(LyricsState),
) {
    // STA, not MTA: both Romaji backends (`JapanesePhoneticAnalyzer`, `IFELanguage`) are single-threaded classes and
    // fail from an MTA (no proxy for `IFELanguage`; the analyzer refuses to activate). Nothing here needs a message
    // pump: every COM call is synchronous on this thread, and WinHTTP and file I/O aren't COM.
    let _com = ComGuard::init_sta().ok();
    let mut romaji = JapaneseRomaji::new();
    let inflight = Arc::new(AtomicUsize::new(0));

    let mut current_media: Option<NowPlaying> = None;
    let mut last_resolved_key: Option<String> = None;
    let mut pending_online: Option<(Instant, u64, TrackQuery)> = None;
    let mut last_state = LyricsState::default();

    let mut emit = |next: LyricsState| {
        if next != last_state {
            last_state = next.clone();
            out(next);
        }
    };

    loop {
        let first = match &pending_online {
            Some((deadline, _, _)) => match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(m) => m,
                Err(RecvTimeoutError::Timeout) => {
                    if let Some((_, epoch, query)) = pending_online.take()
                        && epoch == active_gen.load(Ordering::Acquire)
                        && !spawn_online_fetch(tx, &active_gen, &inflight, &fetcher, epoch, query.clone())
                    {
                        pending_online = Some((Instant::now() + INFLIGHT_RETRY_DELAY, epoch, query));
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(m) => m,
                Err(_) => break,
            },
        };

        let mut clear_requested = false;
        let mut finished = Vec::new();
        let mut quit = false;

        for msg in std::iter::once(first).chain(std::iter::from_fn(|| rx.try_recv().ok())) {
            match msg {
                In::Quit => {
                    quit = true;
                    break;
                }
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
                        pending_online = None;
                        active_gen.fetch_add(1, Ordering::AcqRel);
                    }
                }
                In::Track(m) => {
                    let prev_key = current_media.as_ref().and_then(active_track_query).map(|q| q.track_key);
                    let next_key = m.as_ref().and_then(active_track_query).map(|q| q.track_key);
                    if next_key != prev_key {
                        active_gen.fetch_add(1, Ordering::AcqRel);
                        pending_online = None;
                        last_resolved_key = None;
                    }
                    current_media = m;
                }
                In::FetchDone { epoch, query, result } => finished.push((epoch, query, result)),
            }
        }

        if quit {
            active_gen.fetch_add(1, Ordering::AcqRel);
            break;
        }

        if clear_requested {
            let _ = clear_disk_cache(&root, config.cache_max_mb);
            last_resolved_key = None;
            pending_online = None;
            active_gen.fetch_add(1, Ordering::AcqRel);
        }

        let current_gen = active_gen.load(Ordering::Acquire);
        let active_key = current_media.as_ref().and_then(active_track_query).map(|q| q.track_key);

        for (epoch, query, result) in finished {
            let is_current = epoch == current_gen && config.enabled && active_key.as_ref() == Some(&query.track_key);
            match result {
                Ok(Some(mut doc)) => {
                    if !clear_requested {
                        write_cache(&root, &query, Some(&doc), config.cache_max_mb);
                    }
                    if is_current && apply_romaji_cancellable(&mut doc, config.romaji, &mut romaji, &active_gen, epoch)
                    {
                        emit(LyricsState {
                            status: LyricsStatus::Ready,
                            track_key: Some(query.track_key),
                            document: Some(doc),
                        });
                    }
                }
                Ok(None) => {
                    if !clear_requested {
                        write_cache(&root, &query, None, config.cache_max_mb);
                    }
                    if is_current {
                        emit(LyricsState {
                            status: LyricsStatus::NotFound,
                            track_key: Some(query.track_key),
                            document: None,
                        });
                    }
                }
                Err(e) => {
                    if e != "cancelled" {
                        tracing::debug!(error = %e, "lyrics lookup skipped/failed");
                    }
                    if is_current {
                        emit(LyricsState {
                            status: LyricsStatus::NotFound,
                            track_key: Some(query.track_key),
                            document: None,
                        });
                    }
                }
            }
        }

        if !config.enabled {
            last_resolved_key = None;
            pending_online = None;
            emit(LyricsState { status: LyricsStatus::Disabled, track_key: None, document: None });
            continue;
        }

        let Some(query) = current_media.as_ref().and_then(active_track_query) else {
            last_resolved_key = None;
            pending_online = None;
            emit(LyricsState { status: LyricsStatus::Idle, track_key: None, document: None });
            continue;
        };

        if last_resolved_key.as_deref() == Some(query.track_key.as_str()) {
            continue;
        }
        last_resolved_key = Some(query.track_key.clone());

        // 1. Option B: local custom `.lrc` override (always offline, checked first).
        if let Some(mut doc) = load_local_lrc(&root, &query) {
            if apply_romaji_cancellable(&mut doc, config.romaji, &mut romaji, &active_gen, current_gen) {
                emit(LyricsState {
                    status: LyricsStatus::Ready,
                    track_key: Some(query.track_key),
                    document: Some(doc),
                });
            }
            continue;
        }

        // 2. Option B: managed LRU disk cache.
        match read_cache(&root, &query) {
            CacheLookup::Hit(mut doc) => {
                refresh_line_word_cadence(&mut doc);
                if apply_romaji_cancellable(&mut doc, config.romaji, &mut romaji, &active_gen, current_gen) {
                    emit(LyricsState {
                        status: LyricsStatus::Ready,
                        track_key: Some(query.track_key),
                        document: Some(doc),
                    });
                }
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
        pending_online = Some((Instant::now() + online_debounce, current_gen, query));
    }
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

/// Refreshes estimated word timings for cached line-synced documents so cadence improvements apply without
/// invalidating the offline cache.
fn refresh_line_word_cadence(doc: &mut LyricsDocument) {
    if doc.sync_kind != LyricsSyncKind::Line {
        return;
    }
    for line in &mut doc.lines {
        if line.text.trim().is_empty() {
            continue;
        }
        let end_ms = line.end_ms.unwrap_or_else(|| line.start_ms.saturating_add(5_000));
        line.words = estimate_word_timings(&line.text, line.start_ms, end_ms);
    }
}

/// Fills `LyricLine::romaji` (never `subtitle`, which is the source's own paired line) once per loaded song,
/// stopping early if `active_gen` moves away from `expected_gen`.
fn apply_romaji_cancellable(
    doc: &mut LyricsDocument,
    enabled: bool,
    romaji: &mut JapaneseRomaji,
    active_gen: &AtomicU64,
    expected_gen: u64,
) -> bool {
    if enabled && doc.lines.iter().any(|l| l.subtitle.is_none() && contains_kanji(&l.text)) {
        romaji.recheck();
    }
    for line in &mut doc.lines {
        if active_gen.load(Ordering::Acquire) != expected_gen {
            return false;
        }
        line.romaji = (enabled && line.subtitle.is_none()).then(|| romaji.line_to_romaji(&line.text)).flatten();
    }
    active_gen.load(Ordering::Acquire) == expected_gen
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;

    use ice_core::lyrics::{LyricsSource, LyricsSyncKind, parse_lrc};

    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ice-lyrics-svc-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn song(title: &str) -> NowPlaying {
        NowPlaying {
            app_id: "Spotify.exe".into(),
            title: title.into(),
            artist: "Artist".into(),
            album: "Album".into(),
            status: PlaybackStatus::Playing,
            position_ms: Some(0),
            duration_ms: Some(180_000),
            position_updated_ms: None,
            art: None,
        }
    }

    #[test]
    fn slow_online_fetch_does_not_block_or_starve_subsequent_cached_tracks_or_drop() {
        let root = temp_root("nonblocking");
        let cdir = ensure_custom_dir(&root).unwrap();
        fs::write(cdir.join("Artist - FastLocal.lrc"), "[00:01.00]Local lyric").unwrap();

        let (fetch_started_tx, fetch_started_rx) = mpsc::channel();
        let (states_tx, states_rx) = mpsc::channel();
        let fetcher: FetchFn = Arc::new(move |q, cancelled| {
            let _ = fetch_started_tx.send(q.title.clone());
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if cancelled() {
                    return Err("cancelled".into());
                }
                std::thread::sleep(Duration::from_millis(15));
            }
            let (_, lines) = parse_lrc("[00:01.00]Slow remote", q.duration_ms);
            Ok(Some(LyricsDocument {
                track_key: q.track_key.clone(),
                source: LyricsSource::Lrclib,
                sync_kind: LyricsSyncKind::Line,
                instrumental: false,
                lines,
                plain_text: None,
            }))
        });

        let cfg = LyricsConfig { enabled: true, online_lookup: true, ..LyricsConfig::default() };
        let svc = LyricsService::start_with(cfg, root.clone(), fetcher, Duration::from_millis(20), move |st| {
            let _ = states_tx.send(st);
        })
        .unwrap();

        // Track A starts a slow remote fetch.
        svc.update_track(Some(song("SlowRemote")));
        assert_eq!(fetch_started_rx.recv_timeout(Duration::from_secs(2)).unwrap(), "SlowRemote");

        // While SlowRemote is blocked, switch to FastLocal: it must resolve promptly without waiting 5s.
        let t0 = Instant::now();
        svc.update_track(Some(song("FastLocal")));
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut resolved_fast = false;
        while Instant::now() < deadline {
            if let Ok(st) = states_rx.recv_timeout(Duration::from_millis(200))
                && st.status == LyricsStatus::Ready
                && st.document.as_ref().is_some_and(|d| d.lines[0].text == "Local lyric")
            {
                resolved_fast = true;
                break;
            }
        }
        assert!(resolved_fast, "FastLocal should resolve immediately while SlowRemote is aborted");
        assert!(t0.elapsed() < Duration::from_secs(1));

        // Now trigger another slow remote lookup and drop immediately: Drop must not block for 5s.
        svc.update_track(Some(song("SlowRemote2")));
        assert_eq!(fetch_started_rx.recv_timeout(Duration::from_secs(2)).unwrap(), "SlowRemote2");
        let drop_start = Instant::now();
        drop(svc);
        assert!(drop_start.elapsed() < Duration::from_millis(500), "Drop should exit without waiting on HTTP");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn config_then_track_burst_preserves_config_and_ignores_stale_fetch_results() {
        let root = temp_root("burst-and-stale");
        let (states_tx, states_rx) = mpsc::channel();
        let (release_a_tx, release_a_rx) = mpsc::channel();
        let release_a_rx = std::sync::Mutex::new(release_a_rx);
        let fetcher: FetchFn = Arc::new(move |q, _| {
            if q.title == "SongA" {
                let _ = release_a_rx.lock().unwrap().recv_timeout(Duration::from_secs(2));
                return Err("network boom".into());
            }
            let (_, lines) = parse_lrc("[00:01.00]Song B synced", q.duration_ms);
            Ok(Some(LyricsDocument {
                track_key: q.track_key.clone(),
                source: LyricsSource::Lrclib,
                sync_kind: LyricsSyncKind::Line,
                instrumental: false,
                lines,
                plain_text: None,
            }))
        });

        let initial_cfg = LyricsConfig { enabled: false, online_lookup: false, ..LyricsConfig::default() };
        let svc = LyricsService::start_with(initial_cfg, root.clone(), fetcher, Duration::from_millis(20), move |st| {
            let _ = states_tx.send(st);
        })
        .unwrap();

        // Burst: Config(enabled + online) immediately followed by Track(SongA).
        svc.update_config(LyricsConfig { enabled: true, online_lookup: true, ..LyricsConfig::default() });
        svc.update_track(Some(song("SongA")));

        let expected_a_key = active_track_query(&song("SongA")).unwrap().track_key;
        let mut saw_loading_a = false;
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Ok(st) = states_rx.recv_timeout(Duration::from_millis(200))
                && st.status == LyricsStatus::Loading
                && st.track_key.as_deref() == Some(expected_a_key.as_str())
            {
                saw_loading_a = true;
                break;
            }
        }
        assert!(saw_loading_a, "Config + Track burst must apply both and start Loading for SongA");

        // Switch to SongB before SongA finishes; release SongA to return an error.
        svc.update_track(Some(song("SongB")));
        let _ = release_a_tx.send(());

        let mut saw_ready_b = false;
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Ok(st) = states_rx.recv_timeout(Duration::from_millis(200)) {
                assert_ne!(
                    (st.status, st.track_key.as_deref()),
                    (LyricsStatus::NotFound, Some(active_track_query(&song("SongA")).unwrap().track_key.as_str())),
                    "stale error for SongA must never emit after moving to SongB"
                );
                if st.status == LyricsStatus::Ready
                    && st.document.as_ref().is_some_and(|d| d.lines[0].text == "Song B synced")
                {
                    saw_ready_b = true;
                    break;
                }
            }
        }
        assert!(saw_ready_b, "SongB should finish cleanly without being clobbered by stale SongA error");
        drop(svc);
        let _ = fs::remove_dir_all(&root);
    }
}
