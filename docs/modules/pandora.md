# Pandora desktop player

> Paths include `pandora/src-tauri/`, `pandora/ui/`, and the shared desktop artwork in `assets/src/`.

## Appearance

Pandora saves its theme with the other player preferences in `library.json`. Rust owns the default and
deserializes older settings. The Svelte shell applies the selection to the document so the library, player,
overlays and settings share one palette. Use Windows follows the system color scheme live. Buttons and accents keep
the theme colour by default; the "Album artwork accents" switch (`artworkAccents`, off by default, sets
`data-artwork-accents` on the root) lets the cover's colour replace it. The cover's colours (`--art-*`, `lib/theme.ts`)
always feed the Now Playing backdrop. Only the most recent artwork decode can supply those colors.

The palette definitions are in `pandora/ui/src/palettes.css`. Settings previews use the same tokens as the
window. The Windows icon family and the sidebar mark share the Pandora diamond. Regenerate the native assets
with the script documented in [asset credits](../../assets/CREDITS.md).

## Ice companion bridge

`src-tauri/src/companion.rs` lets Ice drive Pandora (experimental): a loopback listener with a per-run token in
`%LOCALAPPDATA%Pandoraridge.json`, answering `ping`, `show` and `recognize` from `ice_core::companion`. Each
connection runs on its own short thread (at most four), because a recognition blocks for about 10 s. `install.json`
is written at every start so Ice can launch Pandora. `--background` keeps the window hidden. The window starts hidden
(`tauri.conf.json`) and `setup` shows it otherwise. `recognize_now` in `main.rs` is the single recognition path for
the Recognize view and for Ice. It refuses a second capture while one runs and emits `library://changed/v1`. A
recognition that Ice asked for also emits `companion://recognized/v1`, so the UI shows a notice. "Launch Ice" reads
Ice's own `install.json` first (`ice_bridge::find_standalone_ice_exe`).

## Home feed and collections

`ytmusic.rs` owns the HTTP side (`browse`, which sends the visitor id so pages stay on one A/B variant);
`ytm_feed.rs` parses pages. YouTube Music serves home in variants: some lead with a "Quick picks" song list, others
only with editorial playlist carousels. `HomeFeed.shelves` keeps every shelf with its title, strapline, songs and
collection cards (playlist / album / artist), and `home()` follows two continuations so song shelves further down
arrive too. A card opens with `ytm_collection` (tracks only from the page's own track shelf, not "More like this";
album rows inherit the album cover and name). `save_collection` stores one as a library playlist.

## Artwork cache

`artcache.rs` keeps covers in `%LOCALAPPDATA%\Pandora\artwork\<sha256(url)>.<ext>` behind the `/art` proxy (a
128-entry memory map stays in front). A hit refreshes the file's modified time; past the budget
(`settings.artworkCacheMb`, default 256 MB, 0 = off) the least recently used files go until 90 % remains. Settings →
Storage shows usage and clears it.

## Layout

Shared pieces: `TrackTable` (every song list: playlists, collections, library tabs), `PageHero` + `MoreMenu`
(playlist / album headers with a round play button; rename, download and delete live in the menu), `Shelf` +
`CoverTile` + `SongRow` (home rows). Buttons come from `app.css`: solid accent `btn-primary` / `btn-play`, a translucent pill `btn-ghost`, and
`btn-icon`, bare until hovered, with `.solid` (a dark circle) for the one key control in a spot: collapse, back, carousel
arrows, title-bar Home / Ice. Closing uses icons (collapse for Now Playing, x for dialogs), not "Close" labels.

`SeekBar.svelte` (player bar and Now Playing) follows Ice's notch scrubber: one pointer slider with keyboard steps, a rail
that thickens on hover, or with `wavyProgress` a travelling wave as thick as the flat rest (`lib/wave.ts`, a Twin of
`app/ui/src/lib/notch/waves.ts`), animated at most 30 Hz while playing. Now Playing's spectrum reads 1024-point FFT
bands spaced logarithmically (`getSpectrumBars`), drawn DPR-sharp at 30 Hz while playing; lyrics only scroll vertically,
wrap long lines, mark the current line by colour, and pause auto-follow for 3 s after a manual scroll.

## Window frame

The native frame is off (`decorations: false`). `TitleBar.svelte` draws the caption: app menu (incl. `quit_app`, which
bypasses close-to-tray), Back / Forward over the route history in `App.svelte` (also Alt+Left/Right and mouse buttons
4/5), Home, the app-wide search field (Ctrl+K, / or Ctrl+L; typing opens Search, which waits 350 ms), Identify, Ice, and
minimize / maximize / close. Empty bar space carries `data-tauri-drag-region`; the window permissions are in
`capabilities/default.json`.

## Queue panel

`QueuePanel.svelte` is the right-hand panel the player bar's queue button opens and closes: Queue (now playing, then
the rest of `playerStore.queue`, click to play, remove on hover) and Recently played (`library.history`). Whether it is open
is a per-machine view preference in `localStorage` (`pandora.queuePanelOpen`).

## Last session at launch

`lib/session.ts` keeps the queue, position in it and playback position in `localStorage` (`pandora.session.v1`, saved every
5 s while playing, on pause and on `pagehide`). At launch `audioEngine.restoreSession(history[0])` shows that song paused at
the saved spot (queue, lyrics, artwork colours) without streaming; Play resumes there (`currentTime` set before metadata).
A saved session whose song is no longer the latest in history falls back to that latest song from 0:00.

## Storage settings

`StorageSettings.svelte`: total use with a Downloads / Artwork bar, Downloads (no size limit, first five listed with remove,
"Show all" opens Library → Downloads, Remove all with a confirm), the artwork cache budget, local files, library counts,
file locations, and Reset in its own panel.

## Type, equalizer and volume

The UI font is Figtree (OFL-1.1, `@fontsource-variable/figtree`, bundled by Vite into `dist`, so nothing is fetched at run
time and the CSP stays `self`); Segoe UI, Yu Gothic UI and Malgun Gothic cover other scripts. `EqDrawer` is a right sheet:
10 `EqBand` sliders (CSS-drawn; the native vertical range input overflowed) over a Catmull-Rom response curve, presets,
sound toggles, speed steps, sleep timer and AutoEq import. The player bar uses `VolumeControl` (seek-bar-style slider,
wheel 5 %, arrows 2 %, level tooltip) and `OutputPicker` (device popover with headphone / speaker icons from the
endpoint form factor).

## Lyric styles

Now Playing offers Default, Fisheye (`LyricsFisheye.svelte`) and Rain (`LyricsRain.svelte`), saved as `settings.lyricsStyle` and chosen from the lyrics
icon in Now Playing's top bar (a `MoreMenu` with checked items and hints) (Rust `LyricsStyle`, unknown values read as Default). Word timings come from `lib/lyricWords.ts`. Design notes and the plan
for Branches and Pulse reveal: [Pandora lyric styles](../pandora-lyric-styles.md).

## Audio quality (not offered)

Pandora streams the ANDROID client's muxed file. Audio-only formats from every client tried (ANDROID, ANDROID_VR,
IOS), at every bitrate, return 403 partway through without a proof-of-origin token, so a bitrate setting would cut
songs off. Revisit once a PO-token provider exists; `yt-dlp` on `PATH` stays the fallback.

## Checks

The library store test covers migration, theme persistence and reset. The offline demo provides cover art
and lyrics for visual checks. See the [Pandora guide](../pandora.md) for launch commands.

## Change log

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
