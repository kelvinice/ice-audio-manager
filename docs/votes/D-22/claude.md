# D-22 · Claude's vote

> Sealed at proposal time (§0.4 rule 2). Claude wrote the entry; it contains no scores or verdicts.

## Scores

| Option | Score | One-line reason |
|---|---|---|
| N0 · Keep §1.5 | −1 | Rules out every lyrics option the owner asked about |
| N1 · Opt-in, per-feature network | +2 | Keeps "no network unless you turned it on, and here's exactly what's sent" |
| N2 · General permission | −1 | Loses the per-feature switch that makes the network use checkable |
| B1 · Control + shortcut | +2 | YouTube Music already works through GSMTC; a shortcut is all it needs to feel supported |
| B2 · Embedded YouTube Music window | −1 | Turns the audio manager into a browser shell for one site; the browser/web app already does this |
| B3 · Native unofficial player | −2 | **Block:** conflicts with YouTube's policy on separating audio, and a signed, published app (D-09) can't ship that |
| K0 · No lyrics | −1 | The owner wants lyrics, and K1 can do it opt-in |
| K1 · LRCLIB opt-in | +2 | Open database, synced lyrics, metadata-only lookup, works for both services |
| K2 · Other providers | 0 | Terms, keys and licences unknown; mostly no sync |
| K3 · YouTube Music unofficial lyrics | −2 | **Block:** same unofficial-endpoint problem as B3, and only covers one service |

## Strongest argument against my top choice (N1 + K1)

Today the app makes **zero** network calls, which is a strong trust signal for an app people may suspect of being malware
(§1, the Phase 6 FAQ "Is this malware?"). Any lookup sends listening history (artist, title) to a volunteer-run third party.
That party can go away, rate-limit us or log requests. The feature is nice to have; the zero-network story is part of the
product.

## New option or amendment

- **K1 on demand:** fetch lyrics only when the user opens the lyrics view (or presses "Show lyrics"), not on every track
  change. Less data leaves the machine, and nothing is sent while the user just listens.
- **N1 wording:** Settings → Privacy lists every enabled network feature with its destination. "All network features off"
  is one switch and the default.
- **B1:** the empty state offers "Open Spotify" and "Open YouTube Music" (the installed web app if present, otherwise the
  browser).

## Evidence I checked myself

- `docs/reference/Auralis/README.md:213`: "❌ No streaming service (local files only)".
  `auralis-web/backend/services/lyrics_extraction.py`: lyrics read from file tags; header "AGPL-3.0-or-later".
- `crates/ice-core/src/media.rs:78-79`: `position_ms` / `duration_ms` are already in `NowPlaying`, so synced lyrics need no
  new media API, only interpolation.
- `docs/code-practices.md` §1.5 and §6 (`CSP enabled. No remote URLs loaded`): B2 needs both changed; K1 needs §1.5 only
  (a Rust-side fetch, nothing loaded into a WebView).
- YouTube API Services developer policies (search result, Google's own page): API clients must not let users download or
  separate audio tracks.

## Position

Ship B1 now: it's a UI change with no new risk. Adopt N1 as the rule for any future network feature. Build K1 on demand after
Phase 4, when Settings can host the switch and the privacy listing. Don't build B2 or B3.

## Revision round (after reveal, 2026-11-23)

No score changes. I back both amendments, which tighten my own "on demand" idea:
- Codex: a one-track lookup first, showing the destination and fields; results bound to the original track; late responses
  dropped after a switch; the cache cleared when the feature is turned off; no match rather than a mistimed line.
- Gemini: the fetch lives in Rust with an `lrclib.net` allowlist, so the WebView CSP (`tauri.conf.json`) stays unchanged; the
  lyrics timer ticks only while visible and `Playing`; B1 quick-launch links in the empty state.
