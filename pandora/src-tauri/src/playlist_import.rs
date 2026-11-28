//! Import public Spotify or YouTube Music playlists (plus CSV / text tracklists)
//! into Pandora playlists without requiring user logins or API keys.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde_json::Value;
use tauri::{AppHandle, Emitter};
use url::Url;

use crate::http;
use crate::library::LibraryStore;
use crate::models::{PlaylistImportProgress, PlaylistImportResult, PlaylistLink, Track};
use crate::playlist_link::{self, LinkItems, SeedContext};
use crate::ytmusic::YtMusicClient;

const IMPORT_EVENT: &str = "playlist://import/v1";
pub const MAX_IMPORT_TRACKS: usize = 250;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedTrack {
    pub title: String,
    pub artist: String,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSource {
    YouTubePlaylist { list_id: String },
    Spotify { kind: &'static str, id: String },
    /// A `spotify.link` share link; `playlist_link::resolve_short_link` turns it into `Spotify`.
    SpotifyShort { url: String },
    TextList { entries: Vec<SeedTrack> },
}

pub fn parse_import_input(raw: &str) -> Result<ImportSource, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Paste a Spotify/YouTube Music playlist URL, Exportify CSV, or track list".into());
    }

    if !trimmed.contains('\n') && !trimmed.contains('\r') {
        if let Some(rest) = trimmed.strip_prefix("spotify:") {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() >= 2 && is_spotify_id(parts[1]) {
                if parts[0] == "playlist" {
                    return Ok(ImportSource::Spotify {
                        kind: "playlist",
                        id: parts[1].to_string(),
                    });
                }
                if parts[0] == "album" {
                    return Ok(ImportSource::Spotify {
                        kind: "album",
                        id: parts[1].to_string(),
                    });
                }
            }
        }

        if let Ok(url) = Url::parse(trimmed) {
            let host = url.host_str().unwrap_or("").to_ascii_lowercase();
            if host == "youtu.be" || host.ends_with(".youtu.be") || host == "youtube.com" || host.ends_with(".youtube.com") {
                if let Some((_, list_id)) = url.query_pairs().find(|(k, _)| k == "list") {
                    let clean = list_id.trim();
                    if !clean.is_empty() && clean.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                        return Ok(ImportSource::YouTubePlaylist { list_id: clean.to_string() });
                    }
                }
                return Err("That YouTube link does not include a playlist (`list=...`) parameter".into());
            }
            if host == "spotify.link" || host.ends_with(".spotify.link") || host == "spotify.app.link" {
                return Ok(ImportSource::SpotifyShort { url: url.to_string() });
            }
            if host == "spotify.com" || host.ends_with(".spotify.com") {
                let segs: Vec<&str> = url
                    .path_segments()
                    .map(|s| s.filter(|p| !p.is_empty()).collect())
                    .unwrap_or_default();
                for pair in segs.windows(2) {
                    let kind = match pair[0] {
                        "playlist" => Some("playlist"),
                        "album" => Some("album"),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        let id = pair[1].split('?').next().unwrap_or("");
                        if is_spotify_id(id) {
                            return Ok(ImportSource::Spotify { kind, id: id.to_string() });
                        }
                    }
                }
                return Err("Expected a Spotify playlist or album URL (open.spotify.com/playlist/...)".into());
            }
        }

        if !trimmed.contains(char::is_whitespace) && is_youtube_playlist_id(trimmed) {
            return Ok(ImportSource::YouTubePlaylist {
                list_id: trimmed.trim_start_matches("VL").to_string(),
            });
        }
    }

    let entries = parse_text_tracklist(trimmed);
    if entries.is_empty() {
        return Err("Could not recognize a playlist URL or any `Artist - Title` / CSV tracks".into());
    }
    Ok(ImportSource::TextList { entries })
}

pub fn is_spotify_id(s: &str) -> bool {
    (16..=32).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_youtube_playlist_id(s: &str) -> bool {
    let bare = s.trim_start_matches("VL");
    bare.len() >= 12
        && (bare.starts_with("PL")
            || bare.starts_with("OLAK5uy_")
            || bare.starts_with("RDCLAK5uy_")
            || bare.starts_with("UU")
            || bare.starts_with("FL"))
        && bare.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn parse_text_tracklist(raw: &str) -> Vec<SeedTrack> {
    let lines: Vec<&str> = raw.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Vec::new();
    }

    // Exportify / generic CSV detection
    let header_lower = lines[0].to_ascii_lowercase();
    if header_lower.contains(',')
        && (header_lower.contains("track") || header_lower.contains("title"))
        && header_lower.contains("artist")
    {
        let cols = parse_csv_line(lines[0]);
        let title_col = cols.iter().position(|c| {
            let l = c.trim().to_ascii_lowercase();
            l == "track name" || l == "track" || l == "title" || l == "song"
        });
        let artist_col = cols.iter().position(|c| {
            let l = c.trim().to_ascii_lowercase();
            l == "artist name(s)" || l == "artist" || l == "artists" || l == "artist name"
        });
        let dur_col = cols.iter().position(|c| {
            let l = c.trim().to_ascii_lowercase();
            l.contains("duration") && l.contains("ms")
        });

        if let (Some(ti), Some(ai)) = (title_col, artist_col) {
            let mut out = Vec::new();
            for line in lines.iter().skip(1) {
                let fields = parse_csv_line(line);
                let title = fields.get(ti).map(|s| s.trim()).unwrap_or("");
                if title.is_empty() {
                    continue;
                }
                let artist = fields
                    .get(ai)
                    .map(|s| s.replace(';', ", ").trim().to_string())
                    .unwrap_or_default();
                let duration_ms = dur_col
                    .and_then(|di| fields.get(di))
                    .and_then(|v| v.trim().parse::<u64>().ok());
                out.push(SeedTrack {
                    title: title.to_string(),
                    artist,
                    duration_ms,
                });
            }
            if !out.is_empty() {
                return out;
            }
        }
    }

    let mut out = Vec::new();
    let mut expect_extinf_url = false;
    for line in lines {
        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            if let Some((_, info)) = rest.split_once(',') {
                let (artist, title) = split_artist_title(info);
                if !title.is_empty() {
                    out.push(SeedTrack { title, artist, duration_ms: None });
                }
            }
            expect_extinf_url = true;
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if expect_extinf_url {
            expect_extinf_url = false;
            if line.contains('\\') || line.contains('/') || line.starts_with("http") {
                continue;
            }
        }

        let cleaned = strip_leading_number(line);
        let (artist, title) = split_artist_title(cleaned);
        if !title.is_empty() {
            out.push(SeedTrack { title, artist, duration_ms: None });
        }
    }
    out
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    fields.push(current);
    fields
}

fn strip_leading_number(s: &str) -> &str {
    let trimmed = s.trim();
    let digit_len = trimmed.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digit_len > 0 && digit_len <= 4 {
        let rest = &trimmed[digit_len..];
        if let Some(after) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return after.trim();
        }
    }
    trimmed
}

fn split_artist_title(s: &str) -> (String, String) {
    for sep in [" - ", " – ", " — "] {
        if let Some((left, right)) = s.split_once(sep) {
            let artist = left.trim();
            let title = right.trim();
            if !artist.is_empty() && !title.is_empty() {
                return (artist.to_string(), title.to_string());
            }
        }
    }
    (String::new(), s.trim().to_string())
}

pub fn extract_spotify_next_data(html: &str) -> Result<(String, Vec<SeedTrack>), String> {
    let marker = "<script id=\"__NEXT_DATA__\" type=\"application/json\">";
    let start_idx = html
        .find(marker)
        .ok_or_else(|| "Could not find playlist data in Spotify page".to_string())?
        + marker.len();
    let end_idx = html[start_idx..]
        .find("</script>")
        .ok_or_else(|| "Malformed Spotify playlist payload".to_string())?
        + start_idx;

    let json_str = &html[start_idx..end_idx];
    let val: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse Spotify playlist JSON: {e}"))?;

    let entity = val
        .pointer("/props/pageProps/state/data/entity")
        .ok_or_else(|| "Spotify playlist is unavailable or private".to_string())?;

    let name = entity
        .get("name")
        .or_else(|| entity.get("title"))
        .and_then(Value::as_str)
        .map(|s| s.replace('\u{00A0}', " ").trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Spotify Playlist".into());

    let fallback_artist = entity
        .get("subtitle")
        .and_then(Value::as_str)
        .map(|s| s.replace('\u{00A0}', " ").trim().to_string())
        .unwrap_or_default();

    let arr = entity
        .get("trackList")
        .and_then(Value::as_array)
        .ok_or_else(|| "Spotify playlist contains no tracks".to_string())?;

    let mut seeds = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(title) = item.get("title").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if title.is_empty() {
            continue;
        }
        let artist = item
            .get("subtitle")
            .and_then(Value::as_str)
            .map(|s| s.replace('\u{00A0}', " ").trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback_artist.clone());
        let duration_ms = item.get("duration").and_then(Value::as_u64);
        seeds.push(SeedTrack {
            title: title.to_string(),
            artist,
            duration_ms,
        });
    }

    if seeds.is_empty() {
        return Err("Spotify playlist has no importable tracks".into());
    }
    Ok((name, seeds))
}

pub fn fetch_spotify_embed(kind: &str, id: &str) -> Result<(String, Vec<SeedTrack>), String> {
    let url = format!("https://open.spotify.com/embed/{kind}/{id}");
    let stream = http::open_request("GET", &url, &[], None, Some(http::DEFAULT_UA), 15_000)?;
    if stream.meta.status == 404 {
        return Err("Spotify playlist not found or is private. Make sure the playlist is Public.".into());
    }
    if !(200..=299).contains(&stream.meta.status) {
        return Err(format!("Spotify embed returned HTTP {}", stream.meta.status));
    }
    let bytes = stream.read_all(4 * 1024 * 1024)?;
    let html = String::from_utf8_lossy(&bytes);
    extract_spotify_next_data(&html)
}

pub fn emit_progress(app: &AppHandle, progress: PlaylistImportProgress) {
    let _ = app.emit(IMPORT_EVENT, progress);
}

fn primary_artist(artist: &str) -> &str {
    artist
        .split([',', ';'])
        .next()
        .map(str::trim)
        .unwrap_or(artist)
}

fn norm(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c.to_ascii_lowercase() } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn pick_best_match<'a>(seed: &SeedTrack, candidates: &'a [Track]) -> Option<&'a Track> {
    let seed_title = norm(&seed.title);
    let seed_artist = norm(primary_artist(&seed.artist));
    let mut best: Option<(i32, &'a Track)> = None;

    for cand in candidates {
        let mut score: i32 = 0;
        let cand_title = norm(&cand.title);
        let cand_artist = norm(&cand.artist);

        if !seed_title.is_empty() {
            if cand_title == seed_title {
                score += 60;
            } else if cand_title.contains(&seed_title) || seed_title.contains(&cand_title) {
                score += 40;
            }
        }

        if !seed_artist.is_empty() {
            if cand_artist == seed_artist {
                score += 35;
            } else if cand_artist.contains(&seed_artist) || seed_artist.contains(&cand_artist) {
                score += 25;
            }
        }

        if let (Some(want_ms), Some(got_ms)) = (seed.duration_ms, cand.duration_ms) {
            let diff = want_ms.abs_diff(got_ms);
            if diff <= 10_000 {
                score += 15;
            } else if diff <= 25_000 {
                score += 5;
            } else if diff > 60_000 {
                score -= 15;
            }
        }

        for tag in ["karaoke", "8d audio", "slowed", "reverb", "nightcore", "sped up", "cover"] {
            if cand_title.contains(tag) && !seed_title.contains(tag) {
                score -= 25;
            }
        }

        if best.as_ref().is_none_or(|(b, _)| score > *b) {
            best = Some((score, cand));
        }
    }

    best.map(|(_, t)| t).or_else(|| candidates.first())
}

/// Searches YouTube Music for each seed (4 at a time); one slot per seed, `None` when nothing matched.
pub fn match_seeds_on_ytm(
    seeds: &[SeedTrack],
    source_label: &str,
    ytm: &YtMusicClient,
    cancel: &AtomicBool,
    app: &AppHandle,
) -> Result<Vec<Option<Track>>, String> {
    let total = seeds.len().min(MAX_IMPORT_TRACKS);
    let slice = &seeds[..total];
    let slots: Vec<Mutex<Option<Option<Track>>>> = (0..total).map(|_| Mutex::new(None)).collect();
    let next_idx = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let matched_count = AtomicUsize::new(0);
    let workers = total.clamp(1, 4);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    if cancel.load(Ordering::Acquire) {
                        break;
                    }
                    let idx = next_idx.fetch_add(1, Ordering::Relaxed);
                    if idx >= total {
                        break;
                    }
                    let seed = &slice[idx];
                    let query = if seed.artist.is_empty() {
                        seed.title.clone()
                    } else {
                        format!("{} {}", seed.title, primary_artist(&seed.artist))
                    };

                    let picked = ytm
                        .search(&query, "songs")
                        .tracks
                        .into_iter()
                        .collect::<Vec<_>>();
                    let best = pick_best_match(seed, &picked).cloned();
                    if best.is_some() {
                        matched_count.fetch_add(1, Ordering::Relaxed);
                    }
                    if let Ok(mut guard) = slots[idx].lock() {
                        *guard = Some(best);
                    }

                    let done_now = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    let matched_now = matched_count.load(Ordering::Relaxed);
                    let label = if seed.artist.is_empty() {
                        seed.title.clone()
                    } else {
                        format!("{} — {}", seed.title, seed.artist)
                    };
                    emit_progress(
                        app,
                        PlaylistImportProgress {
                            stage: "matching".into(),
                            source: source_label.into(),
                            completed: done_now,
                            total,
                            matched: matched_now,
                            current_track: Some(label),
                        },
                    );
                }
            });
        }
    });

    if cancel.load(Ordering::Acquire) {
        emit_progress(
            app,
            PlaylistImportProgress {
                stage: "cancelled".into(),
                source: source_label.into(),
                completed: completed.load(Ordering::Relaxed),
                total,
                matched: matched_count.load(Ordering::Relaxed),
                current_track: None,
            },
        );
        return Err("Playlist import cancelled".into());
    }

    Ok(slots.into_iter().map(|slot| slot.into_inner().ok().flatten().flatten()).collect())
}

/// One stage event for the import / sync progress bar.
pub fn progress(stage: &str, source: &str, completed: usize, total: usize, matched: usize) -> PlaylistImportProgress {
    PlaylistImportProgress { stage: stage.into(), source: source.into(), completed, total, matched, current_track: None }
}

/// What an import found, before the playlist is created.
struct Gathered {
    name: String,
    total: usize,
    tracks: Vec<Track>,
    unmatched: Vec<String>,
    note: Option<String>,
    link: Option<PlaylistLink>,
}

fn gather(parsed: ImportSource, ytm: &YtMusicClient, cancel: &AtomicBool, app: &AppHandle) -> Result<Gathered, String> {
    if let ImportSource::TextList { entries } = parsed {
        let total = entries.len().min(MAX_IMPORT_TRACKS);
        let note =
            (entries.len() > MAX_IMPORT_TRACKS).then(|| format!("Imported first {MAX_IMPORT_TRACKS} tracks from the list."));
        let slots = match_seeds_on_ytm(&entries, "text", ytm, cancel, app)?;
        let unmatched = playlist_link::unmatched_labels(&entries, &slots);
        let tracks = slots.into_iter().flatten().collect();
        return Ok(Gathered { name: "Imported Playlist".into(), total, tracks, unmatched, note, link: None });
    }
    let source = playlist_link::resolve_short_link(parsed)?;
    let (service, url) = playlist_link::canonical_link(&source)
        .ok_or_else(|| "Paste a Spotify or YouTube Music playlist link".to_string())?;
    let fetched = playlist_link::fetch_link(&source, ytm)?;
    if cancel.load(Ordering::Acquire) {
        return Err("Playlist import cancelled".into());
    }
    let (seeds, slots) = match fetched.items {
        LinkItems::Tracks(tracks) => (None, tracks.into_iter().map(Some).collect::<Vec<_>>()),
        LinkItems::Seeds(seeds) => {
            let (known, index) = (Default::default(), Default::default());
            let ctx = SeedContext { known: &known, index: &index, current: &[] };
            let slots = playlist_link::resolve_seeds(&seeds, &ctx, service, ytm, cancel, app)?;
            (Some(seeds), slots)
        }
    };
    let unmatched = seeds.as_deref().map(|s| playlist_link::unmatched_labels(s, &slots)).unwrap_or_default();
    let link = playlist_link::link_record(service, url, seeds.as_deref(), &slots);
    Ok(Gathered {
        name: fetched.name,
        total: fetched.total,
        tracks: slots.into_iter().flatten().collect(),
        unmatched,
        note: fetched.note,
        link: Some(link),
    })
}

pub fn import_playlist(
    raw_input: &str,
    name_override: Option<&str>,
    ytm: &YtMusicClient,
    library: &LibraryStore,
    cancel: &AtomicBool,
    app: &AppHandle,
) -> Result<PlaylistImportResult, String> {
    cancel.store(false, Ordering::Release);
    let parsed = parse_import_input(raw_input)?;
    let source_label = source_label(&parsed);
    emit_progress(app, progress("fetching", source_label, 0, 0, 0));

    let gathered = match gather(parsed, ytm, cancel, app) {
        Ok(found) => found,
        Err(e) => {
            let stage = if cancel.load(Ordering::Acquire) { "cancelled" } else { "error" };
            emit_progress(app, progress(stage, source_label, 0, 0, 0));
            return Err(e);
        }
    };
    let Gathered { name: detected_name, total: total_tracks, tracks, unmatched: unmatched_tracks, note, link } = gathered;

    if tracks.is_empty() {
        emit_progress(app, progress("error", source_label, total_tracks, total_tracks, 0));
        return Err("Could not match any playable YouTube Music tracks".into());
    }

    let final_name = name_override.map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&detected_name).to_string();
    let imported_tracks = tracks.len();
    let (next_lib, playlist) = library.create_playlist_with_tracks(&final_name, tracks, link)?;

    emit_progress(app, progress("done", source_label, total_tracks, total_tracks, imported_tracks));

    Ok(PlaylistImportResult {
        library: next_lib,
        playlist_id: playlist.id,
        playlist_name: playlist.name,
        source: source_label.into(),
        total_tracks,
        imported_tracks,
        unmatched_tracks,
        note,
    })
}

/// `"ytmusic" | "spotify" | "text"` for progress events and results.
pub fn source_label(source: &ImportSource) -> &'static str {
    match source {
        ImportSource::YouTubePlaylist { .. } => "ytmusic",
        ImportSource::Spotify { .. } | ImportSource::SpotifyShort { .. } => "spotify",
        ImportSource::TextList { .. } => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_track(id: &str, title: &str, artist: &str, duration_ms: Option<u64>) -> Track {
        Track {
            id: id.into(),
            source: "ytmusic".into(),
            title: title.into(),
            artist: artist.into(),
            album: String::new(),
            duration_ms,
            artwork_url: None,
            local_path: None,
            cached: false,
            liked: false,
        }
    }

    #[test]
    fn parses_spotify_and_ytm_urls_and_ids() {
        assert_eq!(
            parse_import_input("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=abc"),
            Ok(ImportSource::Spotify {
                kind: "playlist",
                id: "37i9dQZF1DXcBWIGoYBM5M".into()
            })
        );
        assert_eq!(
            parse_import_input("https://open.spotify.com/intl-id/album/1A2B3C4D5E6F7G8H9I0J1K"),
            Ok(ImportSource::Spotify {
                kind: "album",
                id: "1A2B3C4D5E6F7G8H9I0J1K".into()
            })
        );
        assert_eq!(
            parse_import_input("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"),
            Ok(ImportSource::Spotify {
                kind: "playlist",
                id: "37i9dQZF1DXcBWIGoYBM5M".into()
            })
        );
        assert_eq!(
            parse_import_input("https://music.youtube.com/playlist?list=PL4fGSI1pDJn6puJdseH2Rt9sMvt9E2M4i"),
            Ok(ImportSource::YouTubePlaylist {
                list_id: "PL4fGSI1pDJn6puJdseH2Rt9sMvt9E2M4i".into()
            })
        );
        assert_eq!(
            parse_import_input("https://spotify.link/AbCdEf123"),
            Ok(ImportSource::SpotifyShort { url: "https://spotify.link/AbCdEf123".into() })
        );
        assert!(parse_import_input("https://music.youtube.com/playlist?list=PL%22x").is_err());
        assert_eq!(
            parse_import_input("PL4fGSI1pDJn6puJdseH2Rt9sMvt9E2M4i"),
            Ok(ImportSource::YouTubePlaylist {
                list_id: "PL4fGSI1pDJn6puJdseH2Rt9sMvt9E2M4i".into()
            })
        );
    }

    #[test]
    fn parses_exportify_csv_and_text_and_m3u() {
        let csv = "Track URI,Track Name,Artist Name(s),Duration (ms)\n\
                   spotify:track:1,\"I Love You, I'm Sorry\",Gracie Abrams,200000\n\
                   spotify:track:2,Espresso,Sabrina Carpenter;Jack Antonoff,175450\n";
        let got_csv = parse_text_tracklist(csv);
        assert_eq!(got_csv.len(), 2);
        assert_eq!(got_csv[0].title, "I Love You, I'm Sorry");
        assert_eq!(got_csv[0].artist, "Gracie Abrams");
        assert_eq!(got_csv[0].duration_ms, Some(200_000));
        assert_eq!(got_csv[1].artist, "Sabrina Carpenter, Jack Antonoff");

        let plain = "1. Laufey - Silver Lining\n# comment\nSabrina Carpenter – Please Please Please\n";
        let got_plain = parse_text_tracklist(plain);
        assert_eq!(got_plain.len(), 2);
        assert_eq!(got_plain[0].artist, "Laufey");
        assert_eq!(got_plain[0].title, "Silver Lining");
        assert_eq!(got_plain[1].title, "Please Please Please");

        let m3u = "#EXTM3U\n#EXTINF:210,Laufey - From The Start\n/music/from_the_start.mp3\n";
        let got_m3u = parse_text_tracklist(m3u);
        assert_eq!(got_m3u.len(), 1);
        assert_eq!(got_m3u[0].artist, "Laufey");
        assert_eq!(got_m3u[0].title, "From The Start");
    }

    #[test]
    fn extracts_spotify_next_data_and_normalizes_nbsp() {
        let html = r#"<html><head></head><body><script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"state":{"data":{"entity":{"name":"My\u00A0Mix","trackList":[{"title":"Midnight Rain","subtitle":"Taylor\u00A0Swift, Lana","duration":174000}]}}}}}}</script></body></html>"#;
        let (name, tracks) = extract_spotify_next_data(html).unwrap();
        assert_eq!(name, "My Mix");
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Midnight Rain");
        assert_eq!(tracks[0].artist, "Taylor Swift, Lana");
        assert_eq!(tracks[0].duration_ms, Some(174_000));
    }

    #[test]
    fn pick_best_match_prefers_exact_and_skips_karaoke() {
        let seed = SeedTrack {
            title: "Espresso".into(),
            artist: "Sabrina Carpenter".into(),
            duration_ms: Some(175_000),
        };
        let candidates = vec![
            mk_track("bad", "Espresso Karaoke", "Karaoke Stars", Some(175_000)),
            mk_track("good", "Espresso", "Sabrina Carpenter", Some(176_000)),
        ];
        assert_eq!(pick_best_match(&seed, &candidates).unwrap().id, "good");
    }
}
