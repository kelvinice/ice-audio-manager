# Pandora plan

> Living roadmap for the Pandora player (`pandora/`). What exists today is in
> [modules/pandora](modules/pandora/README.md); the user guide is [pandora.md](pandora.md). Ice Audio Manager's own plan
> is [plan.md](plan.md). Last updated 2026-11-28.

## Where it stands

Pandora plays YouTube Music and local files, with playlists, downloads, recommendations, Magic shuffle, an EQ, synced
lyrics in three styles, song recognition and full Ice integration. It is a prototype: features move fast, quality gates
are lighter than Ice's, and some files have grown large.

## Now: make it solid

Do these before adding much more.

| # | Task | Why |
|---|---|---|
| N1 | Add `svelte-check` and Vitest to `pandora/ui`, and run them from `pandora\build.cmd check` | `tsc` doesn't read `.svelte` files, so component type errors only show at run time. Pure helpers (`magicShuffle`, `breath`, `spectrum`, `lyricWords`, `fader`) have no tests |
| N2 | Make `pandora/src-tauri` clean under `cargo fmt` and `clippy -D warnings` | About 25 existing clippy errors and wide format drift hide new problems |
| N3 | Split the large files: `lib/audio.ts` (~750 lines: move queue and shuffle into `lib/queue.ts`), `App.svelte` (~790: routing and handlers), `NowPlayingOverlay.svelte` (~810: lyrics column, controls), `SettingsView.svelte` (~700: one component per section, as Storage and Experimental already are) | Over the ~600-line file and ~300-line component targets |
| N4 | Queue panel shows the real shuffle order | Shuffle picks the next song when it's needed, so "Next up" shows list order, not what will play. Precompute the order per pass |
| N5 | A short rule for settings changes in `modules/pandora/backend.md` | There is no schema version: a new field needs a safe default, and renaming a key is the only way to reset a value for everyone |

## Next: features

Order is a suggestion; the owner picks.

| # | Feature | Notes |
|---|---|---|
| F1 | Local file tags (title, artist, album, cover from ID3 / Vorbis / MP4) | Needs a tag crate (`lofty`, MIT/Apache); new dependency, so owner approval first |
| F2 | True crossfade (the next song starts under the current one) | Two `<audio>` elements and two source nodes into the same graph. Graduates "Smooth transitions" |
| F3 | Mini player | A small always-on-top window (cover, title, transport). New Tauri window and capability |
| F4 | Listening stats | Play counts and listening time per song and artist (history keeps only recent songs today), then a "Your top" page |
| F5 | Lyrics timing per song | A per-song offset (±) saved with the song; later a simple tap-to-sync editor |
| F6 | In-app shortcuts | Space, ←/→ seek, Ctrl+←/→ skip, L like, plus a "?" sheet. App window only, never global hooks |
| F7 | Playlist editing | Drag to reorder, sort, duplicate, playlist folders |

## Later: bigger moves

- **Rust audio pipeline** (`symphonia` decode, WASAPI output): gapless playback, exclusive mode, lower idle memory.
  Large; see "Option C" in [pandora.md](pandora.md).
- **Smart playlists** built from rules (artist, liked, play count, added date).
- Anything that syncs between PCs must stay opt-in (no silent network).

## Experimental

Settings → Experimental (`settings.experimental`, `ExperimentalPanel.svelte`). Rules:

- Off by default, one switch each, explained in one line.
- Bounded work only: no timers except ≤ 30 Hz while visible and playing, with a clear stop; nothing with reduced motion.
- Cheap to remove: its own module where possible (`lib/fader.ts`, `lib/beatPulse.ts`).
- **Graduate** to a normal setting once it has run a week without problems; **remove** it if nobody turns it on.

| Feature | Status | What it does |
|---|---|---|
| Smooth transitions | Experimental (2026-11-28) | Songs fade in (1.2 s) and out over their last 3 s; skipping dips out (0.18 s). `lib/fader.ts` |
| Beat pulse | Experimental (2026-11-28) | The Now Playing cover swells on bass kicks. `lib/beatPulse.ts`, kick detection from `lib/breath.ts` |
| Gentle sleep timer | Experimental (2026-11-28) | The sleep timer lowers the music over its last 30 s |

Ideas not started yet:

- **Energy-matched Magic shuffle:** prefer recommendations close in tempo and loudness to the song just played.
- **Lyrics glow on the beat:** the current lyric line brightens on kicks (reuses `BassBreath`).
- **Focus mode:** Now Playing without lyrics or controls, only the backdrop and a large clock, for long listening.

## Open questions for the owner

1. Which "Next" feature first? F1 (local tags) and F2 (crossfade) need the most groundwork.
2. Should Pandora adopt Ice's stricter rules (CSP, profiling budgets, UI tests) before a first public build?
3. Packaging: a separate installer, or shipped with Ice?
