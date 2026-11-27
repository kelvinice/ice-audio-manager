# ice-win media (now playing)

> Paths: `crates/ice-win/src/{media.rs, media_service.rs}` · Owner: Claude · Verified: 2026-11-23

## Purpose
Reads what is playing from Windows' Global System Media Transport Controls (GSMTC): track, status, timeline and album art, and
sends transport commands. Works for any app with a media session (Spotify is the primary QA target, Q-09; YouTube Music in a
browser or installed app is the second). No network, no sign-in.

## Files
| File | What it does |
|---|---|
| `media.rs` | Stateless WinRT reads: `manager`, `now_playing`, `sessions`, `send(action)`, `current_art`, `read` (track + art), `read_within_cached` (skips PNG encode when `ArtInfo` matches, keeps `prev` state on same-track reads), `refresh_playback` (status/timeline; `stale_resume` only from `Paused`/`Stopped`); `MediaWatcher` + `MediaSignal` forward GSMTC events to a non-blocking sink; art pipeline |
| `media_service.rs` | `MediaService`: the `ice-media` watcher thread + `ice-media-cmd` transport thread (`MediaCommandSender`, queued seek coalescing); coalesces GSMTC signals and stops confirm reads once both track and new art arrive |
| `lyrics.rs`, `lyrics/{cache,lrclib,romaji,words}.rs` | `LyricsService` (`ice-lyrics` STA thread): Option B local `.lrc` overrides (`%LOCALAPPDATA%\IceAudioManager\lyrics\custom\`) & bounded LRU disk cache (`cache_max_mb`, 1 h hit-touch interval, metadata-first eviction), Option A non-blocking cancellable WinHTTP LRCLIB (`https://lrclib.net`) on short-lived `ice-lyrics-net` workers (250 ms skip debounce, max 2 in flight), and offline Japanese → Romaji via `JapanesePhoneticAnalyzer` / MS-IME `IFELanguage::GetJMorphResult` + `kana_to_romaji` (kana lines spaced at ICU word breaks, `words.rs`); `open_language_settings` (`ms-settings:regionlanguage`) |

## How it works
- **Event-driven, no polling.** `MediaWatcher` forwards session / properties / playback / timeline events. `media_service` sets bits in a
  dirty mask (`SESSION_BITS`, `PLAYBACK_BITS`) and wakes at most once per burst, then waits `SETTLE` (60 ms) so a track change (properties +
  playback + timeline together) is read once. Outbound `Command::Media` actions run on `ice-media-cmd` (`MediaCommandSender`, coalescing burst
  seeks and dropping stale seeks before `Next`/`Previous`) so track skips and seeks never wait behind slow GSMTC art/property reads or `ice-audio`.
- **Confirm reads (`CONFIRM_AFTER`: +200 ms, +800 ms, +2 s after a track read).** Spotify can fire "properties changed" before the new
  title is readable, so the first read may return the old track; the re-reads correct it, emit only if something changed, and disarm early as
  soon as both a new track and new art are observed. A timeline duration jump (`DURATION_JUMP_MS`) on a playback-only event also forces a track
  read. The first read gives the cover `FIRST_ART_BUDGET` (250 ms, `media::read_within_cached`) so a slow cover can't hold back the title; confirm
  re-reads use `CONFIRM_ART_BUDGET` (1.2 s) and skip PNG re-encoding when the decoded thumbnail hash matches `known_art`.
- **Own MTA thread (`ice-media`).** Every call here blocks on cross-process WinRT, so it must never run on a UI (STA) thread or on
  `ice-audio` (a slow media app must not delay a device switch). MED-09.
- **Bounded:** `CALL_LIMIT` 500 ms per WinRT call, `ART_BUDGET` 2 s for all art work for one read; oversized art is rejected before decode (MED-02/03).
- **Art pipeline:** Windows' imaging API (no image crate): decode, Fant downscale to ≤ 128 px (`ART_PX`), EXIF orientation, straight alpha → PNG;
  content-hash id for `iceart://<id>`; accent via `ice_core::media::accent_for_art` / `readable_accent`.
- The engine merges `MediaUpdate` into `snapshot.media`, emits `MediaChanged`, and keeps the last 8 PNGs for `Engine::art`.
- Commands: `Command::Media { action }` → `media::send` (`PlayPause`, `Play`, `Pause`, `Next`, `Previous`, and `Seek { position_ms }` via `TryChangePlaybackPositionAsync` relative to `Timeline.StartTime`, clamped to `[StartTime, EndTime]`).

## Traps
- **Never log track titles above `debug`** (privacy, code-practices §4).
- A new track is decided by app + title + artist + album only. Art arriving late must update in place, not replay the track animation
  (twin rule in `app-shell`'s `TrackKey` and the UI's `isSameTrack`).
- Position updates are timeline events; the UI interpolates between them. Don't add polling for the scrubber.

## Tests
Unit tests for colour maths live in `ice-core`; this module is covered by the manual checklist with Spotify. Review: [media-review](../reviews/media-review.md)
(MED-01–MED-10, all resolved). Measurements: first `MediaChanged` ≈ 100 ms after start, art read ≈ 31 ms (Spotify).

## Change log
- 2026-11-23 Claude: initial recap.
- 2026-11-24 Claude: confirm reads, duration-jump detection and a short first-read art budget (fixes a ~5 s stale title on song change).
- 2026-11-24 Gemini: added `MediaAction::Seek { position_ms }` via `TryChangePlaybackPositionAsync` (`seek_ticks` / `clamp_seek_ticks`).
- 2026-11-24 Gemini: `refresh_playback` now populates `NowPlaying.position_updated_ms` from GSMTC `TimelineProperties.LastUpdatedTime` (with stale-resume guard).
- 2026-11-24 Gemini: added `lyrics/` (`LyricsService` on `ice-lyrics` thread, Option B local `.lrc` + LRU disk cache, Option A opt-in WinHTTP LRCLIB client, and offline MS-IME `IFELanguage` Romaji).
- 2026-11-24 Claude: lyrics cache `format: 2` (older records dropped), generated Romaji stored in `LyricLine.romaji` and stripped before caching; IME word breaks for kana-only lines; particles as spoken.
- 2026-11-24 Claude: Romaji fixed: `ice-lyrics` thread is STA (both Windows backends are STA classes); `JapanesePhoneticAnalyzer` first, `IFELanguage` second (probed once), kana last; `romaji_reads_kanji()` for Settings. Kanji Romaji needs Japanese installed in Windows (lyrics.md §9).
- 2026-11-24 Gemini: split `ice-media-cmd` (`MediaCommandSender` + seek/skip coalescing) off GSMTC reads, added `read_within_cached` + early confirm disarm, made LRCLIB lookups non-blocking/cancellable (`ice-lyrics-net`, 250 ms debounce, max 2 in flight) with metadata-first cache eviction and 1 h hit-touch throttling.
- 2026-11-25 Claude: kana-only Romaji is spaced: `lyrics/words.rs` reads word breaks from Windows' ICU (`icuuc.dll` `ubrk_*`, `UBRK_WORD`, `ja`; feature `Win32_Globalization`), filtered by `ice_core::lyrics::join_confident_kana_words` ("ittai konomama itsu made" instead of "ittaikonomamaitsumade"). `JapaneseRomaji::recheck` re-probes the analyzer once per loaded song while kana-only, so adding Japanese works from the next song without a restart. `open_language_settings` opens `ms-settings:regionlanguage` (shared `shell_open`).
- 2026-11-25 Claude: `media.rs` fills `ArtInfo.palette` / `luminance_pct` (`ice_core::media::art_palette`) from the same downscaled pixels as the accent.
- 2026-11-25 Gemini: fixed Spotify mid-song clock rewind (`stale_resume` limited to `Paused`/`Stopped`, `read_within_cached` preserves `prev` on same-track reads) and refreshed cached line-synced word cadence on cache hits.
- 2026-11-27 Claude: `com::ComGuard::init_sta` is public so Pandora can read Japanese Romaji (phonetic analyzer / IME need COM on the calling thread).
