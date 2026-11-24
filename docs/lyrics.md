# Synced Lyrics, Romaji Subtitles & Storage Cache Design

> **Status:** Implemented (Slices 1–5) · **Related decisions:** D-22 (Lyrics & Privacy), D-23 V6 (Lyrics styling), D-15 (WebView optional), §1.5 (Zero silent network), §1.8 (MIT / clean-room).

---

## 1. Goals & Non-Negotiable Constraints

Ice Audio Manager's lyrics feature must feel effortless and alive while staying true to the app's lightweight, privacy-first principles ([`docs/code-practices.md`](code-practices.md)):

1. **Zero silent network (§1.5):** Online lyrics lookup is **off by default**. Users explicitly opt in from `Settings → Privacy`, which lists the exact host (`lrclib.net`) and metadata sent (`title`, `artist`, `album`, `duration`).
2. **Rust-only network & strict CSP:** All HTTP requests happen in Rust (`crates/ice-win`) to an allowlisted host. The WebView CSP remains zero-network (`'self'` + `iceart://` only).
3. **100 % offline capability:** Local `.lrc` files (Option B) and Japanese → Romaji transliteration work with zero network access.
4. **No background polling (§2):** Lyrics sync directly to the `NowPlaying.position_updated_ms` clock from Windows GSMTC (`LastUpdatedTime`). Timers run **only** while the notch is visible, expanded/showing lyrics, and playing — stopping immediately on pause, hide, suspend, or `Lite`/gaming mode.
5. **MIT & clean-room only (§1.8):** Implemented from scratch against public specs (LRCLIB API docs, LRC / Enhanced LRC syntax, Windows `IFELanguage` COM docs) using only MIT-compatible crates.

---

## 2. UI/UX Design — Bottom Extension of the Pill & Expanded Card

Instead of a detached floating window or cramped overlay, lyrics appear as a **seamless bottom extension** of the active pill or expanded card:

- **One living capsule:** The pill/card smoothly grows downward using the existing `(k, c, m)` `SpringGroup` (`w`, `h`, `r`), sharing the exact same `clip-path: inset(… round …)`, continuous 1 px SVG specular rim (`vector-effect="non-scaling-stroke"`), sibling `.pill-shadow`, and D-25 ambient lava-lamp wash (`NotchBackdrop`).
- **Soft hairline separator:** A subtle 1 px `--ice-control-border` divider separates the player header from the attached lyrics drawer, with soft top/bottom fade masks (`mask-image: linear-gradient(...)`) so lines dissolve gracefully at the edges.

### 2.1 Compact Music Pill + Bottom Extension (`track` mode)

```text
╭───────────────────────────────────────────────────────────╮
│  [Art]  Idol                                  ||||||||||| │  ← 46 px music pill
│         YOASOBI                                           │
├───────────────────────────────────────────────────────────┤  ← subtle hairline
│  ♪  無敵の笑顔で荒らすメディア                            │  ← current line (word highlight)
│     muteki no egao de arasu media                         │  ← optional Romaji / paired subtitle
│     知りたいその秘密ミステリアス                          │  ← next line (smaller, dimmed)
╰───────────────────────────────────────────────────────────╯
```

- **Two rows by default (owner, 2026-11-24):** the current line, and under it the next line, dimmed. During the intro or a
  gap, the first row previews the upcoming line and the second row shows the one after it (`LyricFrame.followingLine`).
  A Romaji/subtitle row sits between them when present.
- **Geometry:** Extends cleanly downward from `292×46` (`r = 23`) to `292×88` (two rows, `PILL_LYRICS_H` 42) or `292×102` (with Romaji/subtitle, `PILL_LYRICS_SUB_H` 56) while synced lyrics are available and `show_on_pill` is enabled. Collapses seamlessly back to `292×46` during instrumental intros/outros or when no synced lyrics match.
- **Line change = a scroll, not a swap (2026-11-24, pill and card):** every visible line is one element keyed by the
  line (`lyricRows` in `lib/notch/lyrics.ts`) placed in a slot (`ladderY`). When the song moves on, the next line's
  element glides up into the main row and grows and brightens (`scale 0.875 → 1`), the old line glides up and fades
  out, and only the newly revealed line fades in from below. Its Romaji/subtitle fades in as it reaches the main row.
  460 ms ease-out, compositor-only `transform`/`opacity`, run only at line changes (no loop); instant with
  `systemReduceMotion` or preset `off`. The card keeps the line before the previous one just above view so its top
  line scrolls out instead of popping.

### 2.2 Expanded Card + Bottom Extension (`expanded` mode)

```text
╭─────────────────────────────────────────────────────────────────────╮
│  [58×58]  Idol                                       |||||   │  📌  │
│   Art     YOASOBI · Atmos                                    │  🎙  │
│           ⏮  ⏯  ⏭                                  💬        │  🎵  │ ← Lyrics toggle
│  1:12 ━━━━━━━━━━●──────────────────── 3:33                   │  ▶   │
│  🔊   ━━━━━━━━━━━━━━●────────────────  42%                   │      │
│  [🎧 Arctis Nova 7]  [🔈 Speakers]                           │  ⚙   │
├─────────────────────────────────────────────────────────────────────┤
│           誰かを愛することなんて                              │  ← dimmed previous line
│     無敵の笑顔で荒らすメディア                                │  ← bright active line + word effect
│     muteki no egao de arasu media                            │  ← Romaji / paired subtitle
│           知りたいその秘密ミステリアス                        │  ← dimmed next line (click to seek)
╰─────────────────────────────────────────────────────────────────────╯
```

- **Controls stay reachable:** Media controls, interactive progress scrubber, volume bar, and D-18 favourite devices remain completely untouched in the upper `164 px` block.
- **Lyrics toggle at the right end of the transport row** (`ExpandedMediaCard`, moved from `NotchSideRail` on 2026-11-24:
  a sixth rail button overflowed the 140 px rail and pushed Settings off the bottom). It toggles the bottom extension
  on/off and is dimmed with the tooltip "No lyrics for this song" when unavailable.
- **3-line focus band (D-23 V6):** Shows previous line (muted/dimmed), active line (bright, slightly scaled/weighted, plus Romaji/subtitle underneath), and next line (muted/dimmed).
- **Click any synced line to seek:** Clicking any visible lyric line sends `MediaAction::Seek { position_ms: line.start_ms }` directly through our GSMTC seek pipeline.
- **Plain lyrics fallback:** When only `plainLyrics` (unsynced) are available, the expanded extension displays a gentle scrollable plain-text pane (the compact music pill stays unextended so it never shows static wrong lines).
- **Canvas twin budget:** Bump `CANVAS_H` (`app/ui/src/lib/notch/geometry.ts`) and `NOTCH_CANVAS_H` (`app/src-tauri/src/main.rs`, `// Twin:`) from `210 px` to `292 px` so an expanded card + lyrics extension (`~248 px` + `6 px` top pad) still preserves ≥ 24 px margin around `--ice-drop-shadow`.

### 2.3 Active-Word Visual Effect (Karaoke Emphasis)

Each word in the active lyric line receives a crisp, readable emphasis without causing horizontal layout jitter (compositor-only `transform`, `opacity`, `filter`, and `color` — never animating `font-size` layout widths directly):

| Word state | Visual treatment |
|---|---|
| **Upcoming** | `--ice-lyric-muted` (~55 % opacity), `transform: scale(1)` |
| **Active (current word)** | `--ice-lyric-active` (100 % `--ice-fg` blended subtly with `--ice-active-accent`), `transform: translateY(-1px) scale(1.07)` with soft `text-shadow` / glow (`--ice-lyric-glow`) |
| **Sung** | Settled crisp `--ice-fg` (~86 % opacity), `transform: scale(1)` |

Two timing tiers power word highlights automatically:
1. **True word-level sync (Enhanced LRC `<mm:ss.xx>word`):** When a local `.lrc` file or remote record contains Enhanced LRC inline word timestamps, each word activates at its exact millisecond.
2. **Weighted phrase cadence (standard line-synced LRC):** When only line timestamps `[mm:ss.xx]` exist (the common case on LRCLIB), the pure helper in `ice-core` / UI distributes `[line.start_ms, next_line.start_ms]` across tokens weighted by visual/syllable length (splitting Latin by words and Japanese/CJK cleanly by character/clause clusters). Controlled by `lyrics.word_highlight` (on by default; degrades cleanly to whole-line highlight when off or under `systemReduceMotion` / preset `off`).

### 2.4 Theme Tokens (`--ice-lyric-*`)

Added to `app/ui/src/themes/base.css` and documented in [`docs/theming.md`](theming.md) for all 5 bundled themes (`ice`, `ice_light`, `midnight`, `fluent`, `minimal`):

| Token | Default (`ice`) | `ice_light` | Purpose |
|---|---|---|---|
| `--ice-lyric-active` | `#ffffff` | `#0b1220` | Active word / active lyric foreground |
| `--ice-lyric-muted` | `rgba(237, 241, 247, 0.48)` | `rgba(17, 22, 31, 0.45)` | Upcoming words and neighbour lines |
| `--ice-lyric-sub` | `rgba(237, 241, 247, 0.68)` | `rgba(17, 22, 31, 0.66)` | Romaji / paired translation subtitle |
| `--ice-lyric-glow` | `color-mix(in srgb, var(--ice-active-accent, var(--ice-accent)) 42%, transparent)` | `color-mix(in srgb, var(--ice-active-accent, var(--ice-accent)) 28%, transparent)` | Subtle halo behind active word (`transparent` in `minimal`) |

---

## 3. Lyrics Providers (Options A & B Implemented; Option C Documented)

### Option A — Opt-in LRCLIB (`https://lrclib.net`) *(implemented)*

- **Gate:** Disabled by default (`lyrics.online_lookup = false`). Requires explicit opt-in in `Settings → Privacy`.
- **Host allowlist:** Hardcoded `https://lrclib.net` only, called exclusively from Rust (`crates/ice-win`).
- **Lookup pipeline:**
  1. **Exact lookup:** `GET /api/get?track_name=…&artist_name=…&album_name=…&duration=…` (duration in whole seconds).
  2. **Fallback search (on 404):** `GET /api/search?track_name=…&artist_name=…` after normalizing common streaming suffixes (e.g. `- Remastered 2011`, `(feat. …)`, `[Official Video]`). Pick the highest-confidence match whose duration is within `±2 s` first, relaxing to `±5 s`; prefer `"no match"` over a mismatched track/timing.
  3. **Etiquette & resilience:** Send identifying `User-Agent: IceAudioManager/<ver> (+https://github.com/...)`, respect `Retry-After` on HTTP `429`/`503`, bound requests with a short timeout (e.g. `4 s`), and bind every in-flight lookup to a monotonic `TrackKey` generation so late responses after skipping tracks are dropped.
- **Data sent:** Only `title`, `artist`, `album`, and `duration` from GSMTC. Never sends IP trackers, machine IDs, or listening history beyond the single lookup, and never logs track titles above `debug`.

### Option B — Local `.lrc` Overrides & Managed Disk Cache *(implemented)*

Works **100 % offline** and always runs **before** Option A:

1. **User local `.lrc` folder (overrides):**
   - Path: `%LOCALAPPDATA%\IceAudioManager\lyrics\custom\`
   - Match order (case-insensitive, Windows-safe sanitized filename):
     1. `<Artist> - <Title>.lrc`
     2. `<Title>.lrc`
   - Supports standard `.lrc`, Enhanced word-synced `.lrc`, dual-timestamp paired subtitles, and `[offset:±ms]`.
   - Never evicted by cache limits (user-owned files).
2. **Managed disk cache (for Option A results):**
   - Path: `%LOCALAPPDATA%\IceAudioManager\lyrics\cache\`
   - Keyed by stable content hash of normalized `(artist, title, album, duration_bucket)`.
   - Stores parsed/raw `.lrc` + metadata JSON (`last_accessed_ms`, `source`, `size_bytes`), plus a lightweight negative cache (`not_found`, TTL 24 h) so tracks absent from LRCLIB aren't re-requested repeatedly.
   - Bounded by user-configured `lyrics.cache_max_mb` with **LRU eviction** (see §5).

### Option C — Future Possibility & Constraints *(Not Implemented Now)*

Unofficial provider integrations (e.g., Spotify's private color-lyrics API, YouTube Music InnerTube lyrics, Musixmatch / NetEase / KuGou / word-sync aggregators) remain **out of scope for now** due to hard project constraints (D-22 `K2`/`K3`, §1.5, §1.8):

- **Authentication & account risk:** Spotify and YouTube Music's private lyrics endpoints require user session cookies (`sp_dc`, browser auth) or reverse-engineered client keys, which conflicts with our zero-credential / zero-account guarantee and risks user account rate-limits or bans.
- **Maintenance fragility:** Undocumented third-party endpoints change without notice and often require rotating tokens or scraping.
- **Licensing & TOS:** Commercial providers (Musixmatch, LyricFind) restrict redistribution and require paid commercial API keys; GPL reference aggregators cannot be copied into our MIT codebase.

**Future-proofing seam:** We still define a clean `LyricsProvider` trait in `ice-core` (`fetch(&TrackQuery) -> Result<Option<LyricsDocument>>`) returning a single canonical `LyricsDocument` (`SyncKind::Word | Line | Plain`). If an official open word-synced database, licensed opt-in provider, or user-configured plugin is ever approved in a future decision, it can implement `LyricsProvider` without touching the cache, parser, IPC contract, or Svelte UI.

---

## 4. Romaji & Subtitles (Japanese / CJK Songs)

### 4.1 Dual-Timestamp Paired Subtitles (Zero Extra API)

Many community `.lrc` files (both on LRCLIB and local Option B files) include two lines at the exact same timestamp (e.g. Japanese + English translation, or Japanese + Romaji):

```lrc
[00:18.42]無敵の笑顔で荒らすメディア
[00:18.42]Muteki no egao de arasu media
```

The LRC parser automatically groups same-timestamp pairs (within `≤ 20 ms`) into `{ text, subtitle }`, rendering the second line cleanly as `--ice-lyric-sub` underneath the primary line.

**Paired lines vs generated Romaji (2026-11-24).** `LyricLine.subtitle` is only ever the source's own paired line, and
it always shows. Romaji generated on this PC lives in `LyricLine.romaji`, shows only while the Romaji setting is on,
and is **never written to the disk cache** (`cache::write_cache` strips it; cache records carry `format: 2`, and
older records, which mixed generated Romaji into `subtitle`, are dropped and looked up again). Before this, turning
Romaji off also hid paired translations, and Romaji generator fixes never reached already-cached songs.

### 4.2 Offline Japanese → Romaji Subtitles

When `lyrics.romaji` is enabled and a lyric line contains Hiragana, Katakana, or Kanji **and** does not already have a paired subtitle:

> **Corrected 2026-11-24 (see §9):** the IME path below never worked as shipped. The backends are now, in order:
> `Windows.Globalization.JapanesePhoneticAnalyzer`, then `IFELanguage`, then kana only, all on an **STA** `ice-lyrics`
> thread, and **both Windows backends need the Japanese language installed in Windows**.

1. **Primary (0 MB binary overhead): Windows MS-IME (`IFELanguage::GetJMorphResult`) in `crates/ice-win`**
   - Calls Windows' built-in Japanese morphological analyzer via COM on the dedicated MTA worker thread (strictly wrapped in RAII + `// SAFETY:`).
   - Produces accurate context-aware Kana readings and natural word/clause boundaries (`むてき の えがお で あらす めでぃあ`), which a tiny pure-Rust Hepburn table in `ice-core` converts to clean Romaji (`muteki no egao de arasu media`).
   - **Kana-only lines go through it too** (2026-11-24), for its word breaks: without them a kana line came out as one
     unspaced word (`kiminonamaewa`).
   - **Particles read as spoken** (`segmented_kana_to_romaji`, 2026-11-24): a word that is only は or へ becomes
     **wa** / **e** (`watashi wa`, not `watashi ha`); を stays `wo`. Only possible on segmented kana.
2. **Automatic Fallback (works on every Windows PC): Pure-Rust MIT transliterator**
   - *Planned, not built.* When the Windows Japanese language pack / IME is not installed, falls back transparently to a lightweight MIT-compatible pure-Rust crate (`kakasi` or `wana_kana` + compact Kanji reading map, ~1–2 MB) so Japanese songs still show readable Romaji out of the box.
   - **As built:** without the IME, kana converts directly with the Hepburn table (unspaced) and **lines with kanji get
     no Romaji**. See §7 item 5.
3. **When it runs:** once per song load, after the lyrics are found (local, cache or LRCLIB), never per frame. It is
   not cached (see §4.1), so it re-runs on each load of a song; one IME pass per line is cheap.
4. **Romaji follows the singing** (2026-11-24): under the current line, generated Romaji words light up in step with
   the karaoke highlight (`romajiLitCount`: the sung share of the line's words mapped onto the Romaji's words, since the
   two word splits differ). Paired subtitles (often translations) stay static.
4. **Optional future upgrade (Translation & High-Accuracy Dictionaries):**
   - Full Japanese → English machine translation requires either large local models (300 MB–2 GB+, outside our lightweight footprint) or a separate opt-in translation endpoint; this stays a separate future Privacy decision.
   - Similarly, a full morphological IPADIC/UniDic dictionary (`lindera`, ~15–30 MB) can be offered later as an optional on-demand download managed inside `Settings → Storage`.

---

## 5. Settings — Privacy & New Storage Section (Max Cache Size)

### 5.1 Config Schema (`SCHEMA_VERSION` `v8 → v9`)

Add a `[lyrics]` section to `crates/ice-core/src/config.rs` (migrated automatically from `v8` with all-safe defaults + fixture test):

```toml
[lyrics]
enabled = false          # Master lyrics display toggle
online_lookup = false    # Option A (LRCLIB); explicitly opt-in on Privacy page
show_on_pill = true      # Bottom extension on compact music pill
show_on_card = true      # Bottom extension on expanded card
word_highlight = true    # Active-word karaoke emphasis
romaji = true            # Offline Japanese -> Romaji subtitle line
cache_max_mb = 50        # Managed disk cache ceiling (5..=500 MB)
```

Settings commands (`Command::is_setting() == true`):
- `SetLyricsEnabled { enabled: bool }`
- `SetLyricsOnlineLookup { enabled: bool }`
- `SetLyricsDisplay { show_on_pill: Option<bool>, show_on_card: Option<bool>, word_highlight: Option<bool>, romaji: Option<bool> }`
- `SetLyricsCacheMaxMb { max_mb: u16 }` (validated `5..=500`; triggers immediate LRU trim on shrink)

### 5.2 `Settings → Privacy` Page (D-22)

- **Master kill switch:** `"Keep all network features off"` (disables `online_lookup` and any future network features in one click).
- **Transparent network table:** Explicitly lists:
  - Feature: **Synced lyrics lookup**
  - Destination: `https://lrclib.net`
  - Data sent: Track title, artist, album, and duration only
  - Control: Opt-in toggle (with a prompt/option to clear cached lyrics when turned off).

### 5.3 `Settings → Storage` Section (New)

Added as a dedicated **Storage** page in the Settings navigation rail (or Storage section under General/Storage, using existing `SettingCard`, `Select`, `Row`, and `InfoBar` controls with `--ice-set-*` tokens):

- **Usage readout (queried lazily on page open — zero polling):**
  - `"Lyrics cache: 1.8 MB of 50 MB · 142 cached tracks"` (with a clean slim usage bar in `--ice-set-accent`).
  - `"Local .lrc overrides: 4 files"` (`%LOCALAPPDATA%\IceAudioManager\lyrics\custom`).
- **Max cache size (`cache_max_mb`):**
  - Presets in native `<Select>`: `10 MB`, `25 MB`, `50 MB (Default)`, `100 MB`, `250 MB`, `500 MB`.
  - Lowering the limit immediately runs an LRU eviction sweep and updates the live usage readout.
- **Storage actions:**
  - **`Clear lyrics cache`** — deletes `%LOCALAPPDATA%\IceAudioManager\lyrics\cache\*` (never touches `custom\`), refreshes usage readout, flashes `"Cleared"`.
  - **`Open local lyrics folder`** — ensures `%LOCALAPPDATA%\IceAudioManager\lyrics\custom\` exists and opens it in Explorer so users can drop `.lrc` files easily.
- **LRU eviction invariants:**
  - Max single entry size: `256 KB` (guards against malformed/oversized responses or `.lrc` files).
  - Atomic writes (`temp file → rename`) so partial writes never corrupt cache files.
  - On cache hit, touch entry timestamp; on write or limit decrease, evict oldest-accessed entries until `total_bytes <= cache_max_mb * 1024 * 1024`.

New least-privilege Tauri commands (Settings window only, `permissions/shell.toml` + `capabilities/settings.json`):
- `get_lyrics_storage_stats` → `{ cacheBytes, cacheFiles, customFiles, cacheMaxMb, customDir }`
- `clear_lyrics_cache` → `LyricsStorageStats`
- `open_lyrics_folder` → `()`

---

## 6. Recommended Implementation Path

Build in **five clean, reviewable vertical slices** (each with unit tests, `git diff --check`, and same-PR `docs/modules/` updates):

### Slice 1 — `ice-core` Domain, Parser & Schema v9
1. Add `crates/ice-core/src/lyrics.rs`:
   - `LyricsDocument`, `LyricLine`, `LyricWord`, `LyricsSource` (`Local`, `Lrclib`, `Cache`), `LyricsStatus` (`Idle`, `Loading`, `Ready`, `NotFound`, `Disabled`).
   - Pure LRC + Enhanced LRC (`<mm:ss.xx>`) + `[offset:]` + dual-timestamp paired subtitle parser.
   - Pure weighted word-cadence estimator (Latin word + Japanese/CJK cluster splitting) and pure Kana → Hepburn Romaji table.
   - `LyricsConfig` + `SCHEMA_VERSION` bump `8 → 9` in `crates/ice-core/src/config.rs` with migration fixture tests (`v8 → v9`) and `cache_max_mb` validation (`5..=500`).
   - `EngineEvent::LyricsChanged`, `EngineSnapshot.lyrics`, and settings `Command` variants (`ts-rs` export to `app/ui/src/lib/ipc/engine.ts`).
   - Add sample English + Japanese synced lyrics to `FakeEngine` so the full UI can be developed and QA'd offline via `--fake-engine`.

### Slice 2 — `ice-win` Local `.lrc`, LRU Cache, LRCLIB Client & Offline Romaji
1. **Option B (`crates/ice-win/src/lyrics/cache.rs`):**
   - Local `.lrc` override loader (`%LOCALAPPDATA%\IceAudioManager\lyrics\custom\`).
   - Managed LRU disk cache (`%LOCALAPPDATA%\IceAudioManager\lyrics\cache\`) with atomic writes, `256 KB` entry ceiling, negative cache TTL, `cache_max_mb` enforcement, and `storage_stats()` / `clear_cache()`.
2. **Offline Romaji (`crates/ice-win/src/lyrics/romaji.rs`):**
   - Japanese script detection + `IFELanguage::GetJMorphResult` COM wrapper (RAII, `// SAFETY:`, MTA thread) with automatic fallback to pure-Rust transliterator.
3. **Option A (`crates/ice-win/src/lyrics/lrclib.rs`):**
   - Clean-room HTTPS client against `https://lrclib.net/api/get` + `/api/search` (title normalization, ±2 s → ±5 s duration matching, `User-Agent`, `429`/`503` `Retry-After`, timeout, `TrackKey` cancellation).
4. **Worker wiring (`crates/ice-win/src/lyrics/service.rs`):**
   - Runs off `ice-audio` (never delays switching), listens to track changes, checks `custom` → `cache` → (if `online_lookup`) `lrclib`, computes Romaji once, caches result, and emits `LyricsChanged`.

### Slice 3 — `app/src-tauri` Shell & Capabilities
1. Forward `LyricsChanged` through `engine_bridge`, expose `get_lyrics_storage_stats`, `clear_lyrics_cache`, and `open_lyrics_folder` restricted to the Settings window in `app/src-tauri/permissions/shell.toml` and `capabilities/settings.json`.
2. Update `NOTCH_CANVAS_H` in `app/src-tauri/src/main.rs` (`// Twin:` with `geometry.ts`).

### Slice 4 — `app/ui` Settings (`Privacy` & `Storage`) + Notch Bottom Extension
1. **Settings UI:**
   - Add `PrivacyPage.svelte` (network allowlist transparency + master off switch + LRCLIB opt-in).
   - Add `StoragePage.svelte` (live cache usage bar, `cache_max_mb` selector, `Clear lyrics cache`, `Open local lyrics folder`).
   - Add lyrics display toggles (`Show on pill`, `Show on expanded card`, `Word highlight`, `Japanese Romaji`) on `ActivitiesPage.svelte` (or `NotchPage.svelte`).
2. **Notch Bottom Extension UI:**
   - Add `app/ui/src/lib/notch/lyrics.ts` (+ `lyrics.test.ts`): active line index, active word index, smooth next-transition timeout calculation (wakes only on the next word/line boundary while visible & playing — zero 60 Hz polling!).
   - Add `app/ui/src/notch/activities/LyricsExtension.svelte`: compact pill 1-line+subtitle mode and expanded 3-line focus band with active-word lift/glow and click-to-seek (`MediaAction::Seek`).
   - Wire into `NotchApp.svelte`, `TrackActivity.svelte`, `ExpandedNotch.svelte`, `NotchSideRail.svelte`, `geometry.ts`, and `base.css` (`--ice-lyric-*` tokens).

### Slice 5 — Verification & Docs
1. Run full `build check` (`tsc`, Node unit tests, Vite build, `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`).
2. Manual QA with Spotify (English + Japanese tracks, pause/resume/seek/skip, local `.lrc` override, cache eviction at `10 MB`, `Privacy` off = zero network requests, light & dark modes, 100 % & 150 % DPI, `systemReduceMotion`).
3. Update `docs/theming.md` and affected `docs/modules/*.md` docs in the same PR.

---

## 7. Review (2026-11-24): improvements found, not built yet

From a pass over this plan and the implementation. Built in the same pass: the scrolling line change, the two-row pill,
separate `romaji`/`subtitle` fields with a cache format bump, IME word breaks for kana-only lines, particles as
spoken, and the Romaji sweep (above).

| # | Improvement | Why | Size |
|---|---|---|---|
| 1 | **Timing nudge per song** (e.g. −/+ 250 ms in the card, saved with the cache entry) | LRCLIB line timings are often a little early or late; today the only fix is editing an `.lrc` by hand | Medium |
| 2 | **Gap countdown**: in an instrumental gap over ~4 s, three dots that fill toward the next line instead of a static "···" | Tells the listener the lyrics haven't stalled; the wake timer already knows when the next line starts | Small |
| 3 | **Loading state**: a soft shimmer on the drawer while an LRCLIB lookup is in flight | §2.2 promises it; the UI shows nothing until the lookup ends | Small |
| 4 | ~~**Long lines**: shrink-to-fit or a marquee instead of an ellipsis~~ **Done 2026-11-24** with the §8 fix: shrink to 82 %, then slide with the singing | Many J-pop lines are wider than the 292 px pill, so their end was never visible | — |
| 5 | **Kanji Romaji without the IME** (the §4.2 fallback as planned) | PCs without the Japanese language pack show no Romaji on most Japanese lines | Medium (+1–2 MB; a crate choice under §1.8) |
| 6 | **Romaji long vowels**: an option for macrons (`tōkyō`) or doubling (`toukyou`, today) | Readers prefer one or the other | Small (a setting) |
| 7 | **Translation and Romaji together** when a song has both (three rows in the card) | Today a paired translation wins and generated Romaji isn't shown for that line | Small (card only) |
| 8 | **Fake engine parity**: `--fake-engine` still strips paired subtitles when Romaji is off | It now differs from the real engine, where paired lines always show | Tiny |

---

## 8. Fixed (2026-11-24): the last characters of a line were cut off

**Report (owner):** in the music pill, lyric lines sometimes lost the last characters of their last word.

**Cause (two problems):**
1. **Ellipsis can't apply to the karaoke line.** With word highlight on, the main line is a row of `inline-block` word
   spans (so each word can lift and scale). `text-overflow: ellipsis` only works on inline text, so a line wider than
   the row was cut mid-character at the edge, with no "…". It hit the pill most (292 px wide, 12 px text, often
   long Japanese lines) and only the highlighted line, which is why it looked like "sometimes".
2. **The active word grows past the edge.** The sung word scales to 1.07 with a glow. On the last word of a line that
   just fits, those extra pixels were cut by `overflow: hidden`, and the 1 px lift could be shaved at the top.

**Fix** (`app/ui/src/lib/notch/fitLine.ts`, `LyricsExtension.svelte`):
- The main line **shrinks to fit**, down to 82 % (`FIT_MIN_SCALE`), keeping 4 px free at the edge for the lift
  (`FIT_EDGE_PX`). The card re-centres a shrunk line.
- A line still too long **slides with the singing**: the sung word's end is kept at 70 % of the row, so the last word
  is fully visible by the time it's sung. Soft 16 px fades mark the cut edges.
- It re-measures only when the line or the sung word changes (no loop), plus once when fonts finish loading.
  `transform` only; instant with reduced motion.
- The ladder uses `overflow: clip` with a 2 px clip margin, so the lift and glow aren't shaved vertically.
- Other rows (next, previous) are plain text and keep their ellipsis.
- Tested in `fitLine.test.ts`: fits, shrinks, slides to the very end, centres in the card.

---

## 9. Fixed (2026-11-24): Romaji never appeared on kanji lines

**Report (owner):** no Romaji at all.

**What I measured on the owner's PC** (`cargo test -p ice-win romaji_on_this_pc -- --ignored --nocapture`, plus
one-off diagnostics):

| Path | Result | Why |
|---|---|---|
| `IFELanguage` from the `ice-lyrics` thread (as shipped) | init fails: `QueryInterface` → `E_NOINTERFACE` | `MSIME.Japan` is `ThreadingModel = Apartment` (STA). From an MTA thread COM hands back a proxy, and `IFELanguage` has no proxy/stub, so the interface can't be reached |
| `IFELanguage` from an STA thread (`MSIME.Japan`, `.FELang`, `.FELang.15`) | `GetJMorphResult` → `S_FALSE`, no result, in every mode | The new Microsoft IME (Windows 10 2004+) doesn't do reverse conversion, and this PC has no Japanese dictionary (`System32\IME\IMEJP\DICTS` is missing) |
| `JapanesePhoneticAnalyzer` from an MTA thread | refuses to activate (`0x8000001D`) | Also an STA class |
| `JapanesePhoneticAnalyzer` from an STA thread | 0 words for every line | Only English (en-US) is installed in Windows, so there's no Japanese language data |

So **kanji lines never had Romaji on this PC**, before or after the 2026-11-24 Romaji changes. Only kana-only lines did,
through the built-in kana table (unspaced). The old cache confirms it: records written before `format: 2` had Romaji
on 1 of 34 and 4 of 48 Japanese lines.

**Fix** (`crates/ice-win/src/lyrics/romaji.rs`, `lyrics.rs`):
- The `ice-lyrics` thread is now an **STA**. No message pump is needed: every COM call there is synchronous on that
  thread, and WinHTTP and file I/O aren't COM.
- **Backends in order:** `JapanesePhoneticAnalyzer` (word readings and boundaries; the supported modern API), then
  `IFELanguage` (kept for PCs where it still answers, e.g. the "previous version" IME; mode flags corrected to
  `HIRAGANAOUT | AUTOMATIC | NOINVISIBLECHAR`), then kana only. Each is probed once at start, so a dead backend
  costs nothing per line; the one in use is logged (`Romaji backend`).
- **Settings tells you:** `LyricsStorageStats.romajiReadsKanji` (probed on a short-lived STA thread only when the
  Settings window asks). Activities → Synced lyrics shows "Romaji only covers kana lines on this PC" with how to add
  Japanese, when Windows can't read kanji.

**What the owner needs to do to get kanji Romaji:** add **Japanese** in Windows Settings → Time & language → Language &
region (the language features, no need to type with it). *Expected* to make `JapanesePhoneticAnalyzer` return
readings; not yet verified on this PC, since installing a language changes the system. Then reopen Settings →
Activities: the hint disappears when it works.

**If installing Japanese isn't acceptable:** kanji Romaji needs a reading dictionary shipped with the app (§7 item 5).
Every option is a trade-off for a decision entry: size (a full tokenizer dictionary is ~15–50 MB against the < 10 MB
installer budget), licence (KANJIDIC/JMdict are CC BY-SA; the `kakasi` crate is GPL, see D-24), or an online service
(network, §1.5).
