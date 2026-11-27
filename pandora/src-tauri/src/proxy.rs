//! Local HTTP/1.1 streaming & artwork proxy on `127.0.0.1:0`.
//!
//! Routes:
//! - `GET /stream/yt/<id>`   — offline cache hit or live YouTube Music audio stream with Range pass-through
//! - `GET /stream/local?path=…` — local audio file streaming with full byte-range seeking
//! - `GET /art?url=…`        — CORS-safe album art proxy for `<canvas>` Material You color extraction

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

use crate::artcache::ArtCache;
use crate::http;
use crate::library::LibraryStore;
use crate::ytmusic::YtMusicClient;

fn is_allowed_art_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h == "music.youtube.com"
        || h.ends_with(".googleusercontent.com")
        || h == "googleusercontent.com"
        || h.ends_with(".ytimg.com")
        || h == "ytimg.com"
        || h.ends_with(".ggpht.com")
        || h == "ggpht.com"
        || h.ends_with(".mzstatic.com")
        || h == "mzstatic.com"
        || h.ends_with(".shazam.com")
        || h == "shazam.com"
}

struct ArtEntry {
    content_type: String,
    bytes: Vec<u8>,
}

pub struct ProxyState {
    pub ytm: Arc<YtMusicClient>,
    pub library: Arc<LibraryStore>,
    /// Small in-memory front for the hottest images (the player bar, the visible list).
    art_cache: Mutex<HashMap<String, ArtEntry>>,
    art_disk: Arc<ArtCache>,
}

pub async fn start(
    ytm: Arc<YtMusicClient>,
    library: Arc<LibraryStore>,
    art_disk: Arc<ArtCache>,
) -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("proxy bind failed: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("proxy local_addr failed: {e}"))?
        .port();

    let state = Arc::new(ProxyState {
        ytm,
        library,
        art_cache: Mutex::new(HashMap::new()),
        art_disk,
    });

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let st = Arc::clone(&state);
                    tokio::spawn(async move {
                        let _ = handle_conn(stream, st).await;
                    });
                }
                Err(e) => {
                    tracing::warn!(error = %e, "proxy accept error");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
        }
    });

    Ok(port)
}

async fn handle_conn(stream: TcpStream, state: Arc<ProxyState>) -> Result<(), String> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let mut request_line = String::new();
    let n = reader
        .read_line(&mut request_line)
        .await
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Ok(());
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");

    let mut range_header: Option<String> = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).await.map_err(|e| e.to_string())?;
        if read == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, val)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("range") {
                range_header = Some(val.trim().to_string());
            }
        }
    }

    if method.eq_ignore_ascii_case("OPTIONS") {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, HEAD, OPTIONS\r\nAccess-Control-Allow-Headers: Range\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = write_half.write_all(resp.as_bytes()).await;
        return Ok(());
    }

    if !method.eq_ignore_ascii_case("GET") && !method.eq_ignore_ascii_case("HEAD") {
        return send_text(&mut write_half, 405, "Method Not Allowed", "Method Not Allowed").await;
    }
    let head_only = method.eq_ignore_ascii_case("HEAD");

    let full = format!("http://127.0.0.1{target}");
    let parsed = match Url::parse(&full) {
        Ok(u) => u,
        Err(_) => return send_text(&mut write_half, 400, "Bad Request", "Bad Request").await,
    };
    let path = parsed.path();

    if let Some(video_id) = path.strip_prefix("/stream/yt/") {
        if let Some(cached_path) = state.library.cache_file_for(video_id) {
            tracing::info!(video_id, "Serving YTM track from offline cache");
            return serve_file(&mut write_half, &cached_path, range_header.as_deref(), head_only, Some("audio/mp4")).await;
        }
        for attempt in 0..2 {
            let ytm = Arc::clone(&state.ytm);
            let vid = video_id.to_string();
            let resolved = tokio::task::spawn_blocking(move || ytm.resolve_stream(&vid))
                .await
                .map_err(|e| e.to_string())?;
            match resolved {
                Ok(stream_info) => {
                    let res = proxy_remote_stream(
                        &mut write_half,
                        video_id,
                        &stream_info.url,
                        &stream_info.mime_type,
                        stream_info.content_length,
                        stream_info.user_agent.as_deref(),
                        range_header.as_deref(),
                        head_only,
                        attempt == 0,
                    )
                    .await;
                    if let Err(ref e) = res {
                        if e == "UPSTREAM_403_RETRY" && attempt == 0 {
                            tracing::warn!(video_id, "Upstream 403 on cached URL, refreshing stream URL");
                            state.ytm.invalidate_stream(video_id);
                            continue;
                        }
                    }
                    return res;
                }
                Err(err) => {
                    tracing::warn!(video_id, error = %err, "Failed to resolve YTM stream");
                    return send_text(&mut write_half, 502, "Bad Gateway", &err).await;
                }
            }
        }
        return send_text(&mut write_half, 502, "Bad Gateway", "Stream retry exhausted").await;
    }

    if path == "/stream/local" {
        let local_path = parsed
            .query_pairs()
            .find(|(k, _)| k == "path")
            .map(|(_, v)| v.to_string());
        let Some(p) = local_path else {
            return send_text(&mut write_half, 400, "Bad Request", "Missing path").await;
        };
        let file_path = Path::new(&p);
        if !file_path.is_file() {
            return send_text(&mut write_half, 404, "Not Found", "File not found").await;
        }
        return serve_file(&mut write_half, file_path, range_header.as_deref(), head_only, None).await;
    }

    if path == "/art" {
        let art_url = parsed
            .query_pairs()
            .find(|(k, _)| k == "url")
            .map(|(_, v)| v.to_string());
        let Some(raw) = art_url else {
            return send_text(&mut write_half, 400, "Bad Request", "Missing url").await;
        };
        return serve_art(&mut write_half, &state, &raw).await;
    }

    send_text(&mut write_half, 404, "Not Found", "Not Found").await
}

async fn serve_art(
    write_half: &mut tokio::net::tcp::OwnedWriteHalf,
    state: &Arc<ProxyState>,
    raw_url: &str,
) -> Result<(), String> {
    let parsed = match Url::parse(raw_url) {
        Ok(u) if u.scheme() == "https" => u,
        _ => return send_text(write_half, 400, "Bad Request", "Invalid art URL").await,
    };
    let host = parsed.host_str().unwrap_or("");
    if !is_allowed_art_host(host) {
        tracing::warn!(host, "Rejected artwork host");
        return send_text(write_half, 403, "Forbidden", "Host not allowed").await;
    }

    let cached_hit = state
        .art_cache
        .lock()
        .ok()
        .and_then(|guard| guard.get(raw_url).map(|hit| (hit.content_type.clone(), hit.bytes.clone())));
    if let Some((ct, bytes)) = cached_hit {
        return send_bytes(write_half, 200, "OK", &ct, &bytes).await;
    }

    let disk = Arc::clone(&state.art_disk);
    let key = raw_url.to_string();
    if let Ok(Some((ct, bytes))) = tokio::task::spawn_blocking(move || disk.get(&key)).await {
        remember_art(state, raw_url, &ct, &bytes);
        return send_bytes(write_half, 200, "OK", &ct, &bytes).await;
    }

    let url_owned = raw_url.to_string();
    let fetched = tokio::task::spawn_blocking(move || {
        let stream = http::open_request("GET", &url_owned, &[], None, None, 8_000)?;
        if !(200..=299).contains(&stream.meta.status) {
            return Err(format!("art HTTP {}", stream.meta.status));
        }
        let ct = stream
            .meta
            .content_type
            .clone()
            .unwrap_or_else(|| "image/jpeg".into());
        let bytes = stream.read_all(4 * 1024 * 1024)?;
        Ok((ct, bytes))
    })
    .await
    .map_err(|e| e.to_string())?;

    match fetched {
        Ok((ct, bytes)) => {
            remember_art(state, raw_url, &ct, &bytes);
            let disk = Arc::clone(&state.art_disk);
            let (key, disk_ct, disk_bytes) = (raw_url.to_string(), ct.clone(), bytes.clone());
            // Fire and forget: the response does not wait for the disk write.
            drop(tokio::task::spawn_blocking(move || disk.put(&key, &disk_ct, &disk_bytes)));
            send_bytes(write_half, 200, "OK", &ct, &bytes).await
        }
        Err(e) => {
            tracing::warn!(error = %e, url = raw_url, "Artwork proxy failed");
            send_text(write_half, 502, "Bad Gateway", &e).await
        }
    }
}

fn remember_art(state: &ProxyState, raw_url: &str, content_type: &str, bytes: &[u8]) {
    if let Ok(mut guard) = state.art_cache.lock() {
        if guard.len() > 128 {
            guard.clear();
        }
        guard.insert(raw_url.to_string(), ArtEntry { content_type: content_type.to_string(), bytes: bytes.to_vec() });
    }
}

const CHUNK_SIZE: u64 = 524_288;

pub(crate) fn parse_total_from_content_range(cr: &str) -> Option<u64> {
    let (_, total_str) = cr.trim().rsplit_once('/')?;
    total_str.trim().parse::<u64>().ok().filter(|&n| n > 0)
}

async fn proxy_remote_stream(
    write_half: &mut tokio::net::tcp::OwnedWriteHalf,
    video_id: &str,
    remote_url: &str,
    fallback_mime: &str,
    known_total: Option<u64>,
    user_agent: Option<&str>,
    range: Option<&str>,
    head_only: bool,
    retry_on_403: bool,
) -> Result<(), String> {
    let url = remote_url.to_string();
    let ua = user_agent.map(str::to_string);
    let mime = fallback_mime.to_string();

    // Determine total stream size so every upstream request uses bounded <= 512 KB ranges.
    let total_len = match known_total {
        Some(t) if t > 0 => t,
        _ => {
            let probe_url = url.clone();
            let probe_ua = ua.clone();
            let probe = tokio::task::spawn_blocking(move || {
                let mut hdrs = vec![("Range", "bytes=0-1")];
                if let Some(ref u) = probe_ua {
                    hdrs.push(("User-Agent", u.as_str()));
                }
                let s = http::open_request("GET", &probe_url, &hdrs, None, probe_ua.as_deref(), 12_000)?;
                Ok::<_, String>((s.meta.status, s.meta.content_range))
            })
            .await
            .map_err(|e| e.to_string())?;
            match probe {
                Ok((403, _)) if retry_on_403 => return Err("UPSTREAM_403_RETRY".into()),
                Ok((status, cr)) => {
                    if !(200..=299).contains(&status) {
                        tracing::warn!(video_id, status, "Upstream probe failed");
                        return send_text(write_half, 502, "Bad Gateway", &format!("Upstream HTTP {status}")).await;
                    }
                    let Some(total) = cr.as_deref().and_then(parse_total_from_content_range) else {
                        return send_text(write_half, 502, "Bad Gateway", "Missing Content-Range total").await;
                    };
                    total
                }
                Err(e) => {
                    tracing::warn!(video_id, error = %e, "Upstream probe error");
                    return send_text(write_half, 502, "Bad Gateway", &e).await;
                }
            }
        }
    };

    let (req_start, req_end, is_partial) = match range.and_then(|r| parse_range(r, total_len)) {
        Some((s, e)) => (s, e, true),
        None => (0, total_len.saturating_sub(1), false),
    };
    let resp_len = req_end.saturating_sub(req_start) + 1;

    // Fetch first chunk before committing response headers so stale-URL 403s can retry cleanly.
    let first_end = req_start.saturating_add(CHUNK_SIZE - 1).min(req_end);
    let first_url = url.clone();
    let first_ua = ua.clone();
    let first_mime = mime.clone();
    let first_res = tokio::task::spawn_blocking(move || {
        let range_val = format!("bytes={req_start}-{first_end}");
        let mut hdrs = vec![("Range", range_val.as_str())];
        if let Some(ref u) = first_ua {
            hdrs.push(("User-Agent", u.as_str()));
        }
        let stream = http::open_request("GET", &first_url, &hdrs, None, first_ua.as_deref(), 20_000)?;
        let status = stream.meta.status;
        let ct = stream.meta.content_type.clone().unwrap_or(first_mime);
        if status == 403 {
            return Ok::<_, String>((status, ct, Vec::new()));
        }
        if !(200..=299).contains(&status) {
            return Err(format!("Upstream chunk HTTP {status}"));
        }
        let bytes = if head_only {
            Vec::new()
        } else {
            stream.read_all((CHUNK_SIZE as usize) + 65_536)?
        };
        Ok((status, ct, bytes))
    })
    .await
    .map_err(|e| e.to_string())?;

    let (status, ct, first_bytes) = match first_res {
        Ok((403, _, _)) if retry_on_403 => return Err("UPSTREAM_403_RETRY".into()),
        Ok((403, _, _)) => {
            tracing::warn!(video_id, "Upstream 403 after retry");
            return send_text(write_half, 502, "Bad Gateway", "Upstream HTTP 403").await;
        }
        Ok(ok) => ok,
        Err(e) => {
            tracing::warn!(video_id, error = %e, "Upstream audio chunk failed");
            return send_text(write_half, 502, "Bad Gateway", &e).await;
        }
    };

    tracing::info!(
        video_id,
        upstream_status = status,
        content_type = %ct,
        req_start,
        req_end,
        total_len,
        "Streaming YTM audio (chunked)"
    );

    let mut header = if is_partial {
        format!("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {req_start}-{req_end}/{total_len}\r\n")
    } else {
        "HTTP/1.1 200 OK\r\n".to_string()
    };
    header.push_str(&format!(
        "Content-Type: {ct}\r\nContent-Length: {resp_len}\r\nAccept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\nConnection: close\r\n\r\n"
    ));
    write_half.write_all(header.as_bytes()).await.map_err(|e| e.to_string())?;

    if head_only {
        return Ok(());
    }
    if write_half.write_all(&first_bytes).await.is_err() {
        return Ok(());
    }

    let mut pos = first_end.saturating_add(1);
    while pos <= req_end {
        let chunk_end = pos.saturating_add(CHUNK_SIZE - 1).min(req_end);
        let c_url = url.clone();
        let c_ua = ua.clone();
        let chunk_res = tokio::task::spawn_blocking(move || {
            let range_val = format!("bytes={pos}-{chunk_end}");
            let mut hdrs = vec![("Range", range_val.as_str())];
            if let Some(ref u) = c_ua {
                hdrs.push(("User-Agent", u.as_str()));
            }
            let stream = http::open_request("GET", &c_url, &hdrs, None, c_ua.as_deref(), 20_000)?;
            if !(200..=299).contains(&stream.meta.status) {
                return Err(format!("Upstream chunk HTTP {}", stream.meta.status));
            }
            stream.read_all((CHUNK_SIZE as usize) + 65_536)
        })
        .await
        .map_err(|e| e.to_string())?;

        match chunk_res {
            Ok(bytes) if !bytes.is_empty() => {
                if write_half.write_all(&bytes).await.is_err() {
                    break;
                }
                pos = chunk_end.saturating_add(1);
            }
            Ok(_) => break,
            Err(e) => {
                tracing::warn!(video_id, pos, error = %e, "Mid-stream chunk failed");
                break;
            }
        }
    }
    Ok(())
}

async fn serve_file(
    write_half: &mut tokio::net::tcp::OwnedWriteHalf,
    path: &Path,
    range: Option<&str>,
    head_only: bool,
    mime_override: Option<&str>,
) -> Result<(), String> {
    let meta = tokio::fs::metadata(path).await.map_err(|e| e.to_string())?;
    let total = meta.len();
    let mime = mime_override.unwrap_or_else(|| mime_for_path(path));
    let (start, end, is_partial) = match range.and_then(|r| parse_range(r, total)) {
        Some((s, e)) => (s, e, true),
        None => (0, total.saturating_sub(1), false),
    };
    let len = if total == 0 { 0 } else { end.saturating_sub(start) + 1 };

    let mut header = if is_partial {
        format!(
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{total}\r\n"
        )
    } else {
        "HTTP/1.1 200 OK\r\n".to_string()
    };
    header.push_str(&format!(
        "Content-Type: {mime}\r\nContent-Length: {len}\r\nAccept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n"
    ));
    write_half.write_all(header.as_bytes()).await.map_err(|e| e.to_string())?;
    if head_only || len == 0 {
        return Ok(());
    }

    let path_owned = path.to_path_buf();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
    tokio::task::spawn_blocking(move || {
        let Ok(mut f) = std::fs::File::open(path_owned) else { return };
        if f.seek(SeekFrom::Start(start)).is_err() {
            return;
        }
        let mut remaining = len;
        let mut buf = vec![0u8; 64 * 1024];
        while remaining > 0 {
            let to_read = (remaining as usize).min(buf.len());
            match f.read(&mut buf[..to_read]) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    remaining -= n as u64;
                    if tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    while let Some(chunk) = rx.recv().await {
        if write_half.write_all(&chunk).await.is_err() {
            break;
        }
    }
    Ok(())
}

fn parse_range(header: &str, total: u64) -> Option<(u64, u64)> {
    let spec = header.strip_prefix("bytes=")?.split(',').next()?.trim();
    let (s, e) = spec.split_once('-')?;
    if total == 0 {
        return None;
    }
    if s.is_empty() {
        let suffix = e.parse::<u64>().ok()?;
        let start = total.saturating_sub(suffix);
        return Some((start, total - 1));
    }
    let start = s.parse::<u64>().ok()?;
    let end = if e.is_empty() { total - 1 } else { e.parse::<u64>().ok()?.min(total - 1) };
    (start <= end).then_some((start, end))
}

fn mime_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("wav") => "audio/wav",
        Some("ogg") | Some("opus") => "audio/ogg",
        Some("m4a") | Some("aac") => "audio/mp4",
        Some("webm") | Some("weba") => "audio/webm",
        _ => "application/octet-stream",
    }
}

async fn send_text(
    write_half: &mut tokio::net::tcp::OwnedWriteHalf,
    status: u16,
    reason: &str,
    body: &str,
) -> Result<(), String> {
    send_bytes(write_half, status, reason, "text/plain; charset=utf-8", body.as_bytes()).await
}

async fn send_bytes(
    write_half: &mut tokio::net::tcp::OwnedWriteHalf,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    write_half.write_all(head.as_bytes()).await.map_err(|e| e.to_string())?;
    write_half.write_all(body).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_allowed_art_host, parse_total_from_content_range};

    #[test]
    fn allows_youtube_and_googleusercontent_art_hosts() {
        assert!(is_allowed_art_host("yt3.googleusercontent.com"));
        assert!(is_allowed_art_host("lh3.googleusercontent.com"));
        assert!(is_allowed_art_host("i.ytimg.com"));
        assert!(is_allowed_art_host("yt3.ggpht.com"));
        assert!(is_allowed_art_host("music.youtube.com"));
        assert!(!is_allowed_art_host("evil.example.com"));
    }

    #[test]
    fn parses_total_from_content_range() {
        assert_eq!(parse_total_from_content_range("bytes 0-1/3199756"), Some(3_199_756));
        assert_eq!(parse_total_from_content_range("bytes 0-524287/3199756"), Some(3_199_756));
        assert_eq!(parse_total_from_content_range("bytes 0-1/*"), None);
    }

    #[test]
    #[ignore = "live network smoke test"]
    fn live_proxy_streams_silver_lining() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let ytm = std::sync::Arc::new(crate::ytmusic::YtMusicClient::new());
            let lib = std::sync::Arc::new(crate::library::LibraryStore::load());
            let art = std::sync::Arc::new(crate::artcache::ArtCache::open(lib.cache_dir().join("..").join("artwork"), 64));
            let port = super::start(ytm, lib, art).await.unwrap();

            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut sock = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            sock.write_all(
                b"GET /stream/yt/f2PdKjdPMmU HTTP/1.1\r\nHost: 127.0.0.1\r\nRange: bytes=0-\r\n\r\n",
            )
            .await
            .unwrap();
            let mut buf = vec![0u8; 4096];
            let mut total = 0usize;
            while total < 1024 {
                let n = sock.read(&mut buf[total..]).await.unwrap();
                if n == 0 {
                    break;
                }
                total += n;
            }
            let head_end = buf[..total]
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
                .expect("HTTP headers");
            let headers = String::from_utf8_lossy(&buf[..head_end]);
            assert!(headers.starts_with("HTTP/1.1 206 Partial Content"), "{headers}");
            assert!(
                headers.contains("Content-Type: audio/mp4") || headers.contains("Content-Type: video/mp4"),
                "{headers}"
            );
            let body = &buf[head_end + 4..total];
            assert!(body.windows(4).any(|w| w == b"ftyp"), "expected MP4 ftyp header");
            drop(sock);

            // Verify mid-song range well past 1 MiB also streams cleanly with zero 403 cutoff.
            let mut sock2 = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            sock2
                .write_all(
                    b"GET /stream/yt/f2PdKjdPMmU HTTP/1.1\r\nHost: 127.0.0.1\r\nRange: bytes=2000000-2065535\r\n\r\n",
                )
                .await
                .unwrap();
            let mut mid_buf = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut sock2, &mut mid_buf).await.unwrap();
            let mid_end = mid_buf
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
                .expect("mid HTTP headers");
            let mid_headers = String::from_utf8_lossy(&mid_buf[..mid_end]);
            assert!(mid_headers.starts_with("HTTP/1.1 206 Partial Content"), "{mid_headers}");
            assert_eq!(mid_buf.len() - (mid_end + 4), 65_536);
        });
    }
}
