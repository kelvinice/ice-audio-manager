# D-22 · Gemini's vote

> Cast 2026-11-23 blind (§0.4) from the D-22 entry and my own inspection of `crates/ice-win/src/media.rs`, `app/src-tauri/tauri.conf.json`, `docs/reference/Echo-Music/lrclib/.../LrcLib.kt` (idea/protocol check only; GPL-3.0), `docs/reference/Echo-Music/assets/banner.png`, `docs/reference/BitChord/Banner.png`, and `docs/reference/screens/auralys-android-promo-v4.1.0.png`. I have not opened any other agent's vote file for D-22.

## Scores

#### Part A *pick one* (network use for music features)

| Option | Score | One-line reason |
|---|---|---|
| N0 · Keep §1.5 as it is | +1 | Zero network requests preserves the strict privacy promise cleanly, though it locks out LRCLIB lyrics permanently |
| N1 · Opt-in per-feature network use | +2 | Default off with zero background traffic, explicit destination disclosure in Settings, and local caching satisfies both §1.5 trust and owner lyrics |
| N2 · General network permission | −2 | **Block:** blanket permission without per-feature switches and destination lists breaks Guiding Constraint 3 ("not malware") and §1.5 |

#### Part B *pick many* (YouTube Music)

| Option | Score | One-line reason |
|---|---|---|
| B1 · Control only (today) + "Open YouTube Music" shortcut | +2 | Works cleanly today via GSMTC in Chrome/Edge/PWA with zero WebView2 RAM bloat or policy risk |
| B2 · Embedded YouTube Music window | −1 | Turns an audio manager into a full browser/player (+150–300 MB WebView2 while open, weakens §6 CSP, stores Google sign-in cookies) |
| B3 · Native player via unofficial APIs / stream extraction | −2 | **Block:** violates YouTube API stream-separation policies, breaks when InnerTube shifts, and risks SmartScreen/Store rejection (D-09, Phase 6) |

#### Part C *pick many* (lyrics)

| Option | Score | One-line reason |
|---|---|---|
| K0 · No lyrics | 0 | Safe baseline if N1 fails, though opt-in LRCLIB is clean and isolated |
| K1 · LRCLIB, opt-in (needs N1) | +2 | Key-free, public `lrclib.net` lookup once per track change with local LRC caching fits cleanly behind an opt-in switch |
| K2 · Other public/licensed providers | −1 | API keys, per-site Terms/licensing issues, and mostly plain-text scraping add fragility for little gain |
| K3 · YouTube Music unofficial lyrics | −2 | **Block:** scraping private YouTube Music endpoints shares the policy breakage and trust risk of B3 |

## Strongest argument against my top choice

- **Against N1 + K1 (`lrclib.net` opt-in lyrics):** Even when opt-in and off by default, sending every played `(title, artist, album, duration)` tuple to a third-party server reveals listening habits if enabled without clear expectations, and relaxing `§1.5` ("the *only* network call allowed is the opt-in update check") opens a precedent for future remote features. In addition, synced LRC accuracy in `app/ui` relies on interpolating between sparse GSMTC `TimelineChanged` events (`media.rs:314-362`), because Chrome/YouTube Music updates `position_ms` mainly on track change, seek, or pause/play—meaning live lyrics can drift slightly on long YouTube Music tracks or buffered seeks.

## New option or amendment

1. **N1 guardrail (keep Tauri WebView CSP untouched):** Keep `§6`'s strict frontend CSP (`app/src-tauri/tauri.conf.json:15`, no remote `connect-src` or `img-src` in the WebView!) even under N1. All `lrclib.net` HTTP fetches must live in **Rust** (domain-allowlisted to `lrclib.net` only), while Svelte renders cached LRC strings over Tauri IPC—so the Phase 6 WebView network trace stays empty and untrusted CSS themes still cannot exfiltrate anything.
2. **K1 UI runtime rule (§2 zero idle loops):** Synced lyric highlighting may tick only while the lyrics view is **visible** and `media.status === 'playing'`. On pause, hide, or WebView suspend, stop the line timer immediately (`0` ticks when idle or hidden). Also support local `.lrc` sidecar files or no-network fallback gracefully when N1/K1 are toggled off.
3. **B1 empty-state polish (Phase 3/4):** Pair *"Open YouTube Music"* / *"Open Spotify"* quick-launch links in the NotchBox-style empty media state (`media == null`, no network needed—just launching the installed PWA/URL via user click).

## Evidence I checked myself

- `crates/ice-win/src/media.rs:194-223, 314-362` & `media_service.rs:73, 152`: GSMTC already reads `title`, `artist`, `album`, `status`, clamped `position_ms`, and `duration_ms` locally for any Windows session (including YouTube Music in browsers/PWAs), emitting `MediaSignal::TimelineChanged` only on discrete seek/state/track edges (`PLAYBACK_BITS`), confirming UI-side position interpolation is required for LRC sync.
- `docs/reference/Echo-Music/lrclib/src/main/kotlin/com/music/lrclib/LrcLib.kt:31-89` (concept verified, zero code copied — GPL-3.0): confirms `https://lrclib.net/api/search` takes unauthenticated `track_name`, `artist_name`, and `album_name` query parameters without an API key, matching candidates by track duration after stripping title suffixes.
- `app/src-tauri/tauri.conf.json:15`: confirms WebView CSP is currently `default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; img-src 'self' data: iceart: http://iceart.localhost; style-src 'self' 'unsafe-inline'`, meaning Rust-only HTTP under N1 preserves strict frontend isolation without altering WebView CSP.
- `docs/reference/Echo-Music/assets/banner.png`, `docs/reference/BitChord/Banner.png`, and `docs/reference/screens/auralys-android-promo-v4.1.0.png`: inspected owner screenshots; BitChord/Echo-Music are full Android YouTube Music replacement players (mirroring B3/K3 scope that contradicts D-05), whereas Auralys focuses on Android sharing/listening rooms.

## Position

**Part A:** Vote **`N1 (+2)`** (with `N0 +1`, `N2 −2`). An opt-in, default-off, per-feature network permission with strict domain allowlisting in Rust, explicit destination/data disclosure in Settings, local caching, and **no relaxation of WebView CSP** preserves Phase 6's zero-network default trace while letting interested users enable synced lyrics cleanly.

**Part B:** Keep **`B1 (+2)`** (GSMTC control + an explicit user-triggered launcher shortcut in the empty card). Reject **`B2 (−1)`** because hosting `music.youtube.com` inside our process tree drags browser-scale RAM, cookies, and CSP exceptions into a lightweight audio manager, and block **`B3 (−2)`** for ToS, breakage, and SmartScreen/Store signing risks.

**Part C:** Adopt **`K1 (+2)`** (`lrclib.net` opt-in in Phase 4+ after Settings & privacy note land), keeping lyric interpolation strictly scoped to visible playing views (§2) and formatting synced lines per D-23 V6. Reject `K2 (−1)` and block `K3 (−2)`.
