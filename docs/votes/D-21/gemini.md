# D-21 · Gemini's vote

> Cast on 2026-11-23, **before** blind voting (§0.4) was adopted. Moved here verbatim from the D-21 entry so the
> remaining voters can vote without seeing it. Gemini voted after reading Claude's proposal and scores, so
> these two votes are anchored. Both may be re-cast with the §0.4 checklist before the reveal.

## Position
**Part A: +2. Part B: C0 +1, C1 +2, C2 −2.**
- **WebView's idle CPU when nothing animates (measured):** In Phase 0.2 ([phase0-overlay.md](spikes/phase0-overlay.md)), a settled visible WebView notch (`3_notch_visible_warm`) measured **0.059% avg CPU** over 60 s at **77.10 MB Private WS** (`149.84 MB` Private Bytes; `80.45 MB` Private WS in Phase 2 `bench-p2`), versus `0.004%–0.009%` CPU and `6.00–6.44 MB` Private WS when `TrySuspend`ed ([manual-checklist.md](qa/manual-checklist.md)). In Phase 3 CDP traces, once springs and visualizer bars settle on silence or pause, both `SpringGroup` and `VisualizerController` stop completely (**`0` `rAF` calls and `0` active intervals**). Thus, keeping the WebView warm at idle in `Rich` buys static pixels primarily with **RAM (~71–74 MB extra Private WS over suspended)**, *provided* Part A item 4 replaces `spawn_cursor_poll` with event-driven hit-testing so no 30 Hz cursor loop runs forever.
- **What `Rich` should add at idle (judgment on art glow, live bars, animated idle):**
  - **Base `Rich` idle + static art-accent glow (on in `Rich`):** Keeping `IdlePill.svelte` awake gives real `Segoe UI Variable` typography, 1 px `non-scaling-stroke` SVG rim, rounded `AlbumArtTile`, and `0 ms` hover-intent responsiveness without resume lag. **Static art glow** (`.accent-glow` in `NotchApp.svelte`, a composited CSS `radial-gradient` tinted by `--ice-active-glow` at `opacity: 0.28`) can stay visible at idle whenever media is loaded because it triggers zero `rAF` frames while static and only crossfades (`400 ms`) on track changes.
  - **Live visualizer bars while music plays (opt-in sub-toggle under `Rich`, off by default):** In Phase 3 (`qa/manual-checklist.md`), our 5-bar `scaleY`/`opacity` spring animation at 30 Hz meter feed measured **0.435% whole-machine CPU** in CDP (`239.8 µs` avg `rAF`, `0 ms` layout) and **~0.21% whole-machine CPU** (`2.522%` single-core `TaskDuration`) in `--fake-engine` WebView2 E2E (plus `0.013%` for `ice-meter`), settling back to `0` `rAF` calls once paused or silent. Keeping compact idle bars opt-in gives enthusiasts active-music motion under `< 1%` CPU without forcing continuous animation on base `Rich`.
  - **Continuous "animated idle" when nothing is playing (reject / do not build):** Do not add perpetual breathing dots, rotating border shaders, or looping shimmers during silence. A never-ending `rAF` or CSS animation loop at idle breaks `code-practices.md` §2 (*"Hidden/idle means idle"*), prevents deep CPU/GPU idle states, and adds visual noise without information. Every animation in `Rich` must remain state- or audio-driven and settle to `0` `rAF` at rest.
- **Part A (+2) & Part B (C0 +1, C1 +2, C2 −2):** Reusing `PerformanceProfile` with config-driven idle linger (`IDLE_SUSPEND_LINGER_MS`), showing live `proc_metrics` cost in Settings, falling back to `Lite` over fullscreen/games, and gating `Rich` on event-driven hit-testing is clean. Under Part B, **C2 is a block**: default profiles (`Balanced`, `Instant`, `Lite`) must preserve Q-06 / Guiding Constraint 1 (`≤ 15 MB` suspended WS, `< 0.2%` idle CPU). **C1 (+2)** gives opt-in `Rich` honest, CI-measured caps (`≤ 120 MB` WS, `≤ 250 MB` PB, `< 0.5%` static idle CPU / `< 1%` with opt-in live bars) without pressuring corner-cutting.

## Scores (rows as they were in the vote tables)

#### Vote · Part A *proposal* (opt-in `Rich` profile + per-profile idle linger in config + live cost in Settings + no permanent poll)

| Gemini | +2 | Fits `PerformanceProfile`; base `Rich` + static art glow adds only ~0.06% measured idle CPU once hit-testing is event-driven; keep idle bars opt-in and reject perpetual silence loops |

#### Vote · Part B *pick one* (resource caps)

| Gemini | +1 | +2 | −2 *(guiding constraint 1 / Q-06: default profiles must keep ≤ 15 MB idle / < 0.2% CPU gates)* |

