//! Disk cache for remote artwork (`%LOCALAPPDATA%\Pandora\artwork`).
//!
//! Files are named by the SHA-256 of the source URL plus an extension from the content type, so any URL maps to
//! one fixed, filesystem-safe name and nothing about the URL leaks into the path. A hit refreshes the file's
//! modified time; when the folder grows past its budget the least recently used files go first, down to 90 %.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use sha2::{Digest, Sha256};

const MIB: u64 = 1024 * 1024;
const KNOWN: [(&str, &str); 4] =
    [("jpg", "image/jpeg"), ("png", "image/png"), ("webp", "image/webp"), ("gif", "image/gif")];

pub struct ArtCache {
    dir: PathBuf,
    budget: AtomicU64,
    used: AtomicU64,
    /// Serialises eviction so two writers do not scan and delete at once.
    evicting: Mutex<()>,
}

impl ArtCache {
    pub fn open(dir: PathBuf, budget_mb: u32) -> Self {
        let _ = fs::create_dir_all(&dir);
        let used = scan(&dir).iter().map(|entry| entry.1).sum();
        Self {
            dir,
            budget: AtomicU64::new(u64::from(budget_mb) * MIB),
            used: AtomicU64::new(used),
            evicting: Mutex::new(()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn used_bytes(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    pub fn set_budget_mb(&self, budget_mb: u32) {
        self.budget.store(u64::from(budget_mb) * MIB, Ordering::Relaxed);
        self.evict_if_needed();
    }

    /// The cached image for `url`, as `(content type, bytes)`.
    pub fn get(&self, url: &str) -> Option<(String, Vec<u8>)> {
        let key = key(url);
        KNOWN.iter().find_map(|(ext, content_type)| {
            let path = self.dir.join(format!("{key}.{ext}"));
            let bytes = fs::read(&path).ok()?;
            // Mark as recently used; failure only makes the entry evict sooner.
            let _ = fs::File::options().write(true).open(&path).and_then(|f| f.set_modified(SystemTime::now()));
            Some(((*content_type).to_string(), bytes))
        })
    }

    /// Stores `bytes` for `url` unless caching is off or the type is not a raster image we know.
    pub fn put(&self, url: &str, content_type: &str, bytes: &[u8]) {
        let budget = self.budget.load(Ordering::Relaxed);
        let size = bytes.len() as u64;
        let Some(ext) = ext_for(content_type) else { return };
        if budget == 0 || size == 0 || size > budget / 4 {
            return;
        }
        let key = key(url);
        let target = self.dir.join(format!("{key}.{ext}"));
        let part = self.dir.join(format!("{key}.part"));
        if fs::write(&part, bytes).and_then(|()| fs::rename(&part, &target)).is_err() {
            let _ = fs::remove_file(&part);
            return;
        }
        self.used.fetch_add(size, Ordering::Relaxed);
        self.evict_if_needed();
    }

    /// Deletes every cached image and returns the bytes freed.
    pub fn clear(&self) -> u64 {
        let _guard = self.evicting.lock();
        let mut freed = 0;
        for (path, size, _) in scan(&self.dir) {
            if fs::remove_file(path).is_ok() {
                freed += size;
            }
        }
        self.used.store(scan(&self.dir).iter().map(|entry| entry.1).sum(), Ordering::Relaxed);
        freed
    }

    fn evict_if_needed(&self) {
        let budget = self.budget.load(Ordering::Relaxed);
        if self.used.load(Ordering::Relaxed) <= budget {
            return;
        }
        let Ok(_guard) = self.evicting.try_lock() else { return };
        let mut entries = scan(&self.dir);
        let mut used: u64 = entries.iter().map(|entry| entry.1).sum();
        entries.sort_by_key(|entry| entry.2);
        let target = budget / 10 * 9;
        for (path, size, _) in entries {
            if used <= target {
                break;
            }
            if fs::remove_file(path).is_ok() {
                used = used.saturating_sub(size);
            }
        }
        self.used.store(used, Ordering::Relaxed);
    }
}

fn key(url: &str) -> String {
    Sha256::digest(url.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn ext_for(content_type: &str) -> Option<&'static str> {
    let mime = content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    KNOWN
        .iter()
        .find(|(_, known)| *known == mime || (mime == "image/jpg" && *known == "image/jpeg"))
        .map(|(ext, _)| *ext)
}

/// Cached image files as `(path, size, modified)`; skips partial writes and anything else in the folder.
fn scan(dir: &Path) -> Vec<(PathBuf, u64, SystemTime)> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let ext = path.extension()?.to_str()?;
            if !KNOWN.iter().any(|(known, _)| *known == ext) {
                return None;
            }
            let meta = entry.metadata().ok()?;
            Some((path, meta.len(), meta.modified().unwrap_or(SystemTime::UNIX_EPOCH)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cache(name: &str, budget_mb: u32) -> ArtCache {
        let dir = std::env::temp_dir().join(format!("pandora-artcache-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        ArtCache::open(dir, budget_mb)
    }

    #[test]
    fn round_trips_by_url_hash() {
        let cache = temp_cache("roundtrip", 8);
        cache.put("https://i.ytimg.com/a.jpg", "image/jpeg", b"jpeg-bytes");
        assert_eq!(cache.get("https://i.ytimg.com/a.jpg"), Some(("image/jpeg".into(), b"jpeg-bytes".to_vec())));
        assert_eq!(cache.get("https://i.ytimg.com/b.jpg"), None);
        assert_eq!(cache.used_bytes(), 10);
        let name = fs::read_dir(cache.dir()).unwrap().flatten().next().unwrap().file_name();
        assert_eq!(name.to_string_lossy().len(), 64 + 4, "sha256 hex + .jpg");
        let _ = fs::remove_dir_all(cache.dir());
    }

    #[test]
    fn skips_unknown_types_and_disabled_budget() {
        let cache = temp_cache("skip", 8);
        cache.put("https://x/a.svg", "image/svg+xml", b"<svg/>");
        assert_eq!(cache.get("https://x/a.svg"), None);
        cache.set_budget_mb(0);
        cache.put("https://x/b.png", "image/png", b"png");
        assert_eq!(cache.get("https://x/b.png"), None);
        let _ = fs::remove_dir_all(cache.dir());
    }

    #[test]
    fn evicts_least_recently_used_past_budget() {
        let cache = temp_cache("evict", 1);
        let blob = vec![7u8; 200 * 1024];
        for i in 0..5 {
            cache.put(&format!("https://x/{i}.jpg"), "image/jpeg", &blob);
            // Distinct modified times so "oldest" is well defined.
            let path = cache.dir().join(format!("{}.jpg", key(&format!("https://x/{i}.jpg"))));
            let when = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000 + i);
            let _ = fs::File::options().write(true).open(path).and_then(|f| f.set_modified(when));
        }
        cache.put("https://x/new.jpg", "image/jpeg", &blob);
        assert!(cache.used_bytes() <= MIB);
        assert!(cache.get("https://x/0.jpg").is_none(), "oldest entry is evicted first");
        assert!(cache.get("https://x/new.jpg").is_some());
        let before = cache.used_bytes();
        assert_eq!(cache.clear(), before);
        assert_eq!(cache.used_bytes(), 0);
        let _ = fs::remove_dir_all(cache.dir());
    }
}
