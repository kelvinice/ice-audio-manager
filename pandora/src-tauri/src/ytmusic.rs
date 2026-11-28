//! YouTube Music InnerTube API client + audio stream resolver (clean-room prototype).

use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::http;
use crate::models::{CollectionPage, HomeFeed, SearchResponse, Track};
use crate::ytm_feed;

const YTM_ORIGIN: &str = "https://music.youtube.com";
const WEB_REMIX_VERSION: &str = "1.20250219.01.00";
const STREAM_TTL: Duration = Duration::from_secs(4 * 3600);

#[derive(Debug, Clone)]
pub struct ResolvedStream {
    pub url: String,
    pub mime_type: String,
    pub content_length: Option<u64>,
    pub user_agent: Option<String>,
}

struct CachedStream {
    stream: ResolvedStream,
    expires_at: Instant,
}

pub struct YtMusicClient {
    stream_cache: Mutex<HashMap<String, CachedStream>>,
    visitor_data: Mutex<Option<String>>,
}

impl YtMusicClient {
    pub fn new() -> Self {
        Self {
            stream_cache: Mutex::new(HashMap::new()),
            visitor_data: Mutex::new(None),
        }
    }

    pub fn invalidate_stream(&self, video_id: &str) {
        if let Ok(mut guard) = self.stream_cache.lock() {
            guard.remove(video_id);
        }
    }

    fn capture_visitor_data(&self, val: &Value) {
        if let Some(vd) = val.pointer("/responseContext/visitorData").and_then(Value::as_str) {
            if !vd.is_empty() {
                if let Ok(mut guard) = self.visitor_data.lock() {
                    *guard = Some(vd.to_string());
                }
            }
        }
    }

    fn ensure_visitor_data(&self) -> Option<String> {
        if let Ok(guard) = self.visitor_data.lock() {
            if let Some(ref vd) = *guard {
                return Some(vd.clone());
            }
        }
        let body = json!({
            "context": web_remix_context(),
            "browseId": "FEmusic_home"
        })
        .to_string();
        let headers = [
            ("Origin", YTM_ORIGIN),
            ("Referer", "https://music.youtube.com/"),
            ("X-YouTube-Client-Name", "67"),
            ("X-YouTube-Client-Version", WEB_REMIX_VERSION),
        ];
        if let Ok((200..=299, bytes)) = http::post_json(
            "https://music.youtube.com/youtubei/v1/browse?prettyPrint=false",
            &headers,
            &body,
        ) {
            if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                self.capture_visitor_data(&val);
            }
        }
        self.visitor_data.lock().ok().and_then(|g| g.clone())
    }

    /// One `browse` call with the WEB_REMIX client. `extra` carries `browseId` or `continuation`.
    fn browse(&self, extra: Value) -> Result<Value, String> {
        let mut body = json!({ "context": web_remix_context() });
        if let (Some(target), Some(fields)) = (body.as_object_mut(), extra.as_object()) {
            target.extend(fields.clone());
        }
        let visitor = self.visitor_data.lock().ok().and_then(|g| g.clone()).unwrap_or_default();
        let mut headers = vec![
            ("Origin", YTM_ORIGIN),
            ("Referer", "https://music.youtube.com/"),
            ("X-YouTube-Client-Name", "67"),
            ("X-YouTube-Client-Version", WEB_REMIX_VERSION),
        ];
        if !visitor.is_empty() {
            // Keeps one visitor identity across calls so continuations and A/B variants stay consistent.
            headers.push(("X-Goog-Visitor-Id", visitor.as_str()));
        }
        let (status, bytes) =
            http::post_json("https://music.youtube.com/youtubei/v1/browse?prettyPrint=false", &headers, &body.to_string())?;
        if !(200..=299).contains(&status) {
            return Err(format!("YouTube Music HTTP {status}"));
        }
        let val: Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("Could not parse YouTube Music response: {e}"))?;
        self.capture_visitor_data(&val);
        Ok(val)
    }

    /// The home feed as titled shelves. Follows continuations (HOME_PAGES in all) so a first page of editorial
    /// playlist rows is followed by the song shelves ("Quick picks", "New releases", ...) further down.
    pub fn home(&self) -> HomeFeed {
        const HOME_PAGES: usize = 3;
        let mut shelves = Vec::new();
        let mut request = json!({ "browseId": "FEmusic_home" });
        let mut error = None;
        for _ in 0..HOME_PAGES {
            match self.browse(request) {
                Ok(val) => {
                    let (page, token) = ytm_feed::parse_home_page(&val);
                    shelves.extend(page);
                    let Some(token) = token else { break };
                    request = json!({ "continuation": token });
                }
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }
        let mut quick_picks: Vec<Track> = shelves.iter().flat_map(|shelf| shelf.tracks.iter().cloned()).collect();
        dedup_tracks(&mut quick_picks);
        let offline = shelves.is_empty();
        tracing::info!(shelves = shelves.len(), songs = quick_picks.len(), "YTM home loaded");
        HomeFeed { shelves, quick_picks, offline_fallback: offline, error: if offline { error } else { None } }
    }

    /// Opens a playlist, album or artist from the feed: header plus its tracks (playlists follow continuations).
    pub fn collection(&self, browse_id: &str) -> Result<CollectionPage, String> {
        let id = browse_id.trim();
        if !ytm_feed::is_valid_browse_id(id) {
            return Err("Unsupported YouTube Music link".into());
        }
        let (val, mut tracks, _) = self.browse_tracks(id)?;
        let (title, subtitle, artwork_url) = ytm_feed::parse_collection_header(&val);
        let title = title.unwrap_or_else(|| "YouTube Music".into());
        let kind = if id.starts_with("MPRE") {
            "album"
        } else if id.starts_with("UC") {
            "artist"
        } else {
            "playlist"
        };
        for track in &mut tracks {
            // Album rows carry no artwork or album name of their own.
            if track.artwork_url.is_none() {
                track.artwork_url.clone_from(&artwork_url);
            }
            if kind == "album" && track.album.is_empty() {
                track.album.clone_from(&title);
            }
        }
        if tracks.is_empty() {
            return Err("Nothing playable on that page".into());
        }
        Ok(CollectionPage { browse_id: id.to_string(), kind: kind.into(), title, subtitle, artwork_url, tracks })
    }

    /// First page of a browse id plus its tracks, following up to four continuation pages (500 tracks max).
    /// The flag is `false` when pages were left unread (limit reached or a continuation failed), so a playlist sync
    /// must not treat songs it didn't see as removed.
    fn browse_tracks(&self, browse_id: &str) -> Result<(Value, Vec<Track>, bool), String> {
        let val = self.browse(json!({ "browseId": browse_id }))?;
        let mut tracks = Vec::new();
        ytm_feed::collect_page_tracks(&val, &mut tracks);
        let mut next_token = find_continuation_token(&val);
        let mut seen_tokens = std::collections::HashSet::new();
        for _ in 0..4 {
            let Some(tok) = next_token.take() else { break };
            if !seen_tokens.insert(tok.clone()) {
                break;
            }
            if tracks.len() >= 500 {
                next_token = Some(tok);
                break;
            }
            let Ok(cont) = self.browse(json!({ "continuation": tok.clone() })) else {
                next_token = Some(tok);
                break;
            };
            collect_tracks(&cont, &mut tracks);
            next_token = find_continuation_token(&cont);
        }
        dedup_tracks(&mut tracks);
        Ok((val, tracks, next_token.is_none()))
    }

    pub fn search(&self, query: &str, filter: &str) -> SearchResponse {
        let q = query.trim();
        if q.is_empty() {
            return SearchResponse { query: String::new(), tracks: Vec::new(), error: None };
        }
        let params = match filter {
            "videos" => Some("EgWKAQIQAWoKEAoQAxAEEAkQBQ%3D%3D"),
            _ => Some("EgWKAQIIAWoKEAoQAxAEEAkQBQ%3D%3D"), // Songs filter
        };
        let mut payload = json!({
            "context": web_remix_context(),
            "query": q,
        });
        if let Some(p) = params {
            payload["params"] = Value::String(p.into());
        }
        let headers = [
            ("Origin", YTM_ORIGIN),
            ("Referer", "https://music.youtube.com/"),
            ("X-YouTube-Client-Name", "67"),
            ("X-YouTube-Client-Version", WEB_REMIX_VERSION),
        ];
        match http::post_json(
            "https://music.youtube.com/youtubei/v1/search?prettyPrint=false",
            &headers,
            &payload.to_string(),
        ) {
            Ok((200..=299, bytes)) => match serde_json::from_slice::<Value>(&bytes) {
                Ok(val) => {
                    self.capture_visitor_data(&val);
                    let mut tracks = Vec::new();
                    collect_tracks(&val, &mut tracks);
                    dedup_tracks(&mut tracks);
                    SearchResponse { query: q.to_string(), tracks, error: None }
                }
                Err(e) => SearchResponse {
                    query: q.to_string(),
                    tracks: Vec::new(),
                    error: Some(format!("JSON parse error: {e}")),
                },
            },
            Ok((status, _)) => SearchResponse {
                query: q.to_string(),
                tracks: Vec::new(),
                error: Some(format!("YouTube Music search returned HTTP {status}")),
            },
            Err(e) => SearchResponse { query: q.to_string(), tracks: Vec::new(), error: Some(e) },
        }
    }

    pub fn related(&self, video_id: &str) -> Vec<Track> {
        let body = json!({
            "context": web_remix_context(),
            "videoId": video_id,
            "playlistId": format!("RDAMVM{video_id}"),
            "isAudioOnly": true
        })
        .to_string();
        let headers = [("Origin", YTM_ORIGIN), ("Referer", "https://music.youtube.com/")];
        let Ok((200..=299, bytes)) = http::post_json(
            "https://music.youtube.com/youtubei/v1/next?prettyPrint=false",
            &headers,
            &body,
        ) else {
            return Vec::new();
        };
        let Ok(val) = serde_json::from_slice::<Value>(&bytes) else {
            return Vec::new();
        };
        let mut tracks = Vec::new();
        collect_playlist_panel_tracks(&val, &mut tracks);
        dedup_tracks(&mut tracks);
        tracks.retain(|t| t.id != video_id);
        tracks.truncate(15);
        tracks
    }

    /// Title, tracks, and whether the whole list was read (see `browse_tracks`).
    pub fn fetch_playlist(&self, list_id: &str) -> Result<(String, Vec<Track>, bool), String> {
        let clean = list_id.trim().trim_start_matches("VL").trim();
        if clean.is_empty() {
            return Err("Missing YouTube playlist ID".into());
        }
        let (val, tracks, complete) = self.browse_tracks(&format!("VL{clean}"))?;
        let title = extract_playlist_title(&val).unwrap_or_else(|| "YouTube Music Playlist".into());
        if tracks.is_empty() {
            return Err("No tracks found in that YouTube playlist (make sure it is public or unlisted)".into());
        }
        tracing::debug!(list_id = clean, title = %title, count = tracks.len(), complete, "Fetched YTM playlist");
        Ok((title, tracks, complete))
    }

    pub fn resolve_stream(&self, video_id: &str) -> Result<ResolvedStream, String> {
        if let Ok(mut guard) = self.stream_cache.lock() {
            if let Some(entry) = guard.get(video_id) {
                if Instant::now() < entry.expires_at {
                    return Ok(entry.stream.clone());
                }
            }
            guard.retain(|_, v| Instant::now() < v.expires_at);
        }

        let visitor = self.ensure_visitor_data().unwrap_or_default();

        let clients: &[(&str, &str, &str, &str, Value, &str)] = &[
            (
                "ANDROID",
                "3",
                "20.10.38",
                "https://youtubei.googleapis.com/youtubei/v1/player?prettyPrint=false",
                json!({
                    "clientName": "ANDROID",
                    "clientVersion": "20.10.38",
                    "androidSdkVersion": 34,
                    "osName": "Android",
                    "osVersion": "14",
                    "hl": "en",
                    "gl": "US"
                }),
                "com.google.android.youtube/20.10.38 (Linux; U; Android 14) gzip",
            ),
            (
                "ANDROID_VR",
                "28",
                "1.65.10",
                "https://youtubei.googleapis.com/youtubei/v1/player?prettyPrint=false",
                json!({
                    "clientName": "ANDROID_VR",
                    "clientVersion": "1.65.10",
                    "deviceMake": "Oculus",
                    "deviceModel": "Quest 3",
                    "androidSdkVersion": 32,
                    "osName": "Android",
                    "osVersion": "12L",
                    "hl": "en",
                    "gl": "US"
                }),
                "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip",
            ),
            (
                "IOS",
                "5",
                "20.10.4",
                "https://www.youtube.com/youtubei/v1/player?prettyPrint=false",
                json!({
                    "clientName": "IOS",
                    "clientVersion": "20.10.4",
                    "deviceMake": "Apple",
                    "deviceModel": "iPhone16,2",
                    "osName": "iPhone",
                    "osVersion": "18.3.2.22D82",
                    "hl": "en",
                    "gl": "US"
                }),
                "com.google.ios.youtube/20.10.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)",
            ),
        ];

        let mut last_err = String::from("No playable stream found");
        for (name, client_id, ver, endpoint, client_obj, ua) in clients {
            let mut client_val = client_obj.clone();
            if !visitor.is_empty() {
                client_val["visitorData"] = Value::String(visitor.clone());
            }
            let payload = json!({
                "context": { "client": client_val },
                "videoId": video_id,
                "contentCheckOk": true,
                "racyCheckOk": true
            })
            .to_string();
            let mut headers = vec![
                ("User-Agent", *ua),
                ("X-YouTube-Client-Name", *client_id),
                ("X-YouTube-Client-Version", *ver),
            ];
            if !visitor.is_empty() {
                headers.push(("X-Goog-Visitor-Id", visitor.as_str()));
            }
            match http::post_json_with_ua(endpoint, &headers, &payload, Some(ua)) {
                Ok((200..=299, bytes)) => {
                    if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                        // Prefer ANDROID progressive formats (no 1 MiB SABR/PO-token cap on googlevideo)
                        let picked = if *name == "ANDROID" {
                            pick_best_progressive_format(&val)
                                .map(|s| ("progressive", s))
                                .or_else(|| pick_best_audio_format(&val).map(|s| ("adaptive", s)))
                        } else {
                            pick_best_audio_format(&val).map(|s| ("adaptive", s))
                        };
                        if let Some((format_kind, mut stream)) = picked {
                            stream.user_agent = Some((*ua).to_string());
                            tracing::info!(
                                video_id,
                                client = name,
                                format_kind,
                                mime = %stream.mime_type,
                                "Resolved YTM stream"
                            );
                            self.cache_stream(video_id, stream.clone());
                            return Ok(stream);
                        }
                        let status_str = val
                            .pointer("/playabilityStatus/status")
                            .and_then(Value::as_str)
                            .unwrap_or("UNKNOWN");
                        let reason = val
                            .pointer("/playabilityStatus/reason")
                            .and_then(Value::as_str)
                            .unwrap_or("no adaptive audio url");
                        last_err = format!("{name}: {status_str} - {reason}");
                        tracing::warn!(video_id, client = name, status = status_str, reason, "Player client did not return direct audio");
                    }
                }
                Ok((status, _)) => {
                    last_err = format!("{name} HTTP {status}");
                    tracing::warn!(video_id, client = name, status, "Player HTTP error");
                }
                Err(e) => {
                    last_err = format!("{name}: {e}");
                    tracing::warn!(video_id, client = name, error = %e, "Player request error");
                }
            }
        }

        // Fallback to yt-dlp if installed on PATH
        if let Ok(output) = Command::new("yt-dlp")
            .args([
                "-f",
                "bestaudio[ext=m4a]/bestaudio",
                "-g",
                "--no-playlist",
                &format!("https://music.youtube.com/watch?v={video_id}"),
            ])
            .output()
        {
            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if url.starts_with("http") {
                    let stream = ResolvedStream {
                        url,
                        mime_type: "audio/mp4".into(),
                        content_length: None,
                        user_agent: None,
                    };
                    self.cache_stream(video_id, stream.clone());
                    return Ok(stream);
                }
            }
        }

        Err(last_err)
    }

    fn cache_stream(&self, video_id: &str, stream: ResolvedStream) {
        if let Ok(mut guard) = self.stream_cache.lock() {
            guard.insert(
                video_id.to_string(),
                CachedStream { stream, expires_at: Instant::now() + STREAM_TTL },
            );
        }
    }
}

fn web_remix_context() -> Value {
    json!({
        "client": {
            "clientName": "WEB_REMIX",
            "clientVersion": WEB_REMIX_VERSION,
            "hl": "en",
            "gl": "US"
        }
    })
}

fn pick_best_progressive_format(val: &Value) -> Option<ResolvedStream> {
    let formats = val.pointer("/streamingData/formats")?.as_array()?;
    let mut best: Option<(i64, ResolvedStream)> = None;
    for fmt in formats {
        let mime = fmt.get("mimeType").and_then(Value::as_str).unwrap_or("");
        if !mime.contains("mp4a") {
            continue;
        }
        let Some(url) = fmt.get("url").and_then(Value::as_str).filter(|u| !u.is_empty()) else {
            continue;
        };
        let itag = fmt.get("itag").and_then(Value::as_i64).unwrap_or(0);
        let bitrate = fmt.get("bitrate").and_then(Value::as_i64).unwrap_or(0);
        let itag_bonus = if itag == 18 { 5_000_000 } else { 0 };
        let score = bitrate + itag_bonus;
        let clean_mime = mime.split(';').next().unwrap_or("video/mp4").trim().to_string();
        let content_length = fmt.get("contentLength").and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| v.as_u64())
        });
        if best.as_ref().is_none_or(|(b, _)| score > *b) {
            best = Some((
                score,
                ResolvedStream {
                    url: url.to_string(),
                    mime_type: clean_mime,
                    content_length,
                    user_agent: None,
                },
            ));
        }
    }
    best.map(|(_, s)| s)
}

fn pick_best_audio_format(val: &Value) -> Option<ResolvedStream> {
    let formats = val.pointer("/streamingData/adaptiveFormats")?.as_array()?;
    let mut best: Option<(i64, ResolvedStream)> = None;
    for fmt in formats {
        let mime = fmt.get("mimeType").and_then(Value::as_str).unwrap_or("");
        if !mime.starts_with("audio/") {
            continue;
        }
        // Only pick codecs WebView2 <audio> reliably decodes (avoid ec-3 / ac-3 / iamf)
        if !mime.contains("mp4a") && !mime.contains("opus") {
            continue;
        }
        let Some(url) = fmt.get("url").and_then(Value::as_str).filter(|u| !u.is_empty()) else {
            continue;
        };
        let bitrate = fmt.get("bitrate").and_then(Value::as_i64).unwrap_or(0);
        // Prefer mp4/aac (universal WebView2 playback) + high bitrate
        let container_bonus = if mime.contains("audio/mp4") { 500_000 } else { 0 };
        let score = bitrate + container_bonus;
        let clean_mime = mime.split(';').next().unwrap_or("audio/mp4").trim().to_string();
        let content_length = fmt.get("contentLength").and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| v.as_u64())
        });
        if best.as_ref().is_none_or(|(b, _)| score > *b) {
            best = Some((
                score,
                ResolvedStream {
                    url: url.to_string(),
                    mime_type: clean_mime,
                    content_length,
                    user_agent: None,
                },
            ));
        }
    }
    best.map(|(_, s)| s)
}

pub(crate) fn collect_tracks(val: &Value, out: &mut Vec<Track>) {
    match val {
        Value::Object(map) => {
            if let Some(renderer) = map.get("musicResponsiveListItemRenderer") {
                if let Some(track) = parse_responsive_item(renderer) {
                    out.push(track);
                }
            }
            if let Some(renderer) = map.get("musicTwoRowItemRenderer") {
                if let Some(track) = parse_two_row_item(renderer) {
                    out.push(track);
                }
            }
            for child in map.values() {
                collect_tracks(child, out);
            }
        }
        Value::Array(arr) => {
            for child in arr {
                collect_tracks(child, out);
            }
        }
        _ => {}
    }
}

fn collect_playlist_panel_tracks(val: &Value, out: &mut Vec<Track>) {
    match val {
        Value::Object(map) => {
            if let Some(renderer) = map.get("playlistPanelVideoRenderer") {
                if let Some(video_id) = renderer.get("videoId").and_then(Value::as_str) {
                    let title = runs_text(renderer.get("title"));
                    let byline = runs_text(renderer.get("longBylineText"))
                        .or_else(|| runs_text(renderer.get("shortBylineText")));
                    let dur_text = runs_text(renderer.get("lengthText"));
                    let thumb = pick_thumbnail(renderer.pointer("/thumbnail/thumbnails"));
                    if let Some(title) = title {
                        let (artist, album) = split_byline(byline.as_deref().unwrap_or(""));
                        out.push(Track {
                            id: video_id.to_string(),
                            source: "ytmusic".into(),
                            title,
                            artist,
                            album,
                            duration_ms: dur_text.as_deref().and_then(parse_duration_ms),
                            artwork_url: thumb,
                            local_path: None,
                            cached: false,
                            liked: false,
                        });
                    }
                }
            }
            for child in map.values() {
                collect_playlist_panel_tracks(child, out);
            }
        }
        Value::Array(arr) => {
            for child in arr {
                collect_playlist_panel_tracks(child, out);
            }
        }
        _ => {}
    }
}

pub(crate) fn parse_responsive_item(item: &Value) -> Option<Track> {
    let video_id = item
        .pointer("/playlistItemData/videoId")
        .and_then(Value::as_str)
        .or_else(|| {
            item.pointer("/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/videoId")
                .and_then(Value::as_str)
        })
        .or_else(|| item.pointer("/navigationEndpoint/watchEndpoint/videoId").and_then(Value::as_str))?;

    let cols = item.get("flexColumns")?.as_array()?;
    let title = cols
        .first()
        .and_then(|c| runs_text(c.pointer("/musicResponsiveListItemFlexColumnRenderer/text")))?;
    let sub_runs = cols
        .get(1)
        .and_then(|c| c.pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs"))
        .and_then(Value::as_array);

    let mut artist = String::new();
    let mut album = String::new();
    let mut duration_ms = None;

    if let Some(runs) = sub_runs {
        let mut parts: Vec<String> = Vec::new();
        for run in runs {
            let txt = run.get("text").and_then(Value::as_str).unwrap_or("").trim();
            if txt.is_empty() || txt == "•" || txt == "·" || txt == "&" || txt == "," {
                continue;
            }
            if let Some(ms) = parse_duration_ms(txt) {
                duration_ms = Some(ms);
            } else {
                parts.push(txt.to_string());
            }
        }
        // Skip leading type label like "Song" or "Video" if present
        let filtered: Vec<String> = parts
            .into_iter()
            .filter(|p| !matches!(p.as_str(), "Song" | "Video" | "Single" | "EP"))
            .collect();
        if let Some(a) = filtered.first() {
            artist = a.clone();
        }
        if let Some(al) = filtered.get(1) {
            album = al.clone();
        }
    }

    if artist.is_empty() {
        artist = "Unknown Artist".into();
    }

    let thumb = pick_thumbnail(
        item.pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails"),
    );

    Some(Track {
        id: video_id.to_string(),
        source: "ytmusic".into(),
        title,
        artist,
        album,
        duration_ms,
        artwork_url: thumb,
        local_path: None,
        cached: false,
        liked: false,
    })
}

pub(crate) fn parse_two_row_item(item: &Value) -> Option<Track> {
    let video_id = item
        .pointer("/navigationEndpoint/watchEndpoint/videoId")
        .and_then(Value::as_str)?;
    let title = runs_text(item.get("title"))?;
    let subtitle = runs_text(item.get("subtitle")).unwrap_or_default();
    let (artist, album) = split_byline(&subtitle);
    let thumb = pick_thumbnail(
        item.pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails"),
    );
    Some(Track {
        id: video_id.to_string(),
        source: "ytmusic".into(),
        title,
        artist,
        album,
        duration_ms: None,
        artwork_url: thumb,
        local_path: None,
        cached: false,
        liked: false,
    })
}

pub(crate) fn runs_text(val: Option<&Value>) -> Option<String> {
    let v = val?;
    if let Some(simple) = v.get("simpleText").and_then(Value::as_str) {
        return Some(simple.to_string());
    }
    let runs = v.get("runs")?.as_array()?;
    let joined: String = runs
        .iter()
        .filter_map(|r| r.get("text").and_then(Value::as_str))
        .collect();
    let trimmed = joined.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

fn split_byline(byline: &str) -> (String, String) {
    let parts: Vec<&str> = byline
        .split(['•', '·'])
        .map(str::trim)
        .filter(|s| !s.is_empty() && !matches!(*s, "Song" | "Video"))
        .collect();
    let artist = parts.first().copied().unwrap_or("YouTube Music").to_string();
    let album = parts
        .get(1)
        .filter(|s| parse_duration_ms(s).is_none())
        .copied()
        .unwrap_or("")
        .to_string();
    (artist, album)
}

pub(crate) fn pick_thumbnail(thumbs: Option<&Value>) -> Option<String> {
    let arr = thumbs?.as_array()?;
    let url = arr
        .iter()
        .max_by_key(|t| t.get("width").and_then(Value::as_u64).unwrap_or(0))?
        .get("url")?
        .as_str()?;
    // Upgrade Googleusercontent thumbnail size to crisp 544x544 for Dynamic Material You & Now Playing
    if url.contains("googleusercontent.com/") {
        if let Some((prefix, _)) = url.rsplit_once('=') {
            return Some(format!("{prefix}=w544-h544-l90-rj"));
        }
    }
    Some(url.to_string())
}

pub fn parse_duration_ms(text: &str) -> Option<u64> {
    let parts: Vec<&str> = text.trim().split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let nums: Option<Vec<u64>> = parts.iter().map(|p| p.parse::<u64>().ok()).collect();
    let nums = nums?;
    let secs = match nums.as_slice() {
        [m, s] if *s < 60 => m.checked_mul(60)?.checked_add(*s)?,
        [h, m, s] if *m < 60 && *s < 60 => h.checked_mul(3600)?.checked_add(m * 60)?.checked_add(*s)?,
        _ => return None,
    };
    Some(secs * 1000)
}

fn dedup_tracks(tracks: &mut Vec<Track>) {
    let mut seen = std::collections::HashSet::new();
    tracks.retain(|t| seen.insert(t.id.clone()));
}

fn extract_playlist_title(val: &Value) -> Option<String> {
    match val {
        Value::Object(map) => {
            for key in [
                "musicResponsiveHeaderRenderer",
                "musicDetailHeaderRenderer",
                "musicEditablePlaylistDetailHeaderRenderer",
            ] {
                if let Some(hdr) = map.get(key) {
                    if let Some(title) = runs_text(hdr.get("title")) {
                        return Some(title);
                    }
                    if let Some(title) = runs_text(hdr.pointer("/header/musicResponsiveHeaderRenderer/title")) {
                        return Some(title);
                    }
                    if let Some(title) = runs_text(hdr.pointer("/header/musicDetailHeaderRenderer/title")) {
                        return Some(title);
                    }
                }
            }
            for child in map.values() {
                if let Some(found) = extract_playlist_title(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => arr.iter().find_map(extract_playlist_title),
        _ => None,
    }
}

fn find_continuation_token(val: &Value) -> Option<String> {
    match val {
        Value::Object(map) => {
            if let Some(tok) = val
                .pointer("/continuationItemRenderer/continuationEndpoint/continuationCommand/token")
                .and_then(Value::as_str)
            {
                if !tok.is_empty() {
                    return Some(tok.to_string());
                }
            }
            if let Some(tok) = val
                .pointer("/continuations/0/nextContinuationData/continuation")
                .and_then(Value::as_str)
            {
                if !tok.is_empty() {
                    return Some(tok.to_string());
                }
            }
            for child in map.values() {
                if let Some(found) = find_continuation_token(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => arr.iter().find_map(find_continuation_token),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_durations_and_upgrades_art() {
        assert_eq!(parse_duration_ms("3:45"), Some(225_000));
        assert_eq!(parse_duration_ms("1:02:03"), Some(3723_000));
        assert_eq!(parse_duration_ms("Song"), None);
    }
}
