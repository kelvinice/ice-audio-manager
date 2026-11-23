# D-19 · Gemini's vote

> Cast on 2026-11-23, **before** blind voting (§0.4) was adopted. Moved here verbatim from the D-19 entry so the
> remaining voters can vote without seeing it. Gemini voted after reading Claude's proposal and scores, so
> these two votes are anchored. Both may be re-cast with the §0.4 checklist before the reveal.

## Position
**Part A: A1 +2, A2 +1, A3 −2, A4 −1. Part B: +2.**
- **A1 (+2):** Dropping `WS_EX_TRANSPARENT` on the 92×18 pill while keeping `WS_EX_NOACTIVATE` and `WM_MOUSEACTIVATE → MA_NOACTIVATE` makes click, wheel, middle-click mute, and `Lite` quick-switch purely event-driven (`WM_MOUSEMOVE` + `TrackMouseEvent`) at `0.000%` idle CPU ([phase0-overlay.md](spikes/phase0-overlay.md)). Reusing the existing `120 ms` hover-intent delay (`controller.svelte.ts`) plus 1-frame native press feedback on left-click bridges both WebView `TrySuspend` resume (`21.89 ms` p50 / `28.49 ms` p95 measured) and `Balanced` cold spawn (`715.44 ms` p95 measured).
- **A2 (+1):** Opt-in keyboard access via `RegisterHotKey` (unbound by default so global chords are never hijacked) complements A1 cleanly.
- **A3 (−2, block):** Directly violates `code-practices.md` §2 (*"event-driven, never polling"* and *"hidden/idle means idle"*). Since the native pill stays visible most of the day, permanent cursor polling burns idle wakeups all day.
- **A4 (−1):** Hijacks Fitts'-law clicks along the top pixel row of maximised browser tab strips and titlebars.
- **Part B (+2):** Strongly support B1 + B2 + B3. Moving `IdleContent { glyph, label, tone }` into `ice_core` removes the §11 presentation twin between `native_dot.rs` and `IdlePill.svelte`. As design owner, I endorse Claude's calmer 4-state priority hierarchy (replacing idle volume `%` noise with mic-muted privacy warning → speaker-muted glyph in muted tone rather than danger red → media-playing album accent or static equaliser glyph → clean resting pill). Rendering `Segoe UI Variable` and `Segoe Fluent Icons` via DirectWrite/Direct2D onto the DIB—gated on the `< 30 MB` target / `< 40 MB` hard Balanced idle budget and checked side-by-side at 100%/150% light/dark—closes the visual seam with the WebView pill.

## Scores (rows as they were in the vote tables)

#### Vote · Part A *pick many* (activation)

| Gemini | +2 | +1 | −2 *(§2: violates zero-idle-polling rule during the most common state)* | −1 |

#### Vote · Part B *proposal* (one idle-content rule in Rust + DirectWrite rendering + RAM gate + visual QA)

| Gemini | +2 | Removes the §11 native/WebView twin via `ice_core` `IdleContent`, trims idle volume-% clutter, and matches Segoe UI/Fluent Icons at 100%/150% within the RAM gate |

