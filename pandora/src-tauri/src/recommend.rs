//! Playlist recommendations. YouTube Music's radio ("up next") lists songs related to a few seed songs spread across
//! the playlist. A song scores higher when several seeds suggest it, when it sits near the top of those lists, and
//! when its artist is already in the playlist. Songs already in the playlist (also under another video id) and songs
//! the caller has already shown are left out, and no artist takes more than two places.

use std::collections::{HashMap, HashSet};

use crate::models::Track;
use crate::ytmusic::YtMusicClient;

/// Seed songs asked per request (one radio request each, run in parallel).
pub const MAX_SEEDS: usize = 5;
/// Recommendations returned per request.
pub const MAX_RESULTS: usize = 10;
const MAX_PER_ARTIST: usize = 2;
/// How fast a suggestion's weight falls with its position in a seed's list.
const POSITION_FALLOFF: f64 = 0.15;
/// Bonus for an artist that the playlist already has.
const KNOWN_ARTIST_BOOST: f64 = 1.3;

/// Fetches radio lists for seeds from `playlist` and ranks them. `round` picks other seeds (Refresh); `skip` holds ids
/// that should not come back (recommendations already shown).
pub fn recommend(ytm: &YtMusicClient, playlist: &[Track], round: u32, skip: &[String]) -> Vec<Track> {
    let seeds = pick_seeds(playlist, round);
    let related: Vec<Vec<Track>> = std::thread::scope(|scope| {
        let handles: Vec<_> = seeds.iter().map(|seed| scope.spawn(|| ytm.related(&seed.id))).collect();
        handles.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    rank(playlist, &related, skip)
}

/// Up to [`MAX_SEEDS`] streamable songs spread evenly across the playlist; each `round` shifts them by one place so a
/// refresh asks about different songs.
pub fn pick_seeds(playlist: &[Track], round: u32) -> Vec<&Track> {
    let pool: Vec<&Track> = playlist.iter().filter(|t| t.source == "ytmusic").collect();
    if pool.len() <= MAX_SEEDS {
        return pool;
    }
    let step = pool.len() as f64 / MAX_SEEDS as f64;
    let shift = round as usize % step.ceil() as usize;
    let mut picked: Vec<usize> =
        (0..MAX_SEEDS).map(|i| ((i as f64 * step) as usize + shift).min(pool.len() - 1)).collect();
    picked.dedup();
    picked.into_iter().map(|i| pool[i]).collect()
}

/// Ranks the seeds' radio lists (see the module docs).
pub fn rank(playlist: &[Track], related: &[Vec<Track>], skip: &[String]) -> Vec<Track> {
    let mut excluded: HashSet<&str> = playlist.iter().map(|t| t.id.as_str()).collect();
    excluded.extend(skip.iter().map(String::as_str));
    let owned_songs: HashSet<String> = playlist.iter().map(song_key).collect();
    let known_artists: HashSet<String> = playlist.iter().map(|t| primary_artist(&t.artist)).collect();

    // Keyed by song (title + artist) so two uploads of one song add up instead of both showing.
    let mut scored: HashMap<String, (Track, f64, usize)> = HashMap::new();
    let mut order = 0;
    for list in related {
        for (pos, track) in list.iter().enumerate() {
            let key = song_key(track);
            if excluded.contains(track.id.as_str()) || owned_songs.contains(&key) {
                continue;
            }
            let weight = 1.0 / (1.0 + pos as f64 * POSITION_FALLOFF);
            scored.entry(key).and_modify(|(_, score, _)| *score += weight).or_insert_with(|| {
                order += 1;
                (track.clone(), weight, order)
            });
        }
    }

    let mut ranked: Vec<(Track, f64, usize)> = scored
        .into_values()
        .map(|(track, score, first)| {
            let boost = if known_artists.contains(&primary_artist(&track.artist)) { KNOWN_ARTIST_BOOST } else { 1.0 };
            (track, score * boost, first)
        })
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.2.cmp(&b.2)));

    let mut per_artist: HashMap<String, usize> = HashMap::new();
    ranked
        .into_iter()
        .filter(|(track, _, _)| {
            let n = per_artist.entry(primary_artist(&track.artist)).or_default();
            *n += 1;
            *n <= MAX_PER_ARTIST
        })
        .map(|(track, _, _)| track)
        .take(MAX_RESULTS)
        .collect()
}

/// One song across uploads: the title without bracketed extras ("(Official Video)", "[Lyrics]") plus the main artist.
fn song_key(track: &Track) -> String {
    song_key_of(&track.title, &track.artist)
}

/// `song_key` from a bare title and artist (playlist sync matches link songs to playlist songs with it).
pub(crate) fn song_key_of(title_raw: &str, artist: &str) -> String {
    let mut title = String::new();
    let mut depth = 0usize;
    for c in title_raw.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => title.extend(c.to_lowercase()),
            _ => {}
        }
    }
    let title: String = title.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{title}\u{1f}{}", primary_artist(artist))
}

/// The first credited artist, lowercased ("A, B", "A & B" and "A feat. B" all give "a").
fn primary_artist(artist: &str) -> String {
    let lower = artist.to_lowercase();
    let end = [",", " & ", " feat", " ft.", " x ", " • "].iter().filter_map(|sep| lower.find(sep)).min();
    lower[..end.unwrap_or(lower.len())].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str, title: &str, artist: &str) -> Track {
        Track {
            id: id.into(),
            source: "ytmusic".into(),
            title: title.into(),
            artist: artist.into(),
            album: String::new(),
            duration_ms: None,
            artwork_url: None,
            local_path: None,
            cached: false,
            liked: false,
        }
    }

    fn ids(tracks: &[Track]) -> Vec<&str> {
        tracks.iter().map(|t| t.id.as_str()).collect()
    }

    #[test]
    fn songs_several_seeds_agree_on_rank_first() {
        let playlist = [track("p1", "Seed One", "A"), track("p2", "Seed Two", "B")];
        let related = [
            vec![track("x", "Only Once", "C"), track("both", "Shared", "D")],
            vec![track("y", "Other", "E"), track("both", "Shared", "D")],
        ];
        assert_eq!(ids(&rank(&playlist, &related, &[]))[0], "both");
    }

    #[test]
    fn playlist_songs_skipped_ids_and_reuploads_are_left_out() {
        let playlist = [track("p1", "Hello", "Adele")];
        let related = [vec![
            track("p1", "Hello", "Adele"),
            track("p1-video", "Hello (Official Music Video)", "Adele, Someone"),
            track("seen", "Shown Before", "C"),
            track("new", "Fresh", "D"),
        ]];
        assert_eq!(ids(&rank(&playlist, &related, &["seen".into()])), ["new"]);
    }

    #[test]
    fn known_artists_rank_higher_and_no_artist_takes_more_than_two_places() {
        let playlist = [track("p1", "Song", "Known")];
        let related = [vec![
            track("a", "Stranger", "Other"),
            track("b", "Familiar", "Known"),
            track("c", "Other 2", "Other"),
            track("d", "Other 3", "Other"),
        ]];
        let out = rank(&playlist, &related, &[]);
        assert_eq!(ids(&out), ["b", "a", "c"]);
    }

    #[test]
    fn seeds_spread_over_the_playlist_and_rotate_each_round() {
        let playlist: Vec<Track> = (0..12).map(|i| track(&format!("t{i}"), &format!("S{i}"), "A")).collect();
        let first = ids(&pick_seeds(&playlist, 0).into_iter().cloned().collect::<Vec<_>>()).join(",");
        let second = ids(&pick_seeds(&playlist, 1).into_iter().cloned().collect::<Vec<_>>()).join(",");
        assert_eq!(first, "t0,t2,t4,t7,t9");
        assert_eq!(second, "t1,t3,t5,t8,t10");

        let mut local = track("l", "Local", "A");
        local.source = "local".into();
        assert!(pick_seeds(&[local], 0).is_empty());
    }

    #[test]
    fn primary_artist_drops_featured_artists() {
        assert_eq!(primary_artist("Daft Punk feat. Pharrell"), "daft punk");
        assert_eq!(primary_artist("A & B"), "a");
        assert_eq!(primary_artist("Solo"), "solo");
    }
}
