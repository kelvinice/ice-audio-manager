//! Option B: local `.lrc` overrides and bounded LRU disk cache (`%LOCALAPPDATA%\IceAudioManager\lyrics`).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ice_core::lyrics::{
    LyricsDocument, LyricsSource, LyricsStorageStats, LyricsSyncKind, TrackQuery, parse_lrc, sanitize_filename,
    track_cache_key,
};
use serde::{Deserialize, Serialize};

/// Ceiling for any single `.lrc` or cached JSON file (guards against malformed/huge files).
pub const MAX_ENTRY_BYTES: u64 = 256 * 1024;
/// Negative cache TTL for LRCLIB 404 misses (24 hours).
const NEGATIVE_TTL_MS: u64 = 24 * 60 * 60 * 1000;
/// Minimum age before a cache hit rewrites the entry on disk to bump `accessed_ms`.
const ACCESS_TOUCH_INTERVAL_MS: u64 = 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedRecord {
    /// [`CACHE_FORMAT`] the record was written with; older records are dropped and looked up again.
    #[serde(default)]
    pub format: u32,
    pub accessed_ms: u64,
    pub not_found_at_ms: Option<u64>,
    pub document: Option<LyricsDocument>,
}

/// Format 2 (2026-11-24): generated Romaji moved from `subtitle` to `LyricLine::romaji` and is no longer cached.
/// Format 0/1 records mixed generated Romaji into `subtitle`, indistinguishable from a source's paired lines.
pub const CACHE_FORMAT: u32 = 2;

/// Outcome of checking the managed disk cache.
#[derive(Debug, Clone)]
pub enum CacheLookup {
    Hit(LyricsDocument),
    NegativeHit,
    Miss,
}

#[must_use]
pub fn default_lyrics_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("IceAudioManager")
        .join("lyrics")
}

#[must_use]
pub fn cache_dir(root: &Path) -> PathBuf {
    root.join("cache")
}

#[must_use]
pub fn custom_dir(root: &Path) -> PathBuf {
    root.join("custom")
}

pub fn now_unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).ok().and_then(|d| u64::try_from(d.as_millis()).ok()).unwrap_or(0)
}

/// Option B: checks `%LOCALAPPDATA%\IceAudioManager\lyrics\custom\` for `<Artist> - <Title>.lrc` or `<Title>.lrc`.
#[must_use]
pub fn load_local_lrc(root: &Path, query: &TrackQuery) -> Option<LyricsDocument> {
    let dir = custom_dir(root);
    let entries = fs::read_dir(&dir).ok()?;
    let artist = sanitize_filename(&query.artist);
    let title = sanitize_filename(&query.title);
    if title.is_empty() {
        return None;
    }
    let mut candidates = Vec::new();
    if !artist.is_empty() {
        candidates.push(format!("{artist} - {title}.lrc").to_lowercase());
    }
    candidates.push(format!("{title}.lrc").to_lowercase());

    let mut matched: Option<(usize, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let lower = name.to_lowercase();
        if let Some(rank) = candidates.iter().position(|c| *c == lower)
            && matched.as_ref().is_none_or(|(best, _)| rank < *best)
        {
            matched = Some((rank, path));
        }
    }
    let (_, path) = matched?;
    let meta = fs::metadata(&path).ok()?;
    if meta.len() == 0 || meta.len() > MAX_ENTRY_BYTES {
        tracing::debug!(path = %path.display(), len = meta.len(), "ignoring oversized or empty local .lrc");
        return None;
    }
    let raw = fs::read_to_string(&path).ok()?;
    let (sync_kind, lines) = parse_lrc(&raw, query.duration_ms);
    if lines.is_empty() {
        let plain = raw.trim();
        if plain.is_empty() {
            return None;
        }
        return Some(LyricsDocument {
            track_key: query.track_key.clone(),
            source: LyricsSource::Local,
            sync_kind: LyricsSyncKind::Plain,
            instrumental: false,
            lines: Vec::new(),
            plain_text: Some(plain.to_string()),
        });
    }
    Some(LyricsDocument {
        track_key: query.track_key.clone(),
        source: LyricsSource::Local,
        sync_kind,
        instrumental: false,
        lines,
        plain_text: None,
    })
}

fn cache_entry_path(root: &Path, query: &TrackQuery) -> PathBuf {
    let key = track_cache_key(&query.title, &query.artist, &query.album, query.duration_ms);
    cache_dir(root).join(format!("{key}.json"))
}

/// Reads from the managed disk cache and refreshes `accessed_ms` on hit.
#[must_use]
pub fn read_cache(root: &Path, query: &TrackQuery) -> CacheLookup {
    let path = cache_entry_path(root, query);
    let Ok(meta) = fs::metadata(&path) else { return CacheLookup::Miss };
    if meta.len() > MAX_ENTRY_BYTES {
        let _ = fs::remove_file(&path);
        return CacheLookup::Miss;
    }
    let Ok(bytes) = fs::read(&path) else { return CacheLookup::Miss };
    let Ok(mut record) = serde_json::from_slice::<CachedRecord>(&bytes) else {
        let _ = fs::remove_file(&path);
        return CacheLookup::Miss;
    };
    if record.format != CACHE_FORMAT {
        let _ = fs::remove_file(&path);
        return CacheLookup::Miss;
    }
    let now = now_unix_ms();
    if let Some(missed_at) = record.not_found_at_ms {
        if now.saturating_sub(missed_at) < NEGATIVE_TTL_MS {
            return CacheLookup::NegativeHit;
        }
        let _ = fs::remove_file(&path);
        return CacheLookup::Miss;
    }
    let Some(mut doc) = record.document.clone() else { return CacheLookup::Miss };
    doc.track_key.clone_from(&query.track_key);
    doc.source = LyricsSource::Cache;
    if now.saturating_sub(record.accessed_ms) >= ACCESS_TOUCH_INTERVAL_MS {
        record.accessed_ms = now;
        if let Ok(updated) = serde_json::to_vec(&record) {
            let _ = write_atomic(&path, &updated);
        }
    }
    CacheLookup::Hit(doc)
}

/// Persists a fetched document (or 24 h negative miss) atomically and enforces `cache_max_mb`.
pub fn write_cache(root: &Path, query: &TrackQuery, doc: Option<&LyricsDocument>, cache_max_mb: u16) {
    let dir = cache_dir(root);
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let now = now_unix_ms();
    // Generated Romaji is per-PC output, not source data: strip it so the cache only holds what the source sent.
    let document = doc.cloned().map(|mut d| {
        for line in &mut d.lines {
            line.romaji = None;
        }
        d
    });
    let record = CachedRecord {
        format: CACHE_FORMAT,
        accessed_ms: now,
        not_found_at_ms: doc.is_none().then_some(now),
        document,
    };
    let Ok(bytes) = serde_json::to_vec(&record) else { return };
    if (bytes.len() as u64) > MAX_ENTRY_BYTES {
        return;
    }
    let path = cache_entry_path(root, query);
    let _ = write_atomic(&path, &bytes);
    enforce_limit(root, cache_max_mb);
}

/// Evicts least-recently-used cache entries until total disk size ≤ `cache_max_mb` MiB.
pub fn enforce_limit(root: &Path, cache_max_mb: u16) {
    let dir = cache_dir(root);
    let Ok(entries) = fs::read_dir(&dir) else { return };
    let max_bytes = u64::from(cache_max_mb) * 1024 * 1024;
    let mut candidates: Vec<(u64, u64, PathBuf)> = Vec::new();
    let mut total = 0u64;

    // Pass 1: metadata only. When under the limit (the common case), skip reading/parsing every cached file.
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let len = meta.len();
        total = total.saturating_add(len);
        let mtime = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| u64::try_from(d.as_millis()).ok())
            .unwrap_or(0);
        candidates.push((mtime, len, path));
    }
    if total <= max_bytes {
        return;
    }

    // Pass 2: only when over the cap, read `accessed_ms` (falling back to `mtime`) to evict oldest first.
    let mut files: Vec<(u64, u64, PathBuf)> = candidates
        .into_iter()
        .map(|(mtime, len, path)| {
            let accessed = fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<CachedRecord>(&b).ok())
                .map_or(mtime, |r| r.accessed_ms);
            (accessed, len, path)
        })
        .collect();
    files.sort_by_key(|(accessed, _, _)| *accessed);
    for (_, len, path) in files {
        if total <= max_bytes {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}

/// Computes live storage statistics for `Settings → Storage` (never polled).
#[must_use]
pub fn storage_stats(root: &Path, cache_max_mb: u16) -> LyricsStorageStats {
    let mut cache_bytes = 0u64;
    let mut cache_files = 0u32;
    if let Ok(entries) = fs::read_dir(cache_dir(root)) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|e| e.to_str()) == Some("json")
                && let Ok(meta) = entry.metadata()
            {
                cache_bytes = cache_bytes.saturating_add(meta.len());
                cache_files = cache_files.saturating_add(1);
            }
        }
    }
    let mut custom_files = 0u32;
    let custom = custom_dir(root);
    if let Ok(entries) = fs::read_dir(&custom) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("lrc"))
            {
                custom_files = custom_files.saturating_add(1);
            }
        }
    }
    LyricsStorageStats {
        cache_bytes,
        cache_files,
        custom_files,
        cache_max_mb,
        custom_dir: custom.display().to_string(),
        // Filled in by `super::storage_stats` (needs an STA thread); file-level stats don't know.
        romaji_reads_kanji: false,
    }
}

/// Clears all managed cache entries (leaving `custom\` user `.lrc` overrides untouched).
pub fn clear_cache(root: &Path, cache_max_mb: u16) -> LyricsStorageStats {
    if let Ok(entries) = fs::read_dir(cache_dir(root)) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let _ = fs::remove_file(&path);
            }
        }
    }
    storage_stats(root, cache_max_mb)
}

/// Ensures `%LOCALAPPDATA%\IceAudioManager\lyrics\custom\` exists and returns its path.
///
/// # Errors
/// Fails if the directory cannot be created.
pub fn ensure_custom_dir(root: &Path) -> Result<PathBuf, String> {
    let dir = custom_dir(root);
    fs::create_dir_all(&dir).map_err(|e| format!("could not create lyrics folder: {e}"))?;
    Ok(dir)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    if fs::rename(&tmp, path).is_err() {
        let _ = fs::remove_file(path);
        fs::rename(&tmp, path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ice-lyrics-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn query(artist: &str, title: &str) -> TrackQuery {
        TrackQuery {
            track_key: format!("Spotify.exe\0{title}\0{artist}\0Album"),
            title: title.into(),
            artist: artist.into(),
            album: "Album".into(),
            duration_ms: Some(180_000),
        }
    }

    #[test]
    fn loads_local_lrc_artist_title_before_title_only() {
        let root = temp_root("local");
        let dir = ensure_custom_dir(&root).unwrap();
        fs::write(dir.join("Idol.lrc"), "[00:01.00]Fallback title").unwrap();
        fs::write(dir.join("YOASOBI - Idol.lrc"), "[00:02.00]Exact match").unwrap();
        let doc = load_local_lrc(&root, &query("YOASOBI", "Idol")).unwrap();
        assert_eq!(doc.source, LyricsSource::Local);
        assert_eq!(doc.lines[0].text, "Exact match");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cache_keeps_paired_subtitles_drops_generated_romaji_and_old_formats() {
        let root = temp_root("romaji");
        let q = query("YOASOBI", "Idol");
        let (sync_kind, mut lines) =
            ice_core::lyrics::parse_lrc("[00:01.00]無敵\n[00:01.00]Invincible\n[00:03.00]笑顔", None);
        lines[1].romaji = Some("egao".into());
        let doc = LyricsDocument {
            track_key: "k".into(),
            source: LyricsSource::Lrclib,
            sync_kind,
            instrumental: false,
            lines,
            plain_text: None,
        };
        write_cache(&root, &q, Some(&doc), 50);
        let CacheLookup::Hit(read) = read_cache(&root, &q) else { panic!("expected a cache hit") };
        assert_eq!(read.lines[0].subtitle.as_deref(), Some("Invincible"), "source's paired line is kept");
        assert_eq!(read.lines[1].romaji, None, "generated Romaji is never cached");

        // A record from before format 2 (Romaji mixed into `subtitle`) is dropped and looked up again.
        let path = cache_entry_path(&root, &q);
        let mut old: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        old.as_object_mut().unwrap().remove("format");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        assert!(matches!(read_cache(&root, &q), CacheLookup::Miss));
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cache_round_trips_negative_hits_and_clears_without_touching_custom() {
        let root = temp_root("cache");
        let cdir = ensure_custom_dir(&root).unwrap();
        fs::write(cdir.join("Keep.lrc"), "[00:01.00]Keep me").unwrap();

        let q1 = query("Artist A", "Track 1");
        let (_, lines) = parse_lrc("[00:05.00]Hello", Some(180_000));
        let doc = LyricsDocument {
            track_key: q1.track_key.clone(),
            source: LyricsSource::Lrclib,
            sync_kind: LyricsSyncKind::Line,
            instrumental: false,
            lines,
            plain_text: None,
        };
        write_cache(&root, &q1, Some(&doc), 50);
        assert!(matches!(read_cache(&root, &q1), CacheLookup::Hit(d) if d.source == LyricsSource::Cache));

        let q2 = query("Artist B", "Missing");
        write_cache(&root, &q2, None, 50);
        assert!(matches!(read_cache(&root, &q2), CacheLookup::NegativeHit));

        let stats = storage_stats(&root, 50);
        assert_eq!(stats.cache_files, 2);
        assert_eq!(stats.custom_files, 1);

        let cleared = clear_cache(&root, 50);
        assert_eq!(cleared.cache_files, 0);
        assert_eq!(cleared.custom_files, 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lru_eviction_removes_oldest_entries_when_over_limit() {
        let root = temp_root("lru");
        let cdir = cache_dir(&root);
        fs::create_dir_all(&cdir).unwrap();
        let payload = "x".repeat(600 * 1024); // ~600 KB raw string split across several entries
        for i in 0..10u64 {
            let rec = CachedRecord {
                format: CACHE_FORMAT,
                accessed_ms: 1_000 + i * 100,
                not_found_at_ms: None,
                document: Some(LyricsDocument {
                    track_key: format!("k{i}"),
                    source: LyricsSource::Lrclib,
                    sync_kind: LyricsSyncKind::Plain,
                    instrumental: false,
                    lines: Vec::new(),
                    plain_text: Some(payload[..150_000].to_string()),
                }),
            };
            fs::write(cdir.join(format!("entry{i}.json")), serde_json::to_vec(&rec).unwrap()).unwrap();
        }
        // 10 * ~150 KB = ~1.5 MB; enforcing 1 MB ceiling must evict oldest entries first.
        enforce_limit(&root, 1);
        assert!(!cdir.join("entry0.json").exists(), "oldest entry should be evicted");
        assert!(cdir.join("entry9.json").exists(), "newest entry should remain");
        let stats = storage_stats(&root, 1);
        assert!(stats.cache_bytes <= 1024 * 1024);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn read_cache_skips_rewrite_when_fresh_and_touches_when_stale() {
        let root = temp_root("touch-interval");
        let q = query("Artist T", "Track T");
        let (_, lines) = parse_lrc("[00:01.00]Hello", Some(180_000));
        let doc = LyricsDocument {
            track_key: q.track_key.clone(),
            source: LyricsSource::Lrclib,
            sync_kind: LyricsSyncKind::Line,
            instrumental: false,
            lines,
            plain_text: None,
        };
        write_cache(&root, &q, Some(&doc), 50);
        let path = cache_entry_path(&root, &q);
        let first: CachedRecord = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let recent_accessed = now_unix_ms().saturating_sub(60_000);
        let mut pinned = first.clone();
        pinned.accessed_ms = recent_accessed;
        fs::write(&path, serde_json::to_vec(&pinned).unwrap()).unwrap();

        assert!(matches!(read_cache(&root, &q), CacheLookup::Hit(_)));
        let after_fresh_read: CachedRecord = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(after_fresh_read.accessed_ms, recent_accessed, "fresh hit should not rewrite JSON");

        pinned.accessed_ms = now_unix_ms().saturating_sub(ACCESS_TOUCH_INTERVAL_MS * 2);
        fs::write(&path, serde_json::to_vec(&pinned).unwrap()).unwrap();
        assert!(matches!(read_cache(&root, &q), CacheLookup::Hit(_)));
        let after_stale_read: CachedRecord = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(after_stale_read.accessed_ms > pinned.accessed_ms, "stale hit refreshes accessed_ms");
        let _ = fs::remove_dir_all(&root);
    }
}
