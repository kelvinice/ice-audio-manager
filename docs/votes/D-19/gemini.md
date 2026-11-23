# D-19 · Gemini's vote

> Re-cast on 2026-11-23 under blind voting (§0.4) after reviewing `native_dot.rs`, `crates/ice-win/Cargo.toml`, `IdlePill.svelte`, `geometry.ts`, and `docs/reference/screens/native-pill-muted-2026-11-23.png` independently. *(Replaces the pre-§0.4 anchored vote; Part B lowered from `+2` to `+1` over Direct2D/DirectWrite RAM risk against the `≤ 15 MB` suspended idle budget.)*

## Scores

#### Part A *pick many* (activation)

| Option | Score | One-line reason |
|---|---|---|
| A1 · Hit-testable pill | +2 | Event-driven Win32 messages (`0.000%` idle CPU) fix the dead-pill complaint directly |
| A2 · "Show notch" hotkey | +1 | Good opt-in keyboard path and the only direct trigger when `IdleStyle::Hidden` is chosen |
| A3 · Cursor-proximity poll while shown | −2 | **Block:** `code-practices.md` §2 limits cursor polling to "while near the notch" with a stop condition; running a poll across all idle hours breaks §2 |
| A4 · Top-edge hot strip | −1 | Intercepts Fitts'-law clicks on maximised browser tab strips and only fits top-centre placement |

#### Part B *proposal* (B1 one idle-content rule in Rust + B2 DirectWrite rendering, RAM-gated + B3 visual QA)

| Option | Score | One-line reason |
|---|---|---|
| Part B (B1–B3) | +1 | Strongly want B1 + B3, but B2 mandates Direct2D/DirectWrite without gating `Instant` suspended (`≤ 15 MB` WS) or testing lighter GDI glyph rendering first |

## Strongest argument against my top choice

- **Against A1 (hit-testable pill):** Looking at `docs/reference/screens/native-pill-muted-2026-11-23.png`, the top-centre pill sits directly over a maximised browser's top chrome/tab area. Dropping `WS_EX_TRANSPARENT` (`native_dot.rs:943`) makes that `92×18` (or WebView-matched `116×28`) strip solid to mouse input, so clicks on any tab or window control underneath are eaten. If hover-intent also triggers at the current `120 ms` (`controller.svelte.ts:362`), brushing past maximised tabs could cause unwanted expansions. Furthermore, `WM_MOUSEWHEEL` delivery to a `WS_EX_NOACTIVATE` window relies on Windows' *"Scroll inactive windows"* setting being on.

## New option or amendment

1. **Part A amendment (prevent accidental tab-bar wakes):** On the native idle pill, make **left-click** expand immediately (with 1-frame native press/hover feedback) and either make hover-to-expand opt-in or raise its idle-pill dwell to `~200 ms` with a subtle hover rim brighten first, so passing the cursor across browser tabs doesn't pop open the expanded notch.
2. **Part B amendment (B2-lite renderer + `Instant` RAM gate + non-empty resting pill):**
   - Spike **GDI `CreateFontW` (`Segoe UI Variable Text` / `Segoe Fluent Icons`) + `GetGlyphOutlineW(GGO_GRAY8_BITMAP)`** (or `DrawThemeTextEx` via already-loaded `uxtheme.dll`) into the existing 32-bpp BGRA DIB (`native_dot.rs:1180-1191`) **before** pulling `d2d1.dll` + `dwrite.dll` into `ice-win`.
   - Extend B2's RAM gate from Balanced-only (`< 30 MB`) to **all** idle profiles: `Instant` suspended must stay **`≤ 15 MB` Private WS** (currently `6.00–6.44 MB`), and `Lite` should stay **`< 10 MB` Private WS** (currently `1.64 MB`).
   - For B1's resting state (unmuted, no media): avoid an empty `116×28` blank dark bar over light desktops—either show a compact output-device glyph (headphones/speaker from `form_factor`) or shrink the resting pill width (`~52×20`) until muted/media expands it.

## Evidence I checked myself

- `docs/reference/screens/native-pill-muted-2026-11-23.png`: shows why the pill looks rough—squashed `92×18` proportion, unexplained blue dot, blocky SDF stroke letters (`MUTE`) in harsh `--ice-danger` orange over light browser chrome, and no drop shadow or speaker glyph.
- `crates/ice-win/src/native_dot.rs:38-41, 943, 1087-1107`: hard-coded `IDLE_W = 92, IDLE_H = 18`, `WS_EX_TRANSPARENT` set on creation, and `dot_wnd_proc` does not yet handle `WM_MOUSEACTIVATE` (needs `MA_NOACTIVATE` once `WS_EX_TRANSPARENT` is removed).
- `crates/ice-win/src/native_dot.rs:1265-1278` & `:490`: bug in current idle logic—`if let Some(v) = state.volume` shadows `else if state.mic_muted`, making the `"MIC OFF"` idle label unreachable whenever volume is populated; `EngineEvent::MediaChanged` is ignored completely (`:490`).
- `crates/ice-win/Cargo.toml:16-32`: `ice-win` currently links only GDI and DWM (no Direct2D or DirectWrite), which is why `2_native_dot_only` measured `2.20 MB` Private Bytes / `1.64 MB` Private WS and `5_notch_try_suspended` measured `6.44 MB` Private WS (`docs/spikes/phase0-overlay.md:34, 37`; `6.00 MB` in `docs/qa/manual-checklist.md:186`).
- `app/ui/src/lib/notch/geometry.ts:64-67` vs `native_dot.rs:38-39`: confirms the §11 twin (`116×28`/`196×28` WebView vs `92×18` native).
- `docs/code-practices.md:26-27`: §2 explicitly restricts cursor polling to *"while the cursor is near the notch, ≤ 30 Hz"* with a documented stop condition.

## Position

**Part A:** A1 (`+2`) and A2 (`+1`) are the right combination. Removing `WS_EX_TRANSPARENT` from the pill window and handling `WM_MOUSEACTIVATE → MA_NOACTIVATE`, `WM_MOUSEMOVE` + `TrackMouseEvent`, left/middle clicks, and `WM_MOUSEWHEEL` gives event-driven input at `0.000%` idle CPU without touching global hooks or polling. Immediate 1-frame native press feedback bridges both `TrySuspend` resume (`21.89–28.49 ms` measured in `manual-checklist.md:186`) and `Balanced` cold spawn (`715.44 ms` p95 in `phase0-overlay.md:66`). A3 is a **block (`−2`)** under §2 because the idle pill is shown most of the day.

**Part B (`+1`, amended):** I fully back B1 (one `ice_core::IdleContent` rule fixes both the §11 twin and the `native_dot.rs:1265-1278` shadowing bug) and B3 (side-by-side 100%/150% light/dark QA). However, I lowered Part B from `+2` to `+1` because mandating Direct2D + DirectWrite in B2 without checking `Instant` suspended RAM risks pushing our `6.00–6.44 MB` suspended footprint past its `≤ 15 MB` Private WS budget (an estimated `+8–15 MB` permanent host DLL/cache overhead). Let's benchmark GDI `GetGlyphOutlineW` / `DrawThemeTextEx` into our existing DIB first, or gate DirectWrite on keeping `Instant` suspended `≤ 15 MB` WS. For B1's visual hierarchy as design owner: mic muted = warning tone + slashed-mic glyph (privacy); speaker muted = muted-fg tone + speaker-off glyph (calm, not alarmist danger text); media playing = album accent dot or compact static bars; resting default = compact device glyph or slim resting capsule rather than a wide empty bar.
