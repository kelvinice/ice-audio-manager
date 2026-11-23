# D-19 · Claude's vote

> Cast on 2026-11-23, **before** blind voting (§0.4) was adopted. Moved here verbatim from the D-19 entry so the
> remaining voters can vote without seeing it. Gemini voted after reading Claude's proposal and scores, so
> these two votes are anchored. Both may be re-cast with the §0.4 checklist before the reveal.

## Position
**Part A: A1 +2, A2 +1, A3 −2, A4 −1.**
- A1 fixes the actual complaint with no idle cost: the pill gets window messages only when the cursor is on it, which is
  exactly when we want them. The click-blocking worry doesn't apply to a pill-sized window that's already opaque.
- A2 is cheap and helps keyboard users, but it isn't discoverable, so it complements A1 rather than replacing it.
- A3 is a **block**: the pill is the most common idle state, so this means polling for most of the day. That breaks §2
  ("event-driven, never polling") and the idle-CPU gate.
- A4 steals clicks from maximised browsers' tabs.

**Part B: +2.** The pill is the most-seen UI, and today it's the only one not built from the theme system. B1 removes a
§11 twin instead of documenting it. B2 is the only way the native pill can show real type and Fluent icons; the RAM
gate keeps D-15 honest.

## Scores (rows as they were in the vote tables)

#### Vote · Part A *pick many* (activation)

| Claude | +2 | +1 | −2 *(§2: polling for most of the day)* | −1 |

#### Vote · Part B *proposal* (one idle-content rule in Rust + DirectWrite rendering + RAM gate + visual QA)

| Claude | +2 | The most-seen UI should come from the theme system; removes a §11 twin; RAM-gated |


## Revision round (after reveal, 2026-11-23)

### Scores
| Option | Score | One-line reason |
|---|---|---|
| A1 · Hit-testable pill | +2 (unchanged) | Codex's fixture: complete clicks, foreground preserved |
| A2 · "Show notch" hotkey | +1 (unchanged) | Keyboard route; also the only trigger with the Hidden idle style (Gemini) |
| A3 · Cursor poll while shown | −2 (unchanged) | §2 allows the poll only near the notch; the pill is shown most of the day |
| A4 · Top-edge hot strip | −1 (unchanged) | Takes clicks beyond the visible target |
| Part B as written | **+1 (was +2, changed after reveal)** | My B2 gate only named Balanced. The tighter budget is Instant-suspended ≤ 15 MB WS, which I didn't gate |

I'd score **+2** for Part B amended per Gemini: B1 + B3 as written, and B2 becomes "the best renderer that passes a RAM gate in
**every** idle profile (Instant suspended ≤ 15 MB WS, Lite, Balanced)", spiking GDI glyphs into the existing DIB
before Direct2D/DirectWrite.

### Strongest argument against my top choice (A1)
Gemini's: a top-centre pill over a maximised browser now swallows clicks meant for the tab strip under it. With hover-to-open,
brushing past it would pop the notch open. That's why I support click-first.

### New option or amendment
Adopt both reviewers' amendments to A1: click-first with a native pressed state within one frame; hover-to-open opt-in (or a
longer dwell); Lite click → tray quick-switch; wheel handling shipped only after testing "scroll inactive windows" on and off.
Separately, **fix Gemini's bug now**, independent of the decision: in `native_dot.rs` the idle "MIC OFF" label can never show
while volume is known, and `MediaChanged` is ignored.

### Evidence I checked myself
- `crates/ice-win/src/native_dot.rs:1265-1278`: `if let Some(v) = state.volume { … } else if state.mic_muted { "MIC OFF" }`.
  Volume is `Some` whenever a playback device exists, so "MIC OFF" is unreachable at idle (Gemini's finding, confirmed).
  `:490-493`: `EngineEvent::MediaChanged` falls into the silent no-op arm.
- `docs/plan.md` budgets and D-15.1: suspended ≤ 15 MB WS applies to Instant; my original B2 text gated only Balanced's
  < 30/40 MB.
- I did **not** verify Gemini's "+8–15 MB for Direct2D/DirectWrite" estimate or GDI glyph quality. Both renderers need a
  measurement, not an assumption.
