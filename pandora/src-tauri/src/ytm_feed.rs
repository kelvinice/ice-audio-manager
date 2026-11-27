//! Parsing for YouTube Music browse pages: the home feed's shelves and collection (playlist / album / artist)
//! pages. The HTTP side lives in `ytmusic.rs`; everything here is pure `serde_json::Value` walking.
//!
//! Home is A/B served: some responses lead with a "Quick picks" song list, others only with editorial playlist
//! carousels. Keeping both songs and collections per shelf makes either variant render.

use serde_json::Value;

use crate::models::{CollectionCard, HomeShelf, Track};
use crate::ytmusic::{collect_tracks, parse_responsive_item, parse_two_row_item, pick_thumbnail, runs_text};

const SECTION_LISTS: [&str; 2] = [
    "/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer",
    "/continuationContents/sectionListContinuation",
];
const SHELF_KINDS: [&str; 3] =
    ["musicCarouselShelfRenderer", "musicImmersiveCarouselShelfRenderer", "musicShelfRenderer"];

/// Shelves on one home page (first page or a continuation) plus the token for the next page.
pub(crate) fn parse_home_page(val: &Value) -> (Vec<HomeShelf>, Option<String>) {
    let Some(list) = SECTION_LISTS.iter().find_map(|path| val.pointer(path)) else {
        return (Vec::new(), None);
    };
    let shelves = list
        .get("contents")
        .and_then(Value::as_array)
        .map(|sections| sections.iter().filter_map(parse_shelf).collect())
        .unwrap_or_default();
    let token = list
        .pointer("/continuations/0/nextContinuationData/continuation")
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .map(str::to_string);
    (shelves, token)
}

fn parse_shelf(section: &Value) -> Option<HomeShelf> {
    let shelf = SHELF_KINDS.iter().find_map(|kind| section.get(*kind))?;
    let header = shelf.get("header").and_then(first_child);
    let title = header.and_then(|h| runs_text(h.get("title"))).or_else(|| runs_text(shelf.get("title")))?;
    let strapline = header.and_then(|h| runs_text(h.get("strapline")));
    let mut out = HomeShelf { title, strapline, ..HomeShelf::default() };
    for item in shelf.get("contents").and_then(Value::as_array).into_iter().flatten() {
        if let Some(row) = item.get("musicResponsiveListItemRenderer") {
            out.tracks.extend(parse_responsive_item(row));
        } else if let Some(card) = item.get("musicTwoRowItemRenderer") {
            if let Some(track) = parse_two_row_item(card) {
                out.tracks.push(track);
            } else if let Some(collection) = parse_collection_card(card) {
                out.collections.push(collection);
            }
        }
    }
    (!out.tracks.is_empty() || !out.collections.is_empty()).then_some(out)
}

/// A two-row card that opens a playlist, album or artist page.
pub(crate) fn parse_collection_card(card: &Value) -> Option<CollectionCard> {
    let endpoint = card.pointer("/navigationEndpoint/browseEndpoint")?;
    let browse_id = endpoint.get("browseId").and_then(Value::as_str)?;
    let page_type = endpoint
        .pointer("/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType")
        .and_then(Value::as_str)
        .unwrap_or("");
    let kind = collection_kind(page_type)?;
    Some(CollectionCard {
        browse_id: browse_id.to_string(),
        kind: kind.to_string(),
        title: runs_text(card.get("title"))?,
        subtitle: runs_text(card.get("subtitle")).unwrap_or_default(),
        artwork_url: pick_thumbnail(card.pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails")),
    })
}

fn collection_kind(page_type: &str) -> Option<&'static str> {
    match page_type {
        "MUSIC_PAGE_TYPE_PLAYLIST" => Some("playlist"),
        "MUSIC_PAGE_TYPE_ALBUM" | "MUSIC_PAGE_TYPE_AUDIOBOOK" => Some("album"),
        "MUSIC_PAGE_TYPE_ARTIST" | "MUSIC_PAGE_TYPE_USER_CHANNEL" => Some("artist"),
        _ => None,
    }
}

/// A browse id we are willing to send back to YouTube Music (playlist, album, artist or channel ids).
pub(crate) fn is_valid_browse_id(id: &str) -> bool {
    (2..=128).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// `(title, subtitle, artwork)` from a collection page header.
pub(crate) fn parse_collection_header(val: &Value) -> (Option<String>, String, Option<String>) {
    let Some(header) = find_header(val) else {
        return (None, String::new(), None);
    };
    let title = runs_text(header.get("title"));
    let parts: Vec<String> = ["straplineTextOne", "subtitle", "secondSubtitle"]
        .iter()
        .filter_map(|key| runs_text(header.get(*key)))
        .collect();
    let art = first_thumbnails(header).and_then(|thumbs| pick_thumbnail(Some(thumbs)));
    (title, parts.join(" • "), art)
}

fn find_header(val: &Value) -> Option<&Value> {
    const HEADERS: [&str; 5] = [
        "musicResponsiveHeaderRenderer",
        "musicImmersiveHeaderRenderer",
        "musicVisualHeaderRenderer",
        "musicDetailHeaderRenderer",
        "musicEditablePlaylistDetailHeaderRenderer",
    ];
    match val {
        Value::Object(map) => {
            for key in HEADERS {
                if let Some(found) = map.get(key) {
                    // The editable header wraps a responsive one.
                    return Some(found.pointer("/header/musicResponsiveHeaderRenderer").unwrap_or(found));
                }
            }
            // Carousels carry their own (title-only) headers; never take those as the page header.
            map.iter().filter(|(key, _)| !SHELF_KINDS.contains(&key.as_str())).find_map(|(_, child)| find_header(child))
        }
        Value::Array(items) => items.iter().find_map(find_header),
        _ => None,
    }
}

fn first_thumbnails(val: &Value) -> Option<&Value> {
    match val {
        Value::Object(map) => map.get("thumbnails").or_else(|| map.values().find_map(first_thumbnails)),
        Value::Array(items) => items.iter().find_map(first_thumbnails),
        _ => None,
    }
}

/// Tracks from a collection page's own track list (`musicShelfRenderer` / `musicPlaylistShelfRenderer`), not
/// from "More like this" carousels beside it. Falls back to every track on the page.
pub(crate) fn collect_page_tracks(val: &Value, out: &mut Vec<Track>) {
    fn walk(val: &Value, out: &mut Vec<Track>) -> bool {
        let mut found = false;
        match val {
            Value::Object(map) => {
                for (key, child) in map {
                    match key.as_str() {
                        "musicPlaylistShelfRenderer" | "musicShelfRenderer" => {
                            collect_tracks(child, out);
                            found = true;
                        }
                        "musicCarouselShelfRenderer" | "musicImmersiveCarouselShelfRenderer" => {}
                        _ => found |= walk(child, out),
                    }
                }
            }
            Value::Array(items) => {
                for child in items {
                    found |= walk(child, out);
                }
            }
            _ => {}
        }
        found
    }
    if !walk(val, out) {
        collect_tracks(val, out);
    }
}

fn first_child(val: &Value) -> Option<&Value> {
    val.as_object().and_then(|map| map.values().next())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn song_row(id: &str, title: &str) -> Value {
        json!({ "musicResponsiveListItemRenderer": {
            "playlistItemData": { "videoId": id },
            "flexColumns": [
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [{ "text": title }] } } },
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [{ "text": "Artist" }] } } }
            ]
        }})
    }

    fn playlist_card(id: &str, title: &str) -> Value {
        json!({ "musicTwoRowItemRenderer": {
            "title": { "runs": [{ "text": title }] },
            "subtitle": { "runs": [{ "text": "Playlist" }] },
            "navigationEndpoint": { "browseEndpoint": {
                "browseId": id,
                "browseEndpointContextSupportedConfigs": { "browseEndpointContextMusicConfig": {
                    "pageType": "MUSIC_PAGE_TYPE_PLAYLIST" } }
            }}
        }})
    }

    fn carousel(title: &str, items: Vec<Value>) -> Value {
        json!({ "musicCarouselShelfRenderer": {
            "header": { "musicCarouselShelfBasicHeaderRenderer": {
                "title": { "runs": [{ "text": title }] }, "strapline": { "runs": [{ "text": "FOR YOU" }] } } },
            "contents": items
        }})
    }

    #[test]
    fn home_keeps_song_and_playlist_shelves_with_titles() {
        let page = json!({ "contents": { "singleColumnBrowseResultsRenderer": { "tabs": [{ "tabRenderer": {
            "content": { "sectionListRenderer": {
                "contents": [
                    carousel("Quick picks", vec![song_row("a", "One"), song_row("b", "Two")]),
                    carousel("Rain + Feels", vec![playlist_card("VLabc", "Mellow")]),
                    { "musicTastebuilderShelfRenderer": {} }
                ],
                "continuations": [{ "nextContinuationData": { "continuation": "next-page" } }]
            }}
        }}]}}});
        let (shelves, token) = parse_home_page(&page);
        assert_eq!(token.as_deref(), Some("next-page"));
        assert_eq!(shelves.len(), 2);
        assert_eq!(shelves[0].title, "Quick picks");
        assert_eq!(shelves[0].strapline.as_deref(), Some("FOR YOU"));
        assert_eq!(shelves[0].tracks.len(), 2);
        assert_eq!(shelves[1].collections[0].browse_id, "VLabc");
        assert_eq!(shelves[1].collections[0].kind, "playlist");
    }

    #[test]
    fn continuation_pages_parse_the_same_way() {
        let page = json!({ "continuationContents": { "sectionListContinuation": {
            "contents": [carousel("New releases", vec![song_row("c", "Three")])]
        }}});
        let (shelves, token) = parse_home_page(&page);
        assert_eq!(shelves[0].tracks[0].id, "c");
        assert!(token.is_none());
    }

    #[test]
    fn collection_tracks_skip_side_carousels() {
        let page = json!({
            "header": { "musicResponsiveHeaderRenderer": {
                "title": { "runs": [{ "text": "Album" }] },
                "straplineTextOne": { "runs": [{ "text": "Band" }] },
                "subtitle": { "runs": [{ "text": "Album" }, { "text": " • " }, { "text": "2024" }] },
                "thumbnail": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [
                    { "url": "https://i.ytimg.com/cover.jpg", "width": 544 } ] } } }
            }},
            "list": { "musicShelfRenderer": { "contents": [song_row("t1", "Song")] } },
            "side": carousel("More", vec![song_row("x", "Other")])
        });
        let mut tracks = Vec::new();
        collect_page_tracks(&page, &mut tracks);
        assert_eq!(tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["t1"]);
        let (title, subtitle, art) = parse_collection_header(&page);
        assert_eq!(title.as_deref(), Some("Album"));
        assert_eq!(subtitle, "Band • Album • 2024");
        assert_eq!(art.as_deref(), Some("https://i.ytimg.com/cover.jpg"));
    }

    #[test]
    fn browse_ids_are_restricted_to_safe_characters() {
        assert!(is_valid_browse_id("VLRDCLAK5uy_l7wbVbkC-dG5fyEQQsBfjm"));
        assert!(is_valid_browse_id("MPREb_4jKsEXKGWTS"));
        assert!(!is_valid_browse_id("VL../../x"));
        assert!(!is_valid_browse_id(""));
    }
}
