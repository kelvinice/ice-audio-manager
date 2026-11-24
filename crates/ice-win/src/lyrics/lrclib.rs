//! Option A: clean-room opt-in HTTPS client for LRCLIB (`https://lrclib.net`) via Windows WinHTTP.
//!
//! Zero extra HTTP/TLS crates: uses OS-native WinHTTP with TLS, system proxy support, strict host allowlist
//! (`lrclib.net` only), 4 s timeout, 256 KB body ceiling, and `Retry-After` backoff on 429/503.

use std::ffi::c_void;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use ice_core::lyrics::{
    LyricsDocument, LyricsProvider, LyricsSource, LyricsSyncKind, TrackQuery, normalize_query_title, parse_lrc,
};
use serde::Deserialize;
use windows::Win32::Networking::WinHttp::{
    INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE, WINHTTP_QUERY_RETRY_AFTER,
    WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryDataAvailable, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts,
};
use windows::core::{HSTRING, PCWSTR};

use super::cache::MAX_ENTRY_BYTES;

const LRCLIB_HOST: &str = "lrclib.net";
const USER_AGENT: &str = "IceAudioManager/0.0.1 (+https://github.com/kelvinice/ice-audio-manager)";
const RESOLVE_CONNECT_TIMEOUT_MS: i32 = 3_000;
const SEND_RECEIVE_TIMEOUT_MS: i32 = 4_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrcLibRecord {
    #[serde(default)]
    track_name: String,
    #[serde(default)]
    artist_name: String,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    plain_lyrics: Option<String>,
    #[serde(default)]
    synced_lyrics: Option<String>,
}

/// Clean-room LRCLIB provider (`https://lrclib.net` only).
#[derive(Default)]
pub struct LrcLibProvider {
    backoff_until: Mutex<Option<Instant>>,
}

impl LrcLibProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn is_backed_off(&self) -> bool {
        self.backoff_until.lock().ok().and_then(|g| *g).is_some_and(|until| Instant::now() < until)
    }

    fn note_backoff(&self, secs: u64) {
        if let Ok(mut g) = self.backoff_until.lock() {
            *g = Some(Instant::now() + Duration::from_secs(secs.clamp(5, 300)));
        }
    }

    /// Fetches lyrics from LRCLIB, checking `is_cancelled` before every request phase and response chunk so
    /// track skips or shutdown abort pending work promptly.
    ///
    /// # Errors
    /// Returns `"cancelled"` when `is_cancelled()` fires, or a human-readable network/rate-limit error.
    pub fn fetch_cancellable(
        &self,
        query: &TrackQuery,
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<LyricsDocument>, String> {
        if query.title.trim().is_empty() || query.artist.trim().is_empty() {
            return Ok(None);
        }
        if is_cancelled() {
            return Err("cancelled".into());
        }
        if self.is_backed_off() {
            return Err("lrclib rate-limit backoff active".into());
        }

        // 1. Exact lookup (`GET /api/get`).
        let mut get_params = vec![("track_name", query.title.clone()), ("artist_name", query.artist.clone())];
        if !query.album.trim().is_empty() {
            get_params.push(("album_name", query.album.clone()));
        }
        if let Some(dur_ms) = query.duration_ms {
            get_params.push(("duration", ((dur_ms + 500) / 1_000).to_string()));
        }
        let get_path = format!("/api/get?{}", encode_query(&get_params));
        match winhttp_get_lrclib(&get_path, is_cancelled)? {
            HttpOutcome::Ok(body) => {
                if let Ok(rec) = serde_json::from_slice::<LrcLibRecord>(&body)
                    && let Some(doc) = record_to_document(rec, query)
                {
                    return Ok(Some(doc));
                }
            }
            HttpOutcome::NotFound => {}
            HttpOutcome::RateLimited(retry_secs) => {
                self.note_backoff(retry_secs);
                return Err(format!("lrclib rate limited ({retry_secs}s)"));
            }
            HttpOutcome::HttpError(code) => return Err(format!("lrclib HTTP {code}")),
        }

        if is_cancelled() {
            return Err("cancelled".into());
        }

        // 2. Normalized search fallback (`GET /api/search`, ±2 s preferred, ±5 s max).
        let clean_title = normalize_query_title(&query.title);
        let search_params = vec![
            ("track_name", if clean_title.is_empty() { query.title.clone() } else { clean_title }),
            ("artist_name", query.artist.clone()),
        ];
        let search_path = format!("/api/search?{}", encode_query(&search_params));
        match winhttp_get_lrclib(&search_path, is_cancelled)? {
            HttpOutcome::Ok(body) => {
                let records = serde_json::from_slice::<Vec<LrcLibRecord>>(&body).unwrap_or_default();
                Ok(pick_best_search_match(records, query).and_then(|rec| record_to_document(rec, query)))
            }
            HttpOutcome::NotFound => Ok(None),
            HttpOutcome::RateLimited(retry_secs) => {
                self.note_backoff(retry_secs);
                Err(format!("lrclib rate limited ({retry_secs}s)"))
            }
            HttpOutcome::HttpError(code) => Err(format!("lrclib HTTP {code}")),
        }
    }
}

impl LyricsProvider for LrcLibProvider {
    fn fetch(&self, query: &TrackQuery) -> Result<Option<LyricsDocument>, String> {
        self.fetch_cancellable(query, &|| false)
    }
}

fn record_to_document(rec: LrcLibRecord, query: &TrackQuery) -> Option<LyricsDocument> {
    if rec.instrumental {
        return Some(LyricsDocument {
            track_key: query.track_key.clone(),
            source: LyricsSource::Lrclib,
            sync_kind: LyricsSyncKind::Plain,
            instrumental: true,
            lines: Vec::new(),
            plain_text: None,
        });
    }
    if let Some(synced) = rec.synced_lyrics.as_deref().filter(|s| !s.trim().is_empty()) {
        let (sync_kind, lines) = parse_lrc(synced, query.duration_ms);
        if !lines.is_empty() {
            return Some(LyricsDocument {
                track_key: query.track_key.clone(),
                source: LyricsSource::Lrclib,
                sync_kind,
                instrumental: false,
                lines,
                plain_text: rec.plain_lyrics,
            });
        }
    }
    let plain = rec.plain_lyrics.filter(|s| !s.trim().is_empty())?;
    Some(LyricsDocument {
        track_key: query.track_key.clone(),
        source: LyricsSource::Lrclib,
        sync_kind: LyricsSyncKind::Plain,
        instrumental: false,
        lines: Vec::new(),
        plain_text: Some(plain),
    })
}

/// Picks the best search candidate: prefers synced lyrics and duration within ±2 s (relaxing to ±5 s).
fn pick_best_search_match(records: Vec<LrcLibRecord>, query: &TrackQuery) -> Option<LrcLibRecord> {
    let target_secs = query.duration_ms.map(|ms| (ms as f64) / 1_000.0);
    let q_title = normalize_query_title(&query.title).to_lowercase();
    let q_artist = query.artist.trim().to_lowercase();

    let mut best: Option<(i32, LrcLibRecord)> = None;
    for rec in records {
        let has_synced = rec.synced_lyrics.as_deref().is_some_and(|s| !s.trim().is_empty());
        let has_plain = rec.plain_lyrics.as_deref().is_some_and(|s| !s.trim().is_empty());
        if !has_synced && !has_plain && !rec.instrumental {
            continue;
        }
        let dur_diff = target_secs.zip(rec.duration).map(|(a, b)| (a - b).abs());
        if let Some(diff) = dur_diff
            && diff > 5.5
        {
            continue;
        }
        let mut score = if has_synced { 100 } else { 20 };
        if let Some(diff) = dur_diff {
            if diff <= 2.0 {
                score += 40;
            } else {
                score += 10;
            }
        }
        if normalize_query_title(&rec.track_name).to_lowercase() == q_title {
            score += 20;
        }
        if rec.artist_name.trim().to_lowercase().contains(&q_artist) {
            score += 10;
        }
        if best.as_ref().is_none_or(|(b_score, _)| score > *b_score) {
            best = Some((score, rec));
        }
    }
    best.map(|(_, rec)| rec)
}

fn encode_query(params: &[(&str, String)]) -> String {
    params.iter().map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v))).collect::<Vec<_>>().join("&")
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            b' ' => out.push_str("%20"),
            b => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

enum HttpOutcome {
    Ok(Vec<u8>),
    NotFound,
    RateLimited(u64),
    HttpError(u32),
}

struct HInternet(*mut c_void);

impl Drop for HInternet {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: handle was returned by WinHttpOpen / WinHttpConnect / WinHttpOpenRequest and owned uniquely here.
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
}

fn winhttp_get_lrclib(path_and_query: &str, is_cancelled: &(dyn Fn() -> bool + Sync)) -> Result<HttpOutcome, String> {
    if is_cancelled() {
        return Err("cancelled".into());
    }
    let ua = HSTRING::from(USER_AGENT);
    // SAFETY: valid null-terminated UTF-16 user-agent string; handle wrapped immediately in RAII HInternet.
    let session = HInternet(unsafe {
        WinHttpOpen(PCWSTR(ua.as_ptr()), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, PCWSTR::null(), PCWSTR::null(), 0)
    });
    if session.0.is_null() {
        return Err("WinHttpOpen failed".into());
    }
    // SAFETY: session is a valid WinHTTP session handle.
    unsafe {
        let _ = WinHttpSetTimeouts(
            session.0,
            RESOLVE_CONNECT_TIMEOUT_MS,
            RESOLVE_CONNECT_TIMEOUT_MS,
            SEND_RECEIVE_TIMEOUT_MS,
            SEND_RECEIVE_TIMEOUT_MS,
        );
    }

    let host = HSTRING::from(LRCLIB_HOST);
    // SAFETY: session is valid and host is null-terminated UTF-16; returned handle wrapped in HInternet.
    let conn = HInternet(unsafe { WinHttpConnect(session.0, PCWSTR(host.as_ptr()), INTERNET_DEFAULT_HTTPS_PORT, 0) });
    if conn.0.is_null() {
        return Err("WinHttpConnect failed".into());
    }

    let verb = HSTRING::from("GET");
    let object = HSTRING::from(path_and_query);
    // SAFETY: conn is valid, verb and object are null-terminated UTF-16; returned handle wrapped in HInternet.
    let req = HInternet(unsafe {
        WinHttpOpenRequest(
            conn.0,
            PCWSTR(verb.as_ptr()),
            PCWSTR(object.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        )
    });
    if req.0.is_null() {
        return Err("WinHttpOpenRequest failed".into());
    }

    if is_cancelled() {
        return Err("cancelled".into());
    }
    // SAFETY: req is a valid request handle with no extra headers or request body.
    unsafe {
        WinHttpSendRequest(req.0, None, None, 0, 0, 0).map_err(|e| format!("WinHttpSendRequest: {e}"))?;
        WinHttpReceiveResponse(req.0, std::ptr::null_mut()).map_err(|e| format!("WinHttpReceiveResponse: {e}"))?;
    }
    if is_cancelled() {
        return Err("cancelled".into());
    }

    let status = query_header_u32(req.0, WINHTTP_QUERY_STATUS_CODE).unwrap_or(0);
    if status == 404 {
        return Ok(HttpOutcome::NotFound);
    }
    if status == 429 || status == 503 {
        let retry = u64::from(query_header_u32(req.0, WINHTTP_QUERY_RETRY_AFTER).unwrap_or(30));
        return Ok(HttpOutcome::RateLimited(retry));
    }
    if status != 200 {
        return Ok(HttpOutcome::HttpError(status));
    }

    let mut body = Vec::new();
    loop {
        if is_cancelled() {
            return Err("cancelled".into());
        }
        let mut avail = 0u32;
        // SAFETY: req is a valid response-ready handle and `avail` is a valid stack pointer.
        unsafe {
            WinHttpQueryDataAvailable(req.0, &mut avail).map_err(|e| format!("WinHttpQueryDataAvailable: {e}"))?;
        }
        if avail == 0 {
            break;
        }
        let remaining = (MAX_ENTRY_BYTES as usize).saturating_sub(body.len());
        if remaining == 0 {
            return Err("lrclib response exceeded 256 KB limit".into());
        }
        let to_read = (avail as usize).min(remaining).min(16 * 1024);
        let start = body.len();
        body.resize(start + to_read, 0);
        let mut read = 0u32;
        // SAFETY: slice `&mut body[start..]` has length `to_read` and `read` is a valid stack pointer.
        unsafe {
            WinHttpReadData(req.0, body[start..].as_mut_ptr().cast(), to_read as u32, &mut read)
                .map_err(|e| format!("WinHttpReadData: {e}"))?;
        }
        body.truncate(start + read as usize);
        if read == 0 {
            break;
        }
    }
    Ok(HttpOutcome::Ok(body))
}

fn query_header_u32(req: *mut c_void, level: u32) -> Option<u32> {
    let mut buf = [0u16; 32];
    let mut byte_len = (buf.len() * 2) as u32;
    // SAFETY: `buf` holds `byte_len` writable bytes for UTF-16 header text from WinHTTP.
    unsafe {
        WinHttpQueryHeaders(
            req,
            level,
            PCWSTR::null(),
            Some(buf.as_mut_ptr().cast()),
            &mut byte_len,
            std::ptr::null_mut(),
        )
        .ok()?;
    }
    let len = (byte_len as usize) / 2;
    String::from_utf16_lossy(&buf[..len]).trim().parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_utf8_safely() {
        assert_eq!(
            encode_query(&[("track_name", "アイドル & 星".into())]),
            "track_name=%E3%82%A2%E3%82%A4%E3%83%89%E3%83%AB%20%26%20%E6%98%9F"
        );
    }

    #[test]
    fn search_prefers_synced_match_within_two_seconds() {
        let query = TrackQuery {
            track_key: "k".into(),
            title: "Idol".into(),
            artist: "YOASOBI".into(),
            album: String::new(),
            duration_ms: Some(213_000),
        };
        let records = vec![
            LrcLibRecord {
                track_name: "Idol".into(),
                artist_name: "YOASOBI".into(),
                duration: Some(213.0),
                instrumental: false,
                plain_lyrics: Some("Plain".into()),
                synced_lyrics: None,
            },
            LrcLibRecord {
                track_name: "Idol".into(),
                artist_name: "YOASOBI".into(),
                duration: Some(214.2),
                instrumental: false,
                plain_lyrics: None,
                synced_lyrics: Some("[00:01.00]Synced".into()),
            },
            LrcLibRecord {
                track_name: "Idol (TV Size)".into(),
                artist_name: "YOASOBI".into(),
                duration: Some(90.0),
                instrumental: false,
                plain_lyrics: None,
                synced_lyrics: Some("[00:01.00]Wrong duration".into()),
            },
        ];
        let picked = pick_best_search_match(records, &query).unwrap();
        assert_eq!(picked.synced_lyrics.as_deref(), Some("[00:01.00]Synced"));
    }

    #[test]
    fn fetch_cancellable_aborts_immediately_when_pre_cancelled() {
        let provider = LrcLibProvider::new();
        let query = TrackQuery {
            track_key: "k".into(),
            title: "Idol".into(),
            artist: "YOASOBI".into(),
            album: String::new(),
            duration_ms: Some(213_000),
        };
        let err = provider.fetch_cancellable(&query, &|| true).unwrap_err();
        assert_eq!(err, "cancelled");
    }
}
