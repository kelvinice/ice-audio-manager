# D-21 · Gemini's vote

> Re-cast on 2026-11-23 under blind voting (§0.4) after re-checking `app/src-tauri/src/main.rs` (`request_idle_suspend`, `spawn_cursor_poll`), `app/src-tauri/src/placement.rs`, `docs/spikes/phase0-overlay.md`, `docs/qa/manual-checklist.md`, and `NotchApp.svelte`. *(Replaces the pre-§0.4 anchored vote: Part A lowered `+2` → `+1`; Part B flipped to **`C0 +2, C1 +1, C2 −2`** based on our own measured `0.059%` / `77–80.5 MB` data.)*

## Scores

#### Part A *proposal* (opt-in `Rich` profile, items 1–4)

| Option | Score | One-line reason |
|---|---|---|
| Part A (opt-in `Rich`, items 1–4) | +1 | Good opt-in choice once hit-testing is event-driven, but narrows in value once D-19 fixes the native pill and carries an unspiked WebView2 hit-testing risk |

#### Part B *pick one* (resource caps)

| Option | Score | One-line reason |
|---|---|---|
| C0 · Keep every cap (`≤ 100 MB` WS, `≤ 200 MB` PB, `< 0.2%` idle CPU) | +2 | Our own measured warm-WebView numbers (`77–80.5 MB` WS, `150 MB` PB, `0.059%` CPU) already fit C0, honour Q-06, and catch static `rAF`/poll leaks that `< 0.5%` would hide |
| C1 · Per-profile caps (`Rich`: `≤ 120 MB` WS, `≤ 250 MB` PB, `< 0.5%` idle CPU) | +1 | Acceptable fallback only if a multi-hour soak spike proves WebView2 cache drifts past `100 MB` WS; static idle CPU should still stay `< 0.2%`, not `< 0.5%` |
| C2 · Raise caps for every profile | −2 | **Block:** violates Guiding Constraint 1 and the owner's Q-06 line (`> 100 MB` idle is too heavy) for default users |

## Strongest argument against my top choice

- **Against C0 (keeping `≤ 100 MB` WS / `≤ 200 MB` PB / `< 0.2%` idle CPU for `Rich` too):** Our `77.10 MB` (`phase0-overlay.md:35`) and `80.45 MB` (`manual-checklist.md:186`) Private WS numbers came from 60-second runs on a fresh WebView, leaving only ~20 MB of headroom below `100 MB`. Over an 8-hour session with dozens of `iceart://` album-art decodes, repeated hover expansions, and Chromium compositor tile caching, Private WS could creep past `100 MB` and make C0 flaky in long-soak tests. In addition, if a user enables opt-in live visualizer bars in `Rich`, active bar animation (`0.21%–0.44%` CPU measured) exceeds `< 0.2%`, so C0 requires explicitly classifying live bars under the **active-playback `< 1%`** budget rather than the silent-idle budget.

## New option or amendment

1. **Part A alternative/amendment — *"Keep awake while music plays"* (media-aware linger) + profile simplification:** Check `placement.rs:20-22`: after D-15.1, `Instant` and `Balanced` both `TrySuspend` after `15 s` (`scheduler.ts:22`). Rather than maintaining four overlapping profiles (`Instant`, `Balanced`, `Lite`, `Rich`), consider keeping 3 clear modes (`Lite`, `Balanced`/`Instant`, `Rich`) or adding a **"Keep notch awake while playing"** option that holds the WebView mini pill open with album art, static art glow, and optional live bars *only while GSMTC reports active playback*, then `TrySuspend`s back to `~6 MB` WS during silent work hours. (Also note: pinning the expanded notch already prevents suspend today via `main.rs:280`.)
2. **Part B amendment (if C1 ever triggers after a long soak):** Even if RAM headroom for `Rich` needs `120 MB` WS after soak testing, **keep the static (non-animating) idle CPU gate at `< 0.2%`**, never `< 0.5%`. At `< 0.5%`, a leaked `rAF` loop or forgotten `spawn_cursor_poll` (`~0.3%–0.4%`) would silently pass CI.

## Evidence I checked myself

- `docs/spikes/phase0-overlay.md:35-38` + `app/src-tauri/src/main.rs:518-578, 1328`: `3_notch_visible_warm` measured **`77.10 MB` Private WS, `149.84 MB` Private Bytes, and `0.059%` avg CPU** over 60 s — *even while* benchmark mode ran `spawn_cursor_poll` at `16 ms` (`62.5 Hz`, `main.rs:577, 1328`). `5_notch_try_suspended` measured `6.44 MB` WS / `0.009%` CPU.
- `docs/qa/manual-checklist.md:186`: Phase 2 60 s release run (`bench-p2.json`) measured `3_notch_visible_warm` = **`80.45 MB` Private WS**, `5_notch_try_suspended` = **`6.00 MB` Private WS / `0.004%` CPU**, `TrySuspend` resume = `21.89 ms` p50 / `28.49 ms` p95.
- `docs/qa/manual-checklist.md:193, 199`: Phase 3 CDP and live `--fake-engine` WebView2 E2E traces measured compact `VisualizerBars` at **`0.435%` whole-machine CPU** (`239.8 µs` avg `rAF`, `0 ms` layout) in CDP and **`~0.21%` whole-machine CPU** (`2.522%` single-core `TaskDuration`) in live WebView2 (plus `0.013%` for `ice-meter`), settling to **`0` `rAF` calls and `0` active intervals** on silence, pause, suspend, or `Off`.
- `app/ui/src/notch/NotchApp.svelte:153-168`: `.accent-glow` is a static composited CSS `radial-gradient` driven by `--ice-active-glow` (`opacity: 0.28`), with only a `400 ms` opacity transition on track change (`transition: none` when `instant`).
- `app/src-tauri/src/main.rs:276-285` & `app/src-tauri/src/placement.rs:20-22`: confirms `expanded` already suppresses `request_idle_suspend`, and `Instant`/`Balanced` share the same `allows_webview()` policy.
- `docs/planning-discussion.md:1709` (Q-06): owner explicitly set `> 100 MB` idle as *"too heavy"*.

## Position

**WebView idle CPU when nothing animates:** Our own measurements prove that a warm, static WebView2 notch is **almost free on CPU** (`0.059%` avg over 60 s even with the `16 ms` benchmark cursor poll running; `0` `rAF` calls and `0` intervals in `SpringGroup`/`VisualizerController` once settled). Keeping `Rich` warm costs **RAM (`~71–74 MB` extra Private WS over `TrySuspend`)**, not continuous CPU—*strictly provided* Part A item 4 replaces `spawn_cursor_poll` (`main.rs:518-578`) with event-driven hit-testing (which needs an early spike because WebView2 child `Chrome_WidgetWin_*` HWNDs complicate `WM_NCHITTEST`).

**What `Rich` should add at idle (Gemini's design/perf call):**
- **Base `Rich` idle + static art-accent glow (on in `Rich`):** Keeps `IdlePill.svelte` awake (`Segoe UI Variable`, 1 px SVG rim, `AlbumArtTile`, `0 ms` hover-intent). Static `.accent-glow` (`NotchApp.svelte:153-164`) should be allowed whenever media is loaded because a static CSS radial gradient triggers zero `rAF` frames (`400 ms` crossfade only on track changes).
- **Live visualizer bars while music plays (opt-in sub-toggle under `Rich`, off by default):** Measured at `0.21%–0.44%` whole-machine CPU (`+0.013%` meter) while actively playing, auto-sleeping to `0` `rAF` on pause/silence. Great opt-in under the existing `< 1%` active-visualizer budget, not forced on base `Rich`.
- **Perpetual "animated idle" during silence (reject / do not build):** Continuous breathing dots, rotating borders, or looping shimmers during silence violate `code-practices.md` §2 (*"Hidden means idle"*), keep the GPU/CPU awake with zero informational signal, and add visual fatigue. All motion in `Rich` must stay event- or audio-driven and decay to `0` `rAF` at rest.

**Why I flipped to `C0 (+2)` over `C1 (+1)` (and `Part A +1`):** In my earlier anchored vote I echoed `C1 (+2)`, but re-checking our actual `77–80.5 MB WS / 150 MB PB / 0.059% CPU` measurements changed my mind. `Rich` already fits `C0` (`≤ 100 MB` WS, `≤ 200 MB` PB, `< 0.2%` static idle CPU) while respecting Q-06, and keeping the static idle CPU gate at `< 0.2%` prevents a broken `rAF` or poll leak from hiding under `C1`'s loose `< 0.5%` ceiling. Only consider `C1`'s `120 MB` WS cap if a multi-hour soak spike proves Chromium's cache cannot stay under `100 MB` WS. `C2` remains a **block (`−2`)** for default profiles.
