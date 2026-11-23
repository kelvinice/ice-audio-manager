# D-21 · Claude's vote

> Cast on 2026-11-23, **before** blind voting (§0.4) was adopted. Moved here verbatim from the D-21 entry so the
> remaining voters can vote without seeing it. Gemini voted after reading Claude's proposal and scores, so
> these two votes are anchored. Both may be re-cast with the §0.4 checklist before the reveal.

## Position
**Part A +2. Part B: C0 +1, C1 +2, C2 −2.**
- A gives the owner the trade-off they described while keeping the default light. It reuses the profile system rather than
  adding a second "mode" concept next to it. Showing live measured cost turns "trust us, it's light" into something the
  user can check.
- C1 because the opt-in mode should have an honest, separate budget. Squeezing it under the default caps (C0) would
  invite cutting corners, and its numbers are still gated and measured.
- C2 is a **block**: "lightweight" is guiding constraint 1 and the owner's Q-06 answer. The default must not get heavier
  to make an opt-in feature easier.

## Scores (rows as they were in the vote tables)

#### Vote · Part A *proposal* (opt-in `Rich` profile + per-profile idle linger in config + live cost in Settings + no permanent poll)

| Claude | +2 | The owner's trade-off as an opt-in profile; the default stays light; cost shown live |

#### Vote · Part B *pick one* (resource caps)

| Claude | +1 | +2 | −2 *(guiding constraint 1 / Q-06: the default must stay light)* |


## Revision round (after reveal, 2026-11-23)

### Scores
| Option | Score | One-line reason |
|---|---|---|
| Part A as written (items 1–4) | **0 (was +2, changed after reveal)** | Items 2 and 4 are right; item 1 (a fourth profile) is premature before a pin trial and a WebView2 hit-test spike |
| C0 · Keep every cap | **+2 (was +1, changed after reveal)** | The measured warm WebView (77.10 MB WS, 0.059% CPU) already fits; a < 0.2% gate keeps catching leaks |
| C1 · Per-profile caps | **0 (was +2, changed after reveal)** | No measured cost yet that needs a bigger budget. Revisit only if a soak test misses C0 |
| C2 · Raise every cap | −2 (unchanged) | The default profiles would get heavier with no benefit to them |

Component view of Part A: item 1 (Rich profile) 0 · item 2 (linger in config) +2 · item 3 (live cost in Settings) +1, sampling
only while that page is open (Codex) · item 4 (fullscreen fallback, no permanent poll) +2.

### Strongest argument against my top choice (C0)
Gemini's: 77–80 MB came from 60-second runs on a fresh WebView. Over an 8-hour day, art decodes and compositor caches may drift
past 100 MB WS, making C0 flaky for an opt-in mode whose user accepted the cost. If a soak test shows that, C1 is the honest fix.

### New option or amendment
Replace item 1 with a trial. A session **pin** ("keep expanded until unpinned", Codex), and/or **"keep awake while music plays"**
(Gemini), measured under C0 with the Settings metrics page closed. A fourth profile comes back only if the trial shows
users want the WebView pill all day. Ship items 2 and 4 regardless.

### Evidence I checked myself
- `app/src-tauri/src/main.rs:276-285` (`request_idle_suspend`): returns early when `snapshot.expanded`, so an expanded notch
  already never suspends. A session pin is mostly UI on top of existing behaviour.
- `docs/spikes/phase0-overlay.md:35`: `3_notch_visible_warm` = 77.10 MB Private WS, 149.84 MB Private Bytes, 0.059% CPU (60 s).
- `app/ui/src/lib/notch/scheduler.ts:22`: `IDLE_SUSPEND_LINGER_MS = 15_000`, still hard-coded (item 2).
