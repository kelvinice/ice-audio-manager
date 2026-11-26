//! Local-first privacy database (`%APPDATA%\Pandora\library.json`), local folder scanner,
//! and offline audio cache (`%LOCALAPPDATA%\Pandora\cache`).

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::http;
use crate::models::{EqState, LibraryState, Playlist, Track};
use crate::ytmusic::YtMusicClient;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "opus", "aac", "weba", "webm"];

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
        let _ = std::fs::create_dir_all(&cache_dir);

        let mut loaded: LibraryState = std::fs::read_to_string(&db_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        refresh_cache_stats(&cache_dir, &mut loaded);
        Self { db_path, cache_dir, state: Mutex::new(loaded) }
    }

    pub fn cache_file_for(&self, track_id: &str) -> Option<PathBuf> {
        let safe = sanitize_id(track_id);
        let p = self.cache_dir.join(format!("{safe}.audio"));
        p.is_file().then_some(p)
    }

    pub fn snapshot(&self) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        refresh_cache_stats(&self.cache_dir, &mut guard);
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
        self.save_locked(&guard);
        refresh_cache_stats(&self.cache_dir, &mut guard);
        guard.clone()
    }

    pub fn record_history(&self, track: Track) -> LibraryState {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.history.retain(|t| t.id != track.id);
        guard.history.insert(0, track.clone());
        guard.history.truncate(100);
        guard.tracks_index.insert(track.id.clone(), track);
        self.save_locked(&guard);
        refresh_cache_stats(&self.cache_dir, &mut guard);
        guard.clone()
    }

    pub fn create_playlist(&self, name: &str) -> Result<LibraryState, String> {
        let clean = name.trim();
        if clean.is_empty() {
            return Err("Playlist name cannot be empty".into());
        }
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let id = format!("pl-{}", now_ms());
        guard.playlists.push(Playlist {
            id,
            name: clean.to_string(),
            track_ids: Vec::new(),
            created_at_ms: now_ms(),
        });
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
        refresh_cache_stats(&self.cache_dir, &mut guard);
        self.save_locked(&guard);
        Ok(guard.clone())
    }

    pub fn delete_cached_track(&self, track_id: &str) -> LibraryState {
        let safe = sanitize_id(track_id);
        let _ = std::fs::remove_file(self.cache_dir.join(format!("{safe}.audio")));
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        refresh_cache_stats(&self.cache_dir, &mut guard);
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
        refresh_cache_stats(&self.cache_dir, &mut guard);
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
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &self.db_path);
        }
    }
}

fn rescan_local_locked(state: &mut LibraryState) {
    let mut found = Vec::new();
    let mut seen_paths = HashSet::new();
    for folder in &state.local_folders {
        scan_dir(Path::new(folder), 0, &mut found, &mut seen_paths);
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

fn refresh_cache_stats(cache_dir: &Path, state: &mut LibraryState) {
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
    let cached_set: HashSet<&str> = ids.iter().map(String::as_str).collect();
    for t in &mut state.liked {
        if t.source == "ytmusic" {
            t.cached = cached_set.contains(t.id.as_str());
        }
    }
    for t in &mut state.history {
        if t.source == "ytmusic" {
            t.cached = cached_set.contains(t.id.as_str());
        }
    }
    for t in state.tracks_index.values_mut() {
        if t.source == "ytmusic" {
            t.cached = cached_set.contains(t.id.as_str());
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

    #[test]
    fn sanitize_and_hash_are_deterministic() {
        assert_eq!(sanitize_id("abc-123_X/Y"), "abc-123_X_Y");
        assert_eq!(fnv1a_64(b"hello"), fnv1a_64(b"hello"));
    }
}
