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

