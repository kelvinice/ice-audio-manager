//! Spotify / YouTube Music playlist links shared by import and sync: canonical form, short-link resolving, fetching
//! the list, and matching Spotify songs to YouTube Music.
//!
//! Link lifetimes (researched 2026-11-28): plain playlist links (`open.spotify.com/playlist/<id>`,
//! `music.youtube.com/playlist?list=<id>`) don't expire; they break only when the playlist is deleted or made private.
//! `si=` is share tracking and changes nothing. Spotify invite links for private / collaborative playlists expire
//! after 7 days, and `spotify.link` short links go through a third-party redirect that may lapse, so Pandora saves the
//! canonical link, never the one pasted. YouTube Mixes (`RD…` except editorial `RDCLAK5uy_`) are rebuilt on every
//! visit, so a sync swaps songs instead of following a fixed list.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::AppHandle;

use crate::http;
use crate::models::{PlaylistLink, Track};
use crate::playlist_import::{
    ImportSource, MAX_IMPORT_TRACKS, SeedTrack, fetch_spotify_embed, is_spotify_id, match_seeds_on_ytm,
};
use crate::recommend::song_key_of;
use crate::ytmusic::YtMusicClient;

/// Spotify's public page lists at most this many songs.
const SPOTIFY_PAGE_LIMIT: usize = 100;

/// What a link's page lists: YouTube Music songs, or Spotify songs still to be matched.
pub enum LinkItems {
    Tracks(Vec<Track>),
    Seeds(Vec<SeedTrack>),
}

pub struct FetchedLink {
    pub name: String,
    pub items: LinkItems,
    pub total: usize,
    /// `false` when the list was cut short, so songs not seen can't be called missing.
    pub complete: bool,
    pub note: Option<String>,
}

/// `("spotify" | "ytmusic", canonical URL)` for a link source; `None` for tracklists and unresolved short links.
pub fn canonical_link(source: &ImportSource) -> Option<(&'static str, String)> {
    match source {
        ImportSource::YouTubePlaylist { list_id } => {
            Some(("ytmusic", format!("https://music.youtube.com/playlist?list={list_id}")))
        }
        ImportSource::Spotify { kind, id } => Some(("spotify", format!("https://open.spotify.com/{kind}/{id}"))),
        ImportSource::SpotifyShort { .. } | ImportSource::TextList { .. } => None,
    }
}

/// Follows a `spotify.link` short link to the playlist or album it names; other sources pass through.
pub fn resolve_short_link(source: ImportSource) -> Result<ImportSource, String> {
    let ImportSource::SpotifyShort { url } = source else {
        return Ok(source);
    };
    let expired = || {
        "That spotify.link short link has expired or doesn’t lead to a playlist. In Spotify, use Share → Copy link to \
         playlist and paste that instead."
            .to_string()
    };
    let stream = http::open_request("GET", &url, &[], None, Some(http::DEFAULT_UA), 15_000)?;
    if !(200..=299).contains(&stream.meta.status) {
        return Err(expired());
    }
    let bytes = stream.read_all(2 * 1024 * 1024)?;
    find_spotify_target(&String::from_utf8_lossy(&bytes)).ok_or_else(expired)
}

/// The first `open.spotify.com/…/playlist|album/<id>` in a page (the short link's redirect target or `og:url`).
pub fn find_spotify_target(html: &str) -> Option<ImportSource> {
    let html = html.replace("\\/", "/");
    let marker = "open.spotify.com/";
    let mut rest = html.as_str();
    while let Some(at) = rest.find(marker) {
        rest = &rest[at + marker.len()..];
        let end =
            rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '/' || c == '-' || c == '_')).unwrap_or(rest.len());
        let segments: Vec<&str> = rest[..end].split('/').filter(|s| !s.is_empty()).collect();
        for pair in segments.windows(2) {
            let kind = match pair[0] {
                "playlist" => "playlist",
                "album" => "album",
                _ => continue,
            };
            if is_spotify_id(pair[1]) {
                return Some(ImportSource::Spotify { kind, id: pair[1].to_string() });
            }
        }
    }
    None
}

/// Why syncing a YouTube list can't follow a fixed song list, if it's a Mix.
pub fn mix_note(list_id: &str) -> Option<String> {
    (list_id.starts_with("RD") && !list_id.starts_with("RDCLAK5uy_")).then(|| {
        "This is a YouTube Mix: YouTube builds it fresh each time, so syncing swaps songs rather than following a \
         fixed list."
            .to_string()
    })
}

pub fn fetch_link(source: &ImportSource, ytm: &YtMusicClient) -> Result<FetchedLink, String> {
    match source {
        ImportSource::YouTubePlaylist { list_id } => {
            let (name, tracks, complete) = ytm.fetch_playlist(list_id)?;
            let note = mix_note(list_id)
                .or_else(|| (!complete).then(|| "Only the first songs of this long playlist were read.".to_string()));
            Ok(FetchedLink { name, total: tracks.len(), items: LinkItems::Tracks(tracks), complete, note })
        }
        ImportSource::Spotify { kind, id } => {
            let (name, mut seeds) = fetch_spotify_embed(kind, id)?;
            let complete = seeds.len() < SPOTIFY_PAGE_LIMIT && seeds.len() <= MAX_IMPORT_TRACKS;
            seeds.truncate(MAX_IMPORT_TRACKS);
            let note = (!complete).then(|| {
                "Spotify’s public page shows up to 100 songs. For larger playlists, paste an Exportify CSV in the \
                 Paste / CSV tab."
                    .to_string()
            });
            Ok(FetchedLink { name, total: seeds.len(), items: LinkItems::Seeds(seeds), complete, note })
        }
        ImportSource::SpotifyShort { .. } | ImportSource::TextList { .. } => {
            Err("Paste a Spotify or YouTube Music playlist link".into())
        }
    }
}

/// Where Spotify songs find their YouTube Music match, cheapest first.
pub struct SeedContext<'a> {
    /// The link's earlier matches (`PlaylistLink::matches`).
    pub known: &'a BTreeMap<String, String>,
    /// Every song Pandora knows (`tracks_index`), to turn a known id back into a song.
    pub index: &'a HashMap<String, Track>,
    /// The playlist's songs, matched by title + main artist.
    pub current: &'a [Track],
}

/// One slot per seed (`None` = no match): earlier match, then a playlist song with the same title and main artist,
/// then a YouTube Music search for the rest.
pub fn resolve_seeds(
    seeds: &[SeedTrack],
    ctx: &SeedContext<'_>,
    source_label: &str,
    ytm: &YtMusicClient,
    cancel: &AtomicBool,
    app: &AppHandle,
) -> Result<Vec<Option<Track>>, String> {
    let by_key: HashMap<String, &Track> = ctx.current.iter().map(|t| (song_key_of(&t.title, &t.artist), t)).collect();
    let mut slots: Vec<Option<Track>> = seeds
        .iter()
        .map(|seed| {
            let key = seed_key(seed);
            ctx.known.get(&key).and_then(|id| ctx.index.get(id)).or_else(|| by_key.get(&key).copied()).cloned()
        })
        .collect();
    let open: Vec<usize> = (0..seeds.len()).filter(|&i| slots[i].is_none()).collect();
    let to_search: Vec<SeedTrack> = open.iter().map(|&i| seeds[i].clone()).collect();
    let found = match_seeds_on_ytm(&to_search, source_label, ytm, cancel, app)?;
    for (slot, track) in open.into_iter().zip(found) {
        slots[slot] = track;
    }
    Ok(slots)
}

pub fn seed_key(seed: &SeedTrack) -> String {
    song_key_of(&seed.title, &seed.artist)
}

/// `Artist — Title` for each seed without a match.
pub fn unmatched_labels(seeds: &[SeedTrack], slots: &[Option<Track>]) -> Vec<String> {
    seeds
        .iter()
        .zip(slots)
        .filter(|(_, slot)| slot.is_none())
        .map(
            |(seed, _)| {
                if seed.artist.is_empty() { seed.title.clone() } else { format!("{} — {}", seed.artist, seed.title) }
            },
        )
        .collect()
}

/// The link to save: its songs in order, plus the Spotify seed → id matches.
pub fn link_record(service: &str, url: String, seeds: Option<&[SeedTrack]>, found: &[Option<Track>]) -> PlaylistLink {
    let mut track_ids = Vec::new();
    for track in found.iter().flatten() {
        if !track_ids.contains(&track.id) {
            track_ids.push(track.id.clone());
        }
    }
    let matches = seeds
        .map(|seeds| {
            seeds
                .iter()
                .zip(found)
                .filter_map(|(seed, slot)| Some((seed_key(seed), slot.as_ref()?.id.clone())))
                .collect()
        })
        .unwrap_or_default();
    PlaylistLink { service: service.to_string(), url, synced_at_ms: now_ms(), track_ids, matches }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_link_pages_resolve_to_the_canonical_playlist() {
        let branch = r#"<script>window.top.location.replace("https:\/\/open.spotify.com\/playlist\/37i9dQZF1DXcBWIGoYBM5M?si=x&_branch_match_id=1")</script>"#;
        let source = find_spotify_target(branch).unwrap();
        assert_eq!(
            canonical_link(&source),
            Some(("spotify", "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M".into()))
        );
        let og = r#"<a href="https://open.spotify.com/user/abc">x</a><meta property="og:url" content="https://open.spotify.com/intl-de/album/1A2B3C4D5E6F7G8H9I0J1K">"#;
        assert_eq!(
            find_spotify_target(og),
            Some(ImportSource::Spotify { kind: "album", id: "1A2B3C4D5E6F7G8H9I0J1K".into() })
        );
        assert_eq!(find_spotify_target("<html>Spotify – Web Player</html>"), None);
    }

    #[test]
    fn only_rebuilt_youtube_mixes_get_the_mix_note() {
        assert!(mix_note("RDAMVMabc123").is_some());
        assert!(mix_note("RDCLAK5uy_editorial").is_none());
        assert!(mix_note("PL4fGSI1pDJn6puJdseH2Rt9sMvt9E2M4i").is_none());
    }

    #[test]
    fn link_record_keeps_order_drops_duplicates_and_maps_seeds() {
        let track = |id: &str| Track {
            id: id.into(),
            source: "ytmusic".into(),
            title: id.into(),
            artist: String::new(),
            album: String::new(),
            duration_ms: None,
            artwork_url: None,
            local_path: None,
            cached: false,
            liked: false,
        };
        let seed = |title: &str| SeedTrack { title: title.into(), artist: "Band".into(), duration_ms: None };
        let seeds = [seed("One"), seed("Two"), seed("Three"), seed("One (Live)")];
        let found = [Some(track("a")), None, Some(track("b")), Some(track("a"))];
        let link = link_record("spotify", "https://open.spotify.com/playlist/x".into(), Some(&seeds), &found);
        assert_eq!(link.track_ids, vec!["a", "b"]);
        assert_eq!(link.matches.get(&seed_key(&seeds[2])).map(String::as_str), Some("b"));
        assert!(!link.matches.contains_key(&seed_key(&seeds[1])));
        assert_eq!(unmatched_labels(&seeds, &found), vec!["Band — Two"]);
    }
}
