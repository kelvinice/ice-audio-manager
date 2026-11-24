# ice-win media (now playing)

> Paths: `crates/ice-win/src/{media.rs, media_service.rs}` · Owner: Claude · Verified: 2026-11-23

## Purpose
Reads what is playing from Windows' Global System Media Transport Controls (GSMTC): track, status, timeline and album art, and
sends transport commands. Works for any app with a media session (Spotify is the primary QA target, Q-09; YouTube Music in a
browser or installed app is the second). No network, no sign-in.

## Files
| File | What it does |
|---|---|
| `media.rs` | Stateless WinRT reads: `manager`, `now_playing`, `sessions`, `send(action)`, `current_art`, `read` (track + art), `refresh_playback` (status/timeline); `MediaWatcher` + `MediaSignal` forward GSMTC events to a non-blocking sink; art pipeline |
| `media_service.rs` | `MediaService`: the `ice-media` thread; coalesces signals, reads once per burst, returns `MediaUpdate` to the engine |

## How it works
- **Event-driven, no polling.** `MediaWatcher` forwards session / properties / playback / timeline events. `media_service` sets bits in a
  dirty mask (`SESSION_BITS`, `PLAYBACK_BITS`) and wakes at most once per burst, then waits `SETTLE` (60 ms) so a track change (properties +
  playback + timeline together) is read once.
- **Confirm reads (`CONFIRM_AFTER`: +300 ms, +1 s, +2.5 s after a track read).** Spotify can fire "properties changed" before the new
  title is readable, so the first read may return the old track; the re-reads correct it and emit only if something changed. A timeline
  duration jump (`DURATION_JUMP_MS`) on a playback-only event also forces a track read. The first read gives the cover only
  `FIRST_ART_BUDGET` (400 ms, `media::read_within`) so a slow cover can't hold back the title; the re-reads use the full `ART_BUDGET`.
- **Own MTA thread (`ice-media`).** Every call here blocks on cross-process WinRT, so it must never run on a UI (STA) thread or on
  `ice-audio` (a slow media app must not delay a device switch). MED-09.
- **Bounded:** `CALL_LIMIT` 500 ms per WinRT call, `ART_BUDGET` 2 s for all art work for one read; oversized art is rejected before decode (MED-02/03).
- **Art pipeline:** Windows' imaging API (no image crate): decode, Fant downscale to ≤ 128 px (`ART_PX`), EXIF orientation, straight alpha → PNG;
  content-hash id for `iceart://<id>`; accent via `ice_core::media::accent_for_art` / `readable_accent`.
- The engine merges `MediaUpdate` into `snapshot.media`, emits `MediaChanged`, and keeps the last 8 PNGs for `Engine::art`.
- Commands: `Command::Media { action }` → `media::send`.

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
