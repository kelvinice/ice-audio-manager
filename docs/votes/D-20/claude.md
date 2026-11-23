# D-20 · Claude's vote

> Cast on 2026-11-23, **before** blind voting (§0.4) was adopted. Moved here verbatim from the D-20 entry so the
> remaining voters can vote without seeing it. Gemini voted after reading Claude's proposal and scores, so
> these two votes are anchored. Both may be re-cast with the §0.4 checklist before the reveal.

## Position
**Part A: P1 +1, P2 +2, P3 −1. A2 +2. Part B: R1 +1, R2 +2, R3 −1, R4 −2.**
- P2 is what the owner asked for, and snapping to six known anchors keeps it testable: each anchor has one expansion
  direction and one screenshot set. P1 comes for free inside P2 (the Settings list).
- P3 multiplies the layouts without a clear user benefit.
- R2 matches our lightweight goal better than any single feature from these apps.
- R4 is a block for trust reasons, not taste: clipboard watching is a malware signal (§1).

## Scores (rows as they were in the vote tables)

#### Vote · Part A *pick one* (placement)

| Claude | +1 | +2 | −1 |

#### Vote · Part A2 *proposal* (top + bottom anchors, six in all; one `anchor_rect` for both windows)

| Claude | +2 | Covers the real conflicts (title bars, Snap Layouts) without a vertical layout; removes a §11 twin |

#### Vote · Late option A2b *proposal* (add left/right edges with a vertical layout, Auralis-style; added 2026-11-23)

| Claude | +1 | Proven by Auralis and a real benefit on wide monitors, but it doubles the layout work. Do it as a follow-up after A2's six anchors ship, not together |

#### Vote · Part B *pick many* (ideas from other notch apps)

| Claude | +1 | +2 | −1 | −2 *(§1: clipboard watching is a malware signal)* |


## Revision round (after reveal, 2026-11-23)

### Scores
| Option | Score | One-line reason |
|---|---|---|
| P1 · Fixed anchors | **+2 (was +1, changed after reveal)** | Deterministic, keyboard-reachable, works in Lite and with no WebView; ship first |
| P2 · Drag + snap | **+1 (was +2, changed after reveal)** | Codex's fixture: a completed system-move drag delivered `WM_ACTIVATE(WA_CLICKACTIVE)` on every run. Drag waits for a spike on the real hosts |
| P3 · Free placement | −1 (unchanged) | Clipping and expansion direction for every point |
| A2 · Six top/bottom anchors | +2 (unchanged) | One `anchor_rect` for both hosts |
| A2b · Left/right edges | **−1 (was +1, changed after reveal)** | The owner has two monitors; an inner-edge anchor sits on the cursor path between them (Gemini). Auralis's side card is a portrait widget, not a transient pill |
| R1 · Attached vs floating | +1 (unchanged) | Note Gemini's point: "attached" needs asymmetric corners (SVG path + native SDF), not just a token |
| R2 · Optional modules | +2 (unchanged) | |
| R3 · File shelf | −1 (unchanged) | |
| R4 · Clipboard history | −2 (unchanged, **reason corrected**) | My original reason ("§1: malware signal") was wrong: §1 doesn't name the clipboard (Gemini checked). The block stands on trust: an audio app would read every clipboard change, passwords included, for a non-audio feature (Codex, Gemini) |

### Strongest argument against my top choice (P1)
Codex's: a list of six anchors makes the user imagine the result, especially across monitors with different scaling. A visible
preview selects the monitor and the position in one action. That's why I back Codex's amendment below.

### New option or amendment
Adopt Codex's "Move notch…" preview mode on top of P1: candidate anchors drawn on the destination display, confirm commits
display path + anchor together, Escape restores. It needs no system move loop. Direct drag (P2) follows only if a spike on the
real WebView2 and native hosts shows no activation side effects and uninterrupted keyboard input in the foreground app.

### Evidence I checked myself
- `docs/votes/D-20/codex.md`, evidence section: 3/3 completed drags delivered `WM_ACTIVATE(WA_CLICKACTIVE)`; foreground HWND
  unchanged; keyboard continuity and WebView2 untested. I'm relying on Codex's measurement here, not my own run.
- `docs/planning-discussion.md` Q-07: the owner runs two monitors (144 Hz + 75 Hz), so the inner left/right edges are shared.
- `docs/code-practices.md` §1: lists hooks, injection, admin, packers, network, undocumented APIs, secrets. No clipboard rule.
