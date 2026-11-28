//! Sync a library playlist with its Spotify / YouTube Music link. `preview` reads the link and works out what changed
//! without saving; the UI shows it and the user picks whether songs that left the link go too; `apply` saves.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::AppHandle;

use crate::library::LibraryStore;
use crate::models::{LibraryState, PlaylistSyncPlan, Track};
use crate::playlist_import::{ImportSource, emit_progress, parse_import_input, progress};
use crate::playlist_link::{self, LinkItems, SeedContext};
use crate::ytmusic::YtMusicClient;

/// Reads `input` (or the playlist's saved link when empty) and returns what a sync would change.
pub fn preview(
    playlist_id: &str,
    input: Option<&str>,
    ytm: &YtMusicClient,
    library: &LibraryStore,
    cancel: &AtomicBool,
    app: &AppHandle,
) -> Result<PlaylistSyncPlan, String> {
    cancel.store(false, Ordering::Release);
    let lib = library.snapshot();
    let playlist =
        lib.playlists.iter().find(|p| p.id == playlist_id).ok_or_else(|| "Playlist not found".to_string())?;
    let raw =
        match input.map(str::trim).filter(|s| !s.is_empty()) {
            Some(pasted) => pasted.to_string(),
            None => playlist.link.as_ref().map(|link| link.url.clone()).ok_or_else(|| {
                "This playlist has no saved link yet. Paste a Spotify or YouTube Music link.".to_string()
            })?,
        };
    let parsed = parse_import_input(&raw)?;
    if matches!(parsed, ImportSource::TextList { .. }) {
        return Err("Paste a Spotify or YouTube Music playlist link. Tracklists can’t be synced.".into());
    }
    let label = crate::playlist_import::source_label(&parsed);
    emit_progress(app, progress("fetching", label, 0, 0, 0));
    let result = read_link(parsed, &lib, playlist_id, ytm, cancel, app);
    let stage = match &result {
        Ok(_) => "done",
        Err(_) if cancel.load(Ordering::Acquire) => "cancelled",
        Err(_) => "error",
    };
    emit_progress(app, progress(stage, label, 0, 0, 0));
    result
}

fn read_link(
    parsed: ImportSource,
    lib: &LibraryState,
    playlist_id: &str,
    ytm: &YtMusicClient,
    cancel: &AtomicBool,
    app: &AppHandle,
) -> Result<PlaylistSyncPlan, String> {
    let playlist =
        lib.playlists.iter().find(|p| p.id == playlist_id).ok_or_else(|| "Playlist not found".to_string())?;
    let source = playlist_link::resolve_short_link(parsed)?;
    let (service, url) = playlist_link::canonical_link(&source)
        .ok_or_else(|| "Paste a Spotify or YouTube Music playlist link".to_string())?;
    let fetched = playlist_link::fetch_link(&source, ytm)?;
    if cancel.load(Ordering::Acquire) {
        return Err("Sync cancelled".into());
    }
    // Same link as last time: only songs that came from it can go missing. A new link: any playlist song can.
    let previous = playlist.link.as_ref().filter(|link| link.url == url);
    let current: Vec<Track> = playlist.track_ids.iter().filter_map(|id| lib.tracks_index.get(id).cloned()).collect();
    let (seeds, slots) = match fetched.items {
        LinkItems::Tracks(tracks) => (None, tracks.into_iter().map(Some).collect::<Vec<_>>()),
        LinkItems::Seeds(seeds) => {
            let empty = Default::default();
            let known = previous.map_or(&empty, |link| &link.matches);
            let ctx = SeedContext { known, index: &lib.tracks_index, current: &current };
            let slots = playlist_link::resolve_seeds(&seeds, &ctx, service, ytm, cancel, app)?;
            (Some(seeds), slots)
        }
    };
    let unmatched_tracks = seeds.as_deref().map(|s| playlist_link::unmatched_labels(s, &slots)).unwrap_or_default();
    let link = playlist_link::link_record(service, url, seeds.as_deref(), &slots);
    let removable = previous.map_or(&playlist.track_ids, |link| &link.track_ids);
    let diff = diff(&playlist.track_ids, removable, slots.into_iter().flatten().collect());
    Ok(PlaylistSyncPlan {
        playlist_id: playlist.id.clone(),
        link,
        source_name: fetched.name,
        added: diff.added,
        missing: diff.missing_ids.iter().filter_map(|id| lib.tracks_index.get(id).cloned()).collect(),
        unchanged: diff.unchanged,
        unmatched_tracks,
        total_tracks: fetched.total,
        complete: fetched.complete,
        note: fetched.note,
    })
}

/// Saves a previewed sync. Songs that left the link are removed only when asked and only when the whole list was read.
pub fn apply(plan: PlaylistSyncPlan, remove_missing: bool, library: &LibraryStore) -> Result<LibraryState, String> {
    if remove_missing && !plan.complete {
        return Err("Pandora couldn’t read the whole list, so it won’t remove songs.".into());
    }
    let remove_ids: Vec<String> =
        if remove_missing { plan.missing.iter().map(|t| t.id.clone()).collect() } else { Vec::new() };
    library.apply_playlist_sync(&plan.playlist_id, plan.added, &remove_ids, plan.link)
}

struct SyncDiff {
    added: Vec<Track>,
    missing_ids: Vec<String>,
    unchanged: usize,
}

/// `found` (the link's songs, in order) against the playlist. Missing = `removable` songs still in the playlist that
/// the link no longer has.
fn diff(playlist_ids: &[String], removable: &[String], found: Vec<Track>) -> SyncDiff {
    let in_playlist: HashSet<&str> = playlist_ids.iter().map(String::as_str).collect();
    let mut seen = HashSet::new();
    let mut added = Vec::new();
    let mut unchanged = 0;
    for track in found {
        if !seen.insert(track.id.clone()) {
            continue;
        }
        if in_playlist.contains(track.id.as_str()) {
            unchanged += 1;
        } else {
            added.push(track);
        }
    }
    let mut missing_ids = Vec::new();
    for id in removable {
        if in_playlist.contains(id.as_str()) && !seen.contains(id) && !missing_ids.contains(id) {
            missing_ids.push(id.clone());
        }
    }
    SyncDiff { added, missing_ids, unchanged }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str) -> Track {
        Track {
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
        }
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn new_songs_are_added_and_link_songs_that_left_are_missing() {
        let playlist = ids(&["a", "b", "mine"]);
        let linked = ids(&["a", "b"]);
        let got = diff(&playlist, &linked, vec![track("b"), track("c"), track("c")]);
        assert_eq!(got.added.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["c"]);
        assert_eq!(got.missing_ids, ids(&["a"]));
        assert_eq!(got.unchanged, 1);
    }

    #[test]
    fn songs_added_in_pandora_are_never_missing_for_a_linked_playlist() {
        let got = diff(&ids(&["a", "mine"]), &ids(&["a"]), vec![track("a")]);
        assert!(got.missing_ids.is_empty());
        assert!(got.added.is_empty());
    }

    #[test]
    fn a_first_link_can_offer_any_playlist_song() {
        let playlist = ids(&["a", "mine"]);
        let got = diff(&playlist, &playlist, vec![track("a")]);
        assert_eq!(got.missing_ids, ids(&["mine"]));
    }

    #[test]
    fn songs_already_removed_by_hand_are_not_missing() {
        let got = diff(&ids(&["b"]), &ids(&["a", "b"]), vec![track("b")]);
        assert!(got.missing_ids.is_empty());
    }
}
