# Pandora library and discovery

> Paths: `ytmusic.rs`, `ytm_feed.rs`, `recommend.rs`, `playlist_import.rs`, `playlist_link.rs`, `playlist_sync.rs`; UI
> `HomeView`, `SearchView`, `CollectionView`, `LibraryView`, `PlaylistRecommendations`, `PlaylistModal`,
> `ImportPlaylistModal`, `SyncPlaylistModal`, `StorageSettings`,
> `Sidebar`, `TrackTable`, `SongRow`, `Shelf`, `CoverTile`, `PageHero` · Part of [Pandora](README.md)

## YouTube Music client (`ytmusic.rs`)

Clean-room InnerTube client over `http.rs`. `search` (songs filter), `browse` (sends the visitor id so pages stay on one
A/B variant), `related(video_id)` (the `next` endpoint with `RDAMVM<id>`, i.e. the song's radio, up to 15 tracks),
`fetch_playlist`, and the stream resolver (`ANDROID_VR`, `IOS`, `ANDROID_MUSIC` clients, then `yt-dlp` on `PATH`).
Audio-only formats get 403 without a proof-of-origin token, so Pandora streams the muxed file and offers no bitrate
setting.

## Home and collections (`ytm_feed.rs`, `HomeView`, `CollectionView`)

Home comes in variants: some lead with a "Quick picks" song list, others only with playlist carousels.
`HomeFeed.shelves` keeps each shelf's title, strapline, songs and collection cards (playlist / album / artist); `home()`
follows two continuations. A card opens with `ytm_collection` (tracks only from the page's own track shelf; album rows
inherit the album cover and name) and `save_collection` stores it as a library playlist. Layout: `Shelf` + `CoverTile`
+ `SongRow`.

## Search (`SearchView`)

The title bar's search field (Ctrl+K, / or Ctrl+L) opens Search after 350 ms of typing; results use `TrackTable` with
add to queue.

## Library (`LibraryView`, `Sidebar`)

Tabs: Playlists, Favorites, Downloads, Local files, History. A playlist page is `PageHero` (cover mosaic, play, shuffle,
menu with rename / download / delete) over `TrackTable`, then recommendations. The sidebar has navigation plus "Your
Library" with pinned Favorites / Downloads and playlist covers. Adding to playlists from any list goes through
`PlaylistModal`.

`TrackTable` is the one song list (playlists, collections, library tabs, search): `#`, title + artist, album, time,
hover actions (like, add to playlist, queue, download, remove), and a filter box from 12 songs.

## Playlist recommendations (`recommend.rs`, `PlaylistRecommendations.svelte`)

Under a playlist's songs: up to 10 songs with play, like and Add (always visible), plus Refresh.

1. `pick_seeds`: up to 5 streamable (`ytmusic`) songs spread evenly over the playlist; `round` shifts them by one place
   so Refresh asks about other songs.
2. `recommend`: one `related` radio per seed, in parallel (`thread::scope`).
3. `rank`: each suggestion scores `1 / (1 + 0.15 × position)` per seed list it is in (seeds that agree add up), ×1.3
   when its main artist is already in the playlist; at most 2 per artist. Left out: songs in the playlist, re-uploads of
   them (same title without bracketed extras + same main artist), and ids in `skip`.

The component loads once per playlist (`{#key playlist.id}` in `LibraryView`); Add removes the song from the list and
adds it to the playlist without reloading; Refresh passes everything shown so far as `skip`. Local-only playlists show
nothing.

## Import (`playlist_import.rs`, `ImportPlaylistModal`)

Public YouTube Music playlists (InnerTube browse), Spotify playlists / albums (public page, no API key; songs matched on
YouTube Music in parallel), or Exportify CSV / M3U / plain tracklists. Progress on `playlist://import/v1`, cancellable.
`spotify.link` short links are followed (best effort: the first `open.spotify.com/playlist|album/<id>` in the page).
A link import saves the canonical link on the playlist (`Playlist.link`); tracklists don't.

## Sync (`playlist_link.rs`, `playlist_sync.rs`, `SyncPlaylistModal`)

Playlist ⋯ menu → Sync. The modal offers "Sync from last link" (saved or imported link) or a new link, then shows a
review; nothing is saved before Apply.

- `preview_playlist_sync` reads the link (`fetch_link`). Spotify songs are matched cheapest first: the link's earlier
  matches (`PlaylistLink.matches`), a playlist song with the same title + main artist (`recommend::song_key_of`), then
  a YouTube Music search. This keeps a re-sync from turning a different search hit into "new + gone".
- `diff`: added = link songs not in the playlist; missing = songs that came from the link last time (`link.trackIds`)
  and are gone from it. On a first or different link every playlist song can be missing. Songs added in Pandora never are.
- The user picks Keep (default) or Remove. `apply_playlist_sync` appends new songs, removes only when asked, and saves
  the link with the new song ids, so kept songs become the user's and aren't asked about again.
- Removing is refused when the list may be cut short (`complete = false`): Spotify's public page lists 100 songs, and
  YouTube Music reading stops at 500 songs or a failed continuation (`browse_tracks` flag).

Link lifetimes (checked 2026-11-28, community sources; no official docs): plain playlist links don't expire, they break
only if the playlist is deleted or made private; `si=` is share tracking. Spotify invite links for private /
collaborative playlists expire after 7 days, and `spotify.link` short links go through a third-party redirect that may
lapse, so only the canonical link is saved. YouTube Mixes (`RD…` except editorial `RDCLAK5uy_`) are rebuilt on each
visit; import and sync add a note.

## Downloads, local files and storage (`StorageSettings`)

Download one song, a playlist or a collection (`download_track`); downloads have no size limit. Settings → Storage
shows a Downloads / Artwork usage bar (parts grow by size), the download count and size (View opens Library →
Downloads, where songs are listed and removed), the artwork cache budget, local folders, library counts, file locations
and Reset (its own card, two-step).
