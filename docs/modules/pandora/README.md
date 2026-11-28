# Pandora desktop player

> Paths: `pandora/src-tauri/`, `pandora/ui/`, shared artwork in `assets/src/pandora*.svg` · Owner: Pandora ·
> Verified: 2026-11-27

Pandora is a Windows music player (Tauri 2 + Svelte 5 + Web Audio) that streams YouTube Music, plays local files and
works with Ice Audio Manager. Ice never depends on it. This folder is the developer recap: read the README, then the
one area doc for your task. The user-facing guide, the build commands and the Ice integration story are in
[docs/pandora.md](../../pandora.md). Lyric style designs are in [pandora-lyric-styles.md](../../pandora-lyric-styles.md).
The roadmap and the rules for experimental features are in [pandora-plan.md](../../pandora-plan.md).

## Features today

| Area | What the user gets | Doc |
|---|---|---|
| Home and search | YouTube Music home shelves (songs + playlist / album / artist cards), app-wide search, collection pages you can save | [library-and-discovery](library-and-discovery.md) |
| Library | Favorites, playlists (create, rename, delete, cover mosaic), history, downloads, local folders | [library-and-discovery](library-and-discovery.md) |
| Recommendations | "Recommended songs" under each playlist, picked from the songs in it, with Add and Refresh | [library-and-discovery](library-and-discovery.md) |
| Import | YouTube Music / Spotify playlist links, CSV / M3U / text tracklists | [library-and-discovery](library-and-discovery.md) |
| Playback | Queue panel, shuffle / Magic shuffle (recommended songs in between) / repeat, autoplay radio, 10-band EQ + AutoEq import, normalize, skip silence, speed, sleep timer, output device picker, Windows media controls, resume the last song at launch | [playback](playback.md) |
| Now Playing | Big cover, art-coloured animated backdrop (Blend, Calm or Lively lava lamp), Ice-style spectrum, wavy seek bar, liked heart burst | [playback](playback.md), [appearance](appearance.md) |
| Lyrics | Synced lyrics (Default, Fisheye, Rain styles), estimated word timing, romaji for Japanese / Korean / Cyrillic | [lyrics](lyrics.md) |
| Song recognition | Name the song playing on the PC (or the mic) via a Shazam-compatible fingerprint; history with play / save | [ice-and-recognition](ice-and-recognition.md) |
| Ice integration | Ice Audio hub (devices, volume, spatial, meter, notch controls), embedded or connected to a running Ice, Ice can ask Pandora to recognise a song | [ice-and-recognition](ice-and-recognition.md) |
| Look and feel | Themes (Midnight, Ice, Light Ice, Daylight, Ember, Use Windows), optional artwork accents, custom title bar, Figtree font | [appearance](appearance.md) |
| Experimental | Smooth transitions, beat pulse, gentle sleep timer (Settings → Experimental, all off) | [playback](playback.md), [plan](../../pandora-plan.md) |
| Storage | Library JSON, audio download cache, artwork disk cache with a budget, Settings → Storage | [backend](backend.md) |

## Layers

```
pandora/ui (Svelte 5, presentation + Web Audio playback)
   │  Tauri commands (lib/ipc.ts)          events: library://changed/v1, playlist://import/v1,
   │  <audio>/<img> via 127.0.0.1 proxy    recognition://status/v1, ice://{status,snapshot,meter}/v1,
   ▼                                       companion://recognized/v1
pandora/src-tauri (Rust: main.rs commands, YouTube Music client, library store, proxy, recognition, Ice bridges)
   │
   ▼
crates/ice-core + crates/ice-win (lyrics parsing / romaji, audio engine, companion protocol)
```

Settings and library state live in Rust (`models.rs`, `library.rs`); the UI renders them and sends changes back.
Playback itself (the `<audio>` element, EQ, spectrum) is in the UI's Web Audio graph (`lib/audio.ts`).

## File map

**Rust (`pandora/src-tauri/src/`)**

| File | What it does | Doc |
|---|---|---|
| `main.rs` | `AppState`, every Tauri command, tray, window close-to-tray, startup flags | [backend](backend.md) |
| `startup.rs` | Launch at sign-in (own HKCU Run value, `--background`; locked in debug builds) | [backend](backend.md) |
| `models.rs` | Shared types: `Track`, `Playlist`, `LibraryState`, `PandoraSettings`, feed / lyrics / Ice types | [backend](backend.md) |
| `library.rs` | `library.json` store, local folder scan, offline audio cache | [backend](backend.md) |
| `http.rs` | WinHTTP client (no extra TLS crates) | [backend](backend.md) |
| `proxy.rs` | `127.0.0.1` proxy: `/stream/yt/<id>`, `/stream/local`, `/art` | [backend](backend.md) |
| `artcache.rs` | Artwork disk cache (LRU by modified time) | [backend](backend.md) |
| `ytmusic.rs` | YouTube Music InnerTube client: search, browse, related (radio), stream resolve | [library-and-discovery](library-and-discovery.md) |
| `ytm_feed.rs` | Parses home shelves and collection pages | [library-and-discovery](library-and-discovery.md) |
| `recommend.rs` | Playlist recommendations: seed choice and ranking | [library-and-discovery](library-and-discovery.md) |
| `playlist_import.rs` | Spotify / YouTube Music / CSV / M3U import | [library-and-discovery](library-and-discovery.md) |
| `lyrics.rs` | Lyrics lookup + word timing + romaji (via `ice-core` / `ice-win`) | [lyrics](lyrics.md) |
| `recognition.rs` | WASAPI capture, fingerprint, Shazam lookup | [ice-and-recognition](ice-and-recognition.md) |
| `ice_bridge.rs` | Pandora → Ice: embedded engine or connected to a running Ice | [ice-and-recognition](ice-and-recognition.md) |
| `companion.rs` | Ice → Pandora loopback bridge (`ping`, `show`, `recognize`) | [ice-and-recognition](ice-and-recognition.md) |

**UI (`pandora/ui/src/`)**

| Path | What it does | Doc |
|---|---|---|
| `App.svelte` | Shell: routes and history, library state, notices, wires every view | [appearance](appearance.md) |
| `app.css`, `palettes.css` | Buttons and base styles; theme tokens `--p-*` | [appearance](appearance.md) |
| `lib/audio.ts` | `audioEngine` + `playerStore`: playback, queue, EQ graph, sleep timer, media session | [playback](playback.md) |
| `lib/ipc.ts`, `lib/types.ts` | Command wrappers and event listeners; TS mirrors of `models.rs` | [backend](backend.md) |
| `lib/theme.ts` | Artwork URL resolving, cover colour extraction, artwork-accent switch | [appearance](appearance.md) |
| `lib/session.ts` | Last queue / position in `localStorage` | [playback](playback.md) |
| `lib/fader.ts`, `lib/beatPulse.ts` | Experimental fades (transitions, sleep) and cover beat pulse | [playback](playback.md) |
| `lib/devices.ts` | Device kind / headphone icon and spatial sound labels | [ice-and-recognition](ice-and-recognition.md) |
| `lib/magicShuffle.ts` | Magic shuffle: recommended songs between list songs | [playback](playback.md) |
| `lib/breath.ts` | Bass breath for the Lively backdrop (Twin of Ice's `ambient.ts`) | [playback](playback.md) |
| `lib/spectrum.ts`, `lib/wave.ts` | Spectrum shaping (Twin of Ice's `beat.ts`), wavy seek line | [playback](playback.md) |
| `lib/lyricWords.ts` | Word timing helpers for lyric styles | [lyrics](lyrics.md) |
| `lib/parametricEq.ts` | AutoEq `ParametricEQ.txt` parser | [playback](playback.md) |
| `lib/format.ts`, `lib/demoTracks.ts` | Formatting; offline demo tracks (`--demo`) | — |
| `components/` | Views and controls; each area doc lists its own | per area |

## Where to look

| Task | Start in |
|---|---|
| New setting | `models.rs` `PandoraSettings` (+ default, + test in `library.rs`), `lib/types.ts`, `SettingsView.svelte` |
| New command | `main.rs` (+ `generate_handler!`), wrapper in `lib/ipc.ts` |
| Something about a song list | `TrackTable.svelte` (all lists) or `SongRow.svelte` (home rows) |
| Playback bug | `lib/audio.ts` |
| Colours | `palettes.css` tokens, never literals in components |

## Rules that bite

- Settings have no schema version: every field needs `#[serde(default)]`-safe defaults, and renaming a key is the way
  to reset a value for everyone (see `artwork_accents`). Unknown enum values fall back with `#[serde(other)]`.
- Never log track titles above `debug` (project rule).
- Pandora's Rust crate is outside the workspace: run `cargo test --manifest-path pandora/src-tauri/Cargo.toml` and
  `npm --prefix pandora/ui run check` / `run build`, or `.\pandora\build.cmd check`.
- Existing Pandora files are not yet clean under `cargo fmt` and `clippy -D warnings`; keep new files clean.

## Change log

- 2026-11-28 · Launch at sign-in (`startup.rs`, `LaunchAtSignIn.svelte`, tray check item), release builds only.
- 2026-11-28 · Rain reveals words a little later again (`quickenWords` pace 0.72, ≤ 340 ms per word; 22 ms letter
  stagger, 0.38 s landing): 0.55 / 260 ms ran ahead of the voice.
- 2026-11-28 · Settings → Experimental (`settings.experimental`): smooth transitions, beat pulse, gentle sleep timer;
  roadmap in `docs/pandora-plan.md`.
- 2026-11-28 · Ice Audio page regrouped: status header, Sound and Notch cards, Connection card (`IceSoundCard`,
  `IceNotchCard`, `lib/devices.ts`).
- 2026-11-28 · Blend backdrop (blurred cover under a softer lava lamp, breathing together) is the new default.
- 2026-11-28 · Lively backdrop motion (`LavaLamp.svelte`, `lib/breath.ts`, `backdropMotion` setting; Calm stays default).
- 2026-11-28 · Light Ice theme (`iceLight`).
- 2026-11-28 · Storage: distinct classes fix the shifted bar segment and download badge; no per-song list; reset card as
  a row with a solid confirm. Settings tabs as a segmented control with animated switching; Library tabs fade in.
- 2026-11-28 · Magic shuffle (`lib/magicShuffle.ts`); shuffle plays each song once per pass.
- 2026-11-28 · Spectrum: the bass bars no longer move as one block (4096-point FFT, centre interpolation for narrow
  bands) and sit lower (bass trim, 72 % reach); the wavy seek bar's head is a round dot instead of an upright pill.
- 2026-11-27 · Module doc split into this folder (README + area docs).
- 2026-11-27 · Rain reveals words sooner (`quickenWords`: 55 % of each word's delay, ≤ 260 ms per word) with a quicker
  drop, since word timings are mostly estimated and lagged the voice.
- 2026-11-27 · Playlist recommendations (`recommend.rs`, `ytm_recommend`, `PlaylistRecommendations.svelte`).
- 2026-11-27 · Buttons and accents keep the theme colour by default: `dynamicTheme` (on) became `artworkAccents` (off; the old
  key is ignored). Cover colours still tint the Now Playing backdrop.
- 2026-11-27 · Rain: a finished line falls from the same stack it was shown in (romaji kept, then faded), so lines with romaji
  no longer jump when they start falling.
- 2026-11-27 · Title bar snowflake: `ice_wake_and_peek` starts Ice if needed and asks it to wake and show the notch for 2 s
  (`peekNotch`; older Ice falls back to wake + `showNotch`).
- 2026-11-27 · Spectrum bars are all accent again (owner: no progress colouring; the seek bar shows progress).
- 2026-11-27 · Spectrum shaped like the Ice pill (`lib/spectrum.ts`, Twin of `beat.ts` / `visualizer.ts`: per-band AGC, tilt, gate,
  springs, resting texture, progress colouring); `HeartButton` (pop, ring and sparks on like; `--p-love`); Romaji fixed (lyrics
  fetch joins a COM STA so kanji are read) and shown in every lyric style; TrackTable download slot always present.
- 2026-11-27 · Lyric style picker moved to an icon menu beside the animated-background toggle; `MoreMenu` gains `checked`,
  `hint`, `heading` and `active`.
- 2026-11-27 · Lyric styles: Default, Fisheye, Rain (`lyricsStyle` setting, picker in the lyrics header); plan for Branches and
  Pulse reveal in `docs/pandora-lyric-styles.md`.
- 2026-11-27 · Figtree font; equalizer sheet rebuilt (`EqBand`, response curve, no sideways scroll); `VolumeControl` and
  `OutputPicker` in the player bar.
- 2026-11-27 · Now Playing backdrop (`NowPlayingBackdrop.svelte`: blurred cover + tint; with the animated background, art-coloured
  orbs on slow circular laps, paused with the song); lyrics follow the song until scrolled, then a bottom-middle Sync button
  re-centres and resumes; left column aligned to the cover width.
- 2026-11-27 · Launch shows the last song paused where it stopped (`lib/session.ts`, `restoreSession`); Settings → Storage
  rebuilt as `StorageSettings.svelte` with a usage bar and the download list (fixes the clipped stat cards).
- 2026-11-27 · Right-hand `QueuePanel` (Queue / Recently played) toggled from the player bar.
- 2026-11-27 · Shared `SeekBar` (Ice-style rail / wave); log-band spectrum; Now Playing lyrics without sideways scroll;
  bare icon buttons with a dark `.solid` variant, translucent `btn-ghost`, collapse / icon close buttons.
- 2026-11-27 · Sidebar as two panels (navigation; "Your Library" with pinned Favorites / Downloads and playlist covers) on
  the title-bar frame, no status dot; `Ice` theme preset (Ice Audio Manager's palette) in Rust, `palettes.css` and the picker.
- 2026-11-27 · Custom title bar with history, app-wide search and window buttons; SearchView uses `TrackTable` (with add
  to queue); `quit_app` command; `vite-env.d.ts` for asset imports.
- 2026-11-27 · Page headers draw their back link inside the tinted area and leave room for descenders; the sidebar's
  playlist actions are one `+` menu (`MoreMenu` takes `icon` / `buttonClass` / `align`).
- 2026-11-27 · Added theme presets, appearance previews and size-specific Windows icons.
- 2026-11-27 · Home feed parses titled shelves of songs and playlist / album cards (fixes the empty feed on the
  playlist-only variant); `ytm_collection` / `save_collection` commands; disk artwork cache (`artcache.rs`,
  `artworkCacheMb`); solid buttons and shared `TrackTable` / `PageHero` / `Shelf` layout; download all from playlists
  and collections.
- 2026-11-27 · Ice companion bridge (`companion.rs`), `--background` start with a hidden-by-default window, one guarded `recognize_now`, `library://changed/v1` + `companion://recognized/v1` UI events, and Ice's `install.json` in "Launch Ice".
