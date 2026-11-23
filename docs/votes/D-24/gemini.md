# D-24 · Gemini's vote

> Cast 2026-11-23 blind (§0.4) from the D-24 entry and my own inspection of `LICENSE`, `Cargo.toml`, `app/ui/package.json`, `app/ui/src/lib/motion/spring.ts`, `crates/ice-win/src/native_dot/glyphs.rs`, `docs/reference/BitChord/`, and `docs/reference/Echo-Music/`. I have not opened `claude.md` or `codex.md` for D-24.

## Scores

| Option | Score | One-line reason |
|---|---|---|
| G0 · Stay MIT (clean-room only) | +2 | No UI, motion, rendering, or lyrics GPL code is actually reusable here (refs are Kotlin/Android or Swift/macOS); MIT keeps distribution and licensing trivial |
| G1 · Relicense to GPL-3.0-or-later now | −1 | Locks the repo into copyleft and Phase 6 corresponding-source/notice obligations before we have a single GPL file worth porting |
| G2 · Stay MIT now, relicense per component | +1 | Pragmatic fallback: keeps MIT today while leaving a structured bar (`named component + worse to write our own + no D-22/§1 conflict`) if a real Rust crate arises later |

## Strongest argument against my top choice

- **Against G0 (clean-room only under MIT):** Even when we write our own Rust/Svelte code cleanly, *reading* GPL-3.0 Android code (`Echo-Music`, `BitChord`) while implementing similar features (such as LRCLIB title/suffix sanitization rules or multi-line LRC fallback quirks) risks accidental similarity or subtle structural carry-over—whereas moving to `GPL-3.0` (`G1`) would eliminate any clean-room anxiety when inspecting `docs/reference/` repos (and the owner explicitly noted they are fine with GPL-3.0). Furthermore, if we ever hit a legitimately hard domain problem in Phase 5+ (e.g., an existing GPL Rust audio-DSP crate or desktop lyric renderer), `G0` refuses it outright while `G2` keeps the door open.

## New option or amendment

- **G0/G2 operational guardrail (if staying MIT):** Formalise a clean-room rule for `docs/reference/`: look at GPL-3.0 / AGPL-3.0 repos only for **screenshots and public HTTP protocol facts** (already practised in D-20/D-22/D-23); never inspect internal helper algorithms right before implementing an MIT equivalent (`LrcLib.kt`'s `MetadataCleaner` regexes, for example, must be authored independently from lrclib.net docs + test cases rather than translated from Kotlin).

## Evidence I checked myself

- **Motion (`app/ui/src/lib/motion/spring.ts:11-60`):** Our pure-TS analytical `(stiffness, damping, mass)` spring solver (`Snappy`, `Smooth`, `Bouncy`, `Off`) is 134 lines, zero external dependencies, tested across `75/144/240 Hz`, and drives our SVG `<rect>` + `clip-path` island directly. `boring.notch` (`planning-discussion.md:149`) is macOS Swift/AppKit and cannot compile in Svelte 5 / TypeScript.
- **Rendering (`crates/ice-win/src/native_dot/glyphs.rs`, `app/ui/src/themes/base.css`, `app/ui/src/notch/activities/`):** Our GDI `CreateFontW`/`ExtTextOutW` DIB renderer (`1.69 MB` Private WS in `Lite` idle) and Svelte 5 activity components (`ExpandedMediaCard.svelte`, `VisualizerBars.svelte`, `AlbumArtTile.svelte`) are custom Win32/WebView2 code styled with `--ice-*` tokens. Checking `docs/reference/Echo-Music` and `docs/reference/BitChord` directory trees confirms both are Kotlin/Jetpack Compose Android apps with zero Win32/GDI or Svelte/CSS rendering code.
- **Lyrics UI (`docs/reference/Echo-Music/lyrics/`, `docs/reference/BitChord/app/.../ui/player/`):** Both are Android Compose UIs (`LyricsShareCard.kt`, etc.) built for vertical phone screens. In Svelte 5, a synced LRC parser + scrollable active-line list driven by our existing `Spring1D` (`spring.ts`) and `--ice-*` opacity/font tokens is ~100–150 lines—writing it cleanly in TypeScript/Svelte is faster and less error-prone than porting Jetpack Compose modifiers from Kotlin.
- **Current repo metadata (`LICENSE:1-21`, `Cargo.toml:11-15`, `app/ui/package.json:12-20`):** Everything across the workspace (`ice-core`, `ice-win`, `ice-audio-manager`) is uniformly MIT with `@tauri-apps/api`, Svelte 5, Vite, and TypeScript (`devDependencies`), so staying MIT requires zero manifest, notice, or store compliance rework. (`docs/reference/Auralis` is **AGPL-3.0**, which moving to GPL-3.0 wouldn't unlock anyway.)

## Position

**Prompt check — is there any specific GPL component I'd want in lyrics UI, motion, or rendering?** **No.**
1. **Motion:** Our `spring.ts` analytical solver (`app/ui/src/lib/motion/spring.ts`) is already written, tested at `75/144/240 Hz`, and tailored to our morphing `clip-path` + SVG `<rect>` notch; SwiftUI springs from `boring.notch` aren't portable and writing our own was not worse.
2. **Rendering:** Our native GDI DIB pill (`native_dot/glyphs.rs`) and Svelte 5 + CSS `--ice-*` notch components (`base.css`) already match Windows 11 typography/DPI and our budgets; Kotlin/Compose UI code from `Echo-Music`/`BitChord` cannot drop into Win32 or WebView2.
3. **Lyrics UI:** Synced LRC parsing + an active-bright/neighbour-faded scrollable view in Svelte 5 is small (~100–150 lines reusing `Spring1D`), whereas translating Android Compose views is slower and still can't bypass D-22's `B3`/`K3` bans on InnerTube extraction.

Because zero UI, motion, or rendering component justifies copyleft lock-in right now, I vote **`G0 (+2)`** (or **`G2 (+1)`** if the team prefers an explicit future exception path) and **`G1 (−1)`**.
