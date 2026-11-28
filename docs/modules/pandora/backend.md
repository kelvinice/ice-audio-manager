# Pandora backend: commands, storage, proxy

> Paths: `pandora/src-tauri/src/{main,models,library,http,proxy,artcache}.rs`, `pandora/ui/src/lib/{ipc,types}.ts` ·
> Part of [Pandora](README.md)

## Commands (`main.rs`)

`AppState` holds the proxy port, demo flag, `YtMusicClient`, `LibraryStore`, `ArtCache`, `IceBridge` and cancel flags
for recognition and import. Blocking work (network, disk) runs in `spawn_blocking`. Groups in `generate_handler!`:

| Group | Commands |
|---|---|
| App | `get_proxy_port`, `quit_app` (skips close-to-tray), `is_demo_mode`, `get_app_paths`, `get_launch_at_sign_in`, `set_launch_at_sign_in` |
| YouTube Music | `ytm_home`, `ytm_search`, `ytm_collection`, `save_collection`, `ytm_related`, `ytm_recommend` |
| Library | `get_library`, `toggle_like`, `record_history`, playlist CRUD (`create_` / `rename_` / `delete_playlist`, `add_to_` / `remove_from_playlist`), `save_eq`, `save_settings`, `reset_settings` |
| Local and offline | `add_local_folder`, `remove_local_folder`, `rescan_local`, `download_track`, `delete_cached_track`, `clear_offline_cache`, `artwork_cache_bytes`, `clear_artwork_cache` |
| Lyrics | `fetch_lyrics` |
| Ice | `ice_status`, `ice_snapshot`, `ice_set_mode_pref`, `ice_switch_device`, `ice_set_volume`, `ice_toggle_mute`, `ice_toggle_mic_mute`, `ice_set_spatial`, `ice_patch_settings`, `ice_set_meter_active`, `ice_launch_standalone`, `ice_stop_standalone`, `ice_ping_bridge`, `ice_toggle_notch`, `ice_show_notch`, `ice_wake_and_peek`, `ice_open_settings` |
| Recognition | `recognize_music`, `cancel_recognition`, `delete_recognition_item`, `clear_recognition_history` |
| Import and sync | `import_playlist`, `cancel_playlist_import` (also cancels a sync preview), `preview_playlist_sync`, `apply_playlist_sync` |

There is no per-command permission file; registering in `generate_handler!` and adding a wrapper in `lib/ipc.ts` is
enough. Wrappers return harmless defaults outside Tauri (`inTauri`), so `npm run dev` previews still render.

Events: `library://changed/v1` (library edited from Rust, e.g. a recognition), `playlist://import/v1` (import
progress), `recognition://status/v1`, `ice://status/v1`, `ice://snapshot/v1`, `ice://meter/v1`,
`companion://recognized/v1`. Listeners are in `lib/ipc.ts`.

Startup flags: `--demo` (offline demo tracks), `--fake-engine` (Ice embedded engine with fake devices),
`--background` (stay in the tray; used when Ice starts Pandora and by launch at sign-in). The window starts hidden and
`setup` shows it.

Launch at sign-in (`startup.rs`): Pandora's own `HKCU\…\Run` value `Pandora` = `"<exe>" --background`, through
`ice_win::autostart` (`set_enabled_as`). Settings → General → Startup (`LaunchAtSignIn.svelte`) and a tray check item (re-read on
right-click) share it. Debug builds (`cfg!(debug_assertions)`) refuse to change it, so `target\debug` never lands in
the Run key; the toggle shows why it is locked.

## Library store (`library.rs`, `models.rs`)

- `%APPDATA%\Pandora\library.json`: liked, playlists, history, local folders and tracks, `tracksIndex` (id → `Track`,
  so playlists only store ids), cached ids, EQ, settings, recognition history, Ice mode preference. One `Mutex`,
  saved after each change.
- `Playlist.link` (optional, `PlaylistLink`): service, canonical URL, last sync time, the song ids that came from the
  link, and (Spotify) seed → YouTube Music id matches. Set by link imports and `apply_playlist_sync`.
- `PandoraSettings` is `#[serde(default)]`: missing fields take defaults, enums use `#[serde(other)]`. `save_settings`
  clamps values (volume 0–1). `reset_settings` resets settings and EQ but keeps the library.
- Offline audio: `%LOCALAPPDATA%\Pandora\cache\<id>`; `snapshot()` re-syncs `cached` flags from disk.
- Local folders: scanned for `.mp3 .flac .wav .ogg .m4a .opus`; title / artist come from `Artist - Title` file names
  and folder names (no tag parsing yet).

## Network and proxy

- `http.rs`: WinHTTP (Windows' own TLS and proxy settings), so no TLS crates. `post_json`, `get` with headers.
- `proxy.rs`: a `127.0.0.1:<ephemeral>` HTTP/1.1 server. `/stream/yt/<id>` serves the offline file or resolves and
  streams the YouTube audio with Range pass-through; `/stream/local?path=` streams a local file with seeking;
  `/art?url=` fetches allowed art hosts (through `artcache.rs`) so `<canvas>` can read pixels. The UI turns remote URLs
  into proxy URLs with `resolveArtworkUrl` (`lib/theme.ts`).
- `artcache.rs`: `%LOCALAPPDATA%\Pandora\artwork\<sha256(url)>.<ext>` with a 128-entry memory map in front. A hit
  refreshes the file time; over `settings.artworkCacheMb` (default 256, 0 = off) the oldest go until 90 % remains.

## Tests

`cargo test --manifest-path pandora/src-tauri/Cargo.toml`: library migration / persistence / reset (`library.rs`),
proxy helpers, art cache, feed parsing, import parsing, lyrics, recommendations, recognition maths.
