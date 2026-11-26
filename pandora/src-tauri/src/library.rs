//! Local-first privacy database (`%APPDATA%\Pandora\library.json`), local folder scanner,
//! and offline audio cache (`%LOCALAPPDATA%\Pandora\cache`).

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::http;
use crate::models::{EqState, LibraryState, PandoraSettings, Playlist, RecognizedTrack, Track};
use crate::ytmusic::YtMusicClient;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "opus", "aac", "weba", "webm"];
static PLAYLIST_SEQ: AtomicU64 = AtomicU64::new(1);

pub struct LibraryStore {
    db_path: PathBuf,
    cache_dir: PathBuf,
    state: Mutex<LibraryState>,
}

impl LibraryStore {
    pub fn load() -> Self {
        let appdata = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let localappdata = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| appdata.clone());
        let db_path = appdata.join("Pandora").join("library.json");
        let cache_dir = localappdata.join("Pandora").join("cache");
        Self::load_at(db_path, cache_dir)
    }

    pub fn load_at(db_path: PathBuf, cache_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&cache_dir);

        let mut loaded: LibraryState = std::fs::read_to_string(&db_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        sync_track_flags(&cache_dir, &mut loaded);
        Self { db_path, cache_dir, state: Mutex::new(loaded) }
    }

    pub fn cache_file_for(&self, track_id: &str) -> Option<PathBuf> {
        let safe = sanitize_id(track_id);
        let p = self.cache_dir.join(format!("{safe}.audio"));
        p.is_file().then_some(p)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn snapshot(&self) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        sync_track_flags(&self.cache_dir, &mut guard);
        guard.clone()
    }

    pub fn toggle_like(&self, mut track: Track) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(pos) = guard.liked.iter().position(|t| t.id == track.id) {
            guard.liked.remove(pos);
            track.liked = false;
        } else {
            track.liked = true;
            guard.liked.insert(0, track.clone());
        }
        guard.tracks_index.insert(track.id.clone(), track);
        sync_track_flags(&self.cache_dir, &mut guard);
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn record_history(&self, mut track: Track) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = guard.tracks_index.get(&track.id) {
            if track.artwork_url.is_none() {
                track.artwork_url = existing.artwork_url.clone();
            }
            if track.duration_ms.is_none() {
                track.duration_ms = existing.duration_ms;
            }
        }
        track.liked = guard.liked.iter().any(|t| t.id == track.id);
        guard.history.retain(|t| t.id != track.id);
        guard.history.insert(0, track.clone());
        guard.history.truncate(100);
        guard.tracks_index.insert(track.id.clone(), track);
        sync_track_flags(&self.cache_dir, &mut guard);
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn create_playlist(&self, name: &str, initial_track: Option<Track>) -> Result<LibraryState, String> {
        let clean = name.trim();
        if clean.is_empty() {
            return Err("Playlist name cannot be empty".into());
        }
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let seq = PLAYLIST_SEQ.fetch_add(1, Ordering::Relaxed);
        let id = format!("pl-{}-{seq}", now_ms());
        let mut track_ids = Vec::new();
        if let Some(track) = initial_track {
            track_ids.push(track.id.clone());
            guard.tracks_index.insert(track.id.clone(), track);
        }
        guard.playlists.push(Playlist {
            id,
            name: clean.to_string(),
            track_ids,
            created_at_ms: now_ms(),
        });
        sync_track_flags(&self.cache_dir, &mut guard);
        self.save_locked(&guard);
        Ok(guard.clone())
    }

    pub fn rename_playlist(&self, playlist_id: &str, name: &str) -> Result<LibraryState, String> {
        let clean = name.trim();
        if clean.is_empty() {
            return Err("Playlist name cannot be empty".into());
        }
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let pl = guard
            .playlists
            .iter_mut()
            .find(|p| p.id == playlist_id)
            .ok_or_else(|| "Playlist not found".to_string())?;
        pl.name = clean.to_string();
        self.save_locked(&guard);
        Ok(guard.clone())
    }

    pub fn delete_playlist(&self, playlist_id: &str) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.playlists.retain(|p| p.id != playlist_id);
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn add_to_playlist(&self, playlist_id: &str, track: Track) -> Result<LibraryState, String> {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.tracks_index.insert(track.id.clone(), track.clone());
        let pl = guard
            .playlists
            .iter_mut()
            .find(|p| p.id == playlist_id)
            .ok_or_else(|| "Playlist not found".to_string())?;
        if !pl.track_ids.contains(&track.id) {
            pl.track_ids.push(track.id);
        }
        sync_track_flags(&self.cache_dir, &mut guard);
        self.save_locked(&guard);
        Ok(guard.clone())
    }

    pub fn remove_from_playlist(&self, playlist_id: &str, track_id: &str) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(pl) = guard.playlists.iter_mut().find(|p| p.id == playlist_id) {
            pl.track_ids.retain(|id| id != track_id);
        }
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn save_eq(&self, eq: EqState) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.eq = eq;
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn set_ice_mode_pref(&self, pref: &str) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.ice_mode_pref = match pref {
            "embedded" | "connected" => pref.to_string(),
            _ => "auto".into(),
        };
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn save_settings(&self, mut settings: PandoraSettings) -> LibraryState {
        settings.default_volume = settings.default_volume.clamp(0.0, 1.0);
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.settings = settings;
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn reset_settings(&self) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.settings = PandoraSettings::default();
        guard.eq = EqState::default();
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn add_recognition_history(&self, mut item: RecognizedTrack) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = item.matched_track.as_mut() {
            m.liked = guard.liked.iter().any(|t| t.id == m.id);
            guard.tracks_index.insert(m.id.clone(), m.clone());
        }
        guard.recognition_history.retain(|r| r.id != item.id);
        guard.recognition_history.insert(0, item);
        guard.recognition_history.truncate(50);
        sync_track_flags(&self.cache_dir, &mut guard);
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn delete_recognition_item(&self, id: &str) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.recognition_history.retain(|r| r.id != id);
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn clear_recognition_history(&self) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.recognition_history.clear();
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn add_local_folder(&self, folder: &str) -> Result<LibraryState, String> {
        let path = PathBuf::from(folder.trim());
        if !path.is_dir() {
            return Err(format!("Folder does not exist: {}", path.display()));
        }
        let canon = path.to_string_lossy().to_string();
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !guard.local_folders.iter().any(|f| f.eq_ignore_ascii_case(&canon)) {
            guard.local_folders.push(canon);
        }
        rescan_local_locked(&mut guard);
        self.save_locked(&guard);
        Ok(guard.clone())
    }

    pub fn remove_local_folder(&self, folder: &str) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.local_folders.retain(|f| !f.eq_ignore_ascii_case(folder));
        rescan_local_locked(&mut guard);
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn rescan_local(&self) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        rescan_local_locked(&mut guard);
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn download_track(&self, track: Track, ytm: &YtMusicClient) -> Result<LibraryState, String> {
        if track.source != "ytmusic" {
            return Err("Only YouTube Music tracks can be cached offline".into());
        }
        let resolved = ytm.resolve_stream(&track.id)?;
        let safe = sanitize_id(&track.id);
        let tmp = self.cache_dir.join(format!("{safe}.audio.part"));
        let target = self.cache_dir.join(format!("{safe}.audio"));
        let mut file = std::fs::File::create(&tmp).map_err(|e| format!("Cache create error: {e}"))?;

        const CHUNK: u64 = 524_288;
        let mut total = resolved.content_length;
        let mut pos: u64 = 0;
        loop {
            let chunk_end = match total {
                Some(t) => pos.saturating_add(CHUNK - 1).min(t.saturating_sub(1)),
                None => pos.saturating_add(CHUNK - 1),
            };
            let range_val = format!("bytes={pos}-{chunk_end}");
            let mut headers = vec![("Range", range_val.as_str())];
            if let Some(ua) = resolved.user_agent.as_deref() {
                headers.push(("User-Agent", ua));
            }
            let stream = http::open_request(
                "GET",
                &resolved.url,
                &headers,
                None,
                resolved.user_agent.as_deref(),
                30_000,
            )?;
            if !(200..=299).contains(&stream.meta.status) {
                let _ = std::fs::remove_file(&tmp);
                return Err(format!("Download failed with HTTP {}", stream.meta.status));
            }
            if total.is_none() {
                total = stream
                    .meta
                    .content_range
                    .as_deref()
                    .and_then(crate::proxy::parse_total_from_content_range);
            }
            let bytes = stream.read_all((CHUNK as usize) + 65_536)?;
            if bytes.is_empty() {
                break;
            }
            file.write_all(&bytes).map_err(|e| format!("Cache write error: {e}"))?;
            pos = pos.saturating_add(bytes.len() as u64);
            if let Some(t) = total {
                if pos >= t {
                    break;
                }
            } else if (bytes.len() as u64) < CHUNK {
                break;
            }
        }
        file.sync_all().map_err(|e| format!("Cache sync error: {e}"))?;
        drop(file);
        std::fs::rename(&tmp, &target).map_err(|e| format!("Cache commit error: {e}"))?;

        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mut stored = track;
        stored.cached = true;
        guard.tracks_index.insert(stored.id.clone(), stored);
        sync_track_flags(&self.cache_dir, &mut guard);
        self.save_locked(&guard);
        Ok(guard.clone())
    }

    pub fn delete_cached_track(&self, track_id: &str) -> LibraryState {
        let safe = sanitize_id(track_id);
        let _ = std::fs::remove_file(self.cache_dir.join(format!("{safe}.audio")));
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        sync_track_flags(&self.cache_dir, &mut guard);
        self.save_locked(&guard);
        guard.clone()
    }

    pub fn clear_offline_cache(&self) -> LibraryState {
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("audio") {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        sync_track_flags(&self.cache_dir, &mut guard);
        self.save_locked(&guard);
        guard.clone()
    }

    fn save_locked(&self, state: &LibraryState) {
        if let Some(parent) = self.db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(json) = serde_json::to_string_pretty(state) else {
            return;
        };
        let tmp = self.db_path.with_extension("json.tmp");
        let written = std::fs::File::create(&tmp).and_then(|mut f| {
            f.write_all(json.as_bytes())?;
            f.sync_all()
        });
        if let Err(e) = written {
            tracing::warn!(error = %e, "Failed to write Pandora library temp file");
            let _ = std::fs::remove_file(&tmp);
            return;
        }
        for attempt in 0..5 {
            match std::fs::rename(&tmp, &self.db_path) {
                Ok(()) => return,
                Err(e) if attempt == 4 => {
                    tracing::warn!(error = %e, "Failed to commit Pandora library.json");
                }
                Err(_) => std::thread::sleep(Duration::from_millis(20 * (attempt + 1))),
            }
        }
        let _ = std::fs::remove_file(&tmp);
    }
}

fn rescan_local_locked(state: &mut LibraryState) {
    let mut found = Vec::new();
    let mut seen_paths = HashSet::new();
    for folder in &state.local_folders {
        scan_dir(Path::new(folder), 0, &mut found, &mut seen_paths);
    }
    let liked_set: HashSet<&str> = state.liked.iter().map(|t| t.id.as_str()).collect();
    for t in &mut found {
        t.liked = liked_set.contains(t.id.as_str());
    }
    for t in &found {
        state.tracks_index.insert(t.id.clone(), t.clone());
    }
    state.local_tracks = found;
}

fn scan_dir(dir: &Path, depth: usize, out: &mut Vec<Track>, seen: &mut HashSet<String>) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            scan_dir(&p, depth + 1, out, seen);
            continue;
        }
        let Some(ext) = p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase) else {
            continue;
        };
        if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        let full = p.to_string_lossy().to_string();
        if !seen.insert(full.to_ascii_lowercase()) {
            continue;
        }
        let stem = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Track")
            .trim()
            .to_string();
        let (artist, title) = if let Some((a, t)) = stem.split_once(" - ") {
            (a.trim().to_string(), t.trim().to_string())
        } else {
            let parent_name = p
                .parent()
                .and_then(|d| d.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Local Library")
                .to_string();
            (parent_name, stem)
        };
        let album = p
            .parent()
            .and_then(|d| d.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Local Files")
            .to_string();
        let id = format!("local-{:016x}", fnv1a_64(full.as_bytes()));
        out.push(Track {
            id,
            source: "local".into(),
            title,
            artist,
            album,
            duration_ms: None,
            artwork_url: None,
            local_path: Some(full),
            cached: true,
            liked: false,
        });
    }
}

fn sync_track_flags(cache_dir: &Path, state: &mut LibraryState) {
    let mut ids = Vec::new();
    let mut total_bytes = 0u64;
    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("audio") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    ids.push(stem.to_string());
                }
                if let Ok(meta) = entry.metadata() {
                    total_bytes = total_bytes.saturating_add(meta.len());
                }
            }
        }
    }
    let cached_set: HashSet<String> = ids.iter().cloned().collect();
    let liked_set: HashSet<String> = state.liked.iter().map(|t| t.id.clone()).collect();

    for t in &mut state.liked {
        t.liked = true;
        if t.source == "ytmusic" {
            t.cached = cached_set.contains(&t.id);
        }
    }
    for t in &mut state.history {
        t.liked = liked_set.contains(&t.id);
        if t.source == "ytmusic" {
            t.cached = cached_set.contains(&t.id);
        }
    }
    for t in &mut state.local_tracks {
        t.liked = liked_set.contains(&t.id);
    }
    for t in state.tracks_index.values_mut() {
        t.liked = liked_set.contains(&t.id);
        if t.source == "ytmusic" {
            t.cached = cached_set.contains(&t.id);
        }
    }
    state.cached_ids = ids;
    state.cache_bytes = total_bytes;
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf29ce484222325_u64;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_track(id: &str) -> Track {
        Track {
            id: id.into(),
            source: "ytmusic".into(),
            title: "Silver Lining".into(),
            artist: "Laufey".into(),
            album: "A Matter of Time".into(),
            duration_ms: Some(198_000),
            artwork_url: Some("https://yt3.googleusercontent.com/example".into()),
            local_path: None,
            cached: false,
            liked: false,
        }
    }

    #[test]
    fn sanitize_and_hash_are_deterministic() {
        assert_eq!(sanitize_id("abc-123_X/Y"), "abc-123_X_Y");
        assert_eq!(fnv1a_64(b"hello"), fnv1a_64(b"hello"));
    }

    #[test]
    fn toggle_like_and_record_history_keep_flags_consistent() {
        let dir = std::env::temp_dir().join(format!("pandora-lib-test-like-{}", now_ms()));
        let store = LibraryStore::load_at(dir.join("library.json"), dir.join("cache"));
        let tr = sample_track("f2PdKjdPMmU");

        let s1 = store.toggle_like(tr.clone());
        assert_eq!(s1.liked.len(), 1);
        assert!(s1.tracks_index["f2PdKjdPMmU"].liked);

        // Recording history later with liked=false must preserve liked=true!
        let s2 = store.record_history(tr.clone());
        assert_eq!(s2.liked.len(), 1);
        assert!(s2.history[0].liked);
        assert!(s2.tracks_index["f2PdKjdPMmU"].liked);

        // Unliking removes from liked and clears flag everywhere.
        let s3 = store.toggle_like(tr);
        assert!(s3.liked.is_empty());
        assert!(!s3.history[0].liked);
        assert!(!s3.tracks_index["f2PdKjdPMmU"].liked);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn playlist_lifecycle_create_rename_add_remove_delete() {
        let dir = std::env::temp_dir().join(format!("pandora-lib-test-pl-{}", now_ms()));
        let store = LibraryStore::load_at(dir.join("library.json"), dir.join("cache"));
        let t1 = sample_track("trk1");
        let t2 = sample_track("trk2");

        let s1 = store.create_playlist("Chill Mix", Some(t1.clone())).unwrap();
        assert_eq!(s1.playlists.len(), 1);
        let pl_id = s1.playlists[0].id.clone();
        assert_eq!(s1.playlists[0].track_ids, vec!["trk1"]);

        let s2 = store.rename_playlist(&pl_id, "Late Night").unwrap();
        assert_eq!(s2.playlists[0].name, "Late Night");

        let s3 = store.add_to_playlist(&pl_id, t2).unwrap();
        assert_eq!(s3.playlists[0].track_ids, vec!["trk1", "trk2"]);

        let s4 = store.remove_from_playlist(&pl_id, "trk1");
        assert_eq!(s4.playlists[0].track_ids, vec!["trk2"]);

        let s5 = store.delete_playlist(&pl_id);
        assert!(s5.playlists.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_persist_clamp_and_load_legacy_library_cleanly() {
        let dir = std::env::temp_dir().join(format!("pandora-lib-test-cfg-{}", now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("library.json");
        let legacy_json = serde_json::json!({
            "liked": [sample_track("leg1")],
            "playlists": [],
            "history": [],
            "localFolders": [],
            "localTracks": [],
            "tracksIndex": {},
            "cachedIds": [],
            "cacheBytes": 0,
            "eq": { "enabled": true, "preset": "Bass Boost", "bands": [6,0,0,0,0,0,0,0,0,0], "normalize": true, "skipSilence": false, "playbackRate": 1.0 },
            "iceModePref": "connected"
        });
        std::fs::write(&db_path, serde_json::to_string(&legacy_json).unwrap()).unwrap();

        let store = LibraryStore::load_at(db_path.clone(), dir.join("cache"));
        let snap = store.snapshot();
        assert_eq!(snap.liked.len(), 1);
        assert!(snap.settings.dynamic_theme);
        assert!((snap.settings.default_volume - 0.85).abs() < 0.001);

        let updated = store.save_settings(PandoraSettings {
            dynamic_theme: false,
            animated_canvas: false,
            show_romaji: false,
            auto_queue_related: false,
            default_volume: 1.8,
            media_session_enabled: false,
            auto_launch_ice_notch: true,
            show_ice_card_in_sidebar: false,
            wavy_progress: false,
            spinning_cover: true,
        });
        assert!(!updated.settings.dynamic_theme);
        assert!((updated.settings.default_volume - 1.0).abs() < 0.001);
        assert!(updated.settings.auto_launch_ice_notch);

        let reloaded = LibraryStore::load_at(db_path, dir.join("cache")).snapshot();
        assert!(!reloaded.settings.dynamic_theme);
        assert!(reloaded.settings.auto_launch_ice_notch);

        let reset = store.reset_settings();
        assert!(reset.settings.dynamic_theme);
        assert_eq!(reset.eq.preset, "Flat");
        assert_eq!(reset.liked.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
