# D-21.1 · Claude's vote

> Sealed at proposal time (§0.4 rule 2). Claude wrote the entry; it contains no scores or verdicts.

## Scores

| Option | Score | One-line reason |
|---|---|---|
| L1 · Idle linger in config | +2 | Removes a §3 magic number; no behaviour change at the defaults |
| L2 · Event-driven hit-testing | +2 | Today's poll runs whenever the WebView notch is visible, in every profile, so this fixes a §2 deviation for everyone and unblocks L3–L5 |
| L3 · Session pin | +1 | Nearly free (an expanded notch already never suspends), and the user chooses the cost knowingly; a forgotten pin is the risk |
| L4 · Awake while playing | +1 | Shows the rich pill exactly when there is something to show; should be opt-in, because heavy listeners would pay ~71 MB all day |
| L5 · `Rich` profile | −1 | With L3/L4 available, a whole profile adds a settings concept and the most RAM for little extra benefit |
| L6 · Live cost readout | +1 | Makes the trade-offs in L3–L5 visible; sampling must stop when the page closes |

## Strongest argument against my top choice (L2)

The poll it replaces is cheap in practice. The Phase 0.2 visible-WebView run measured 0.059 % CPU **with** the bench
cursor poll running at 16 ms (`main.rs` starts it with `benchmark = true`). So L2 is mostly about principle (§2), and the
WebView2 child windows may make a window region hard: it clips glow and spring overshoot, and has to follow the springs.
It could cost a lot of engineering for a near-zero measured gain.

## New option or amendment

- L3: show a small pin mark on the pill while pinned, and unpin automatically on session lock or sleep, so a forgotten pin
  doesn't last for days.
- L4: off by default, one switch in Settings → Notch.
- L2: spike it first on the real WebView2 host (Codex's region test used a native child). If the region can't follow the
  springs cleanly, keep the poll but cap it to "visible **and** cursor within the notch's monitor", which narrows when it runs.

## Evidence I checked myself

- `app/src-tauri/src/main.rs` `spawn_cursor_poll`: sleeps 34 ms near the notch and 100 ms otherwise, and runs while
  `notch_visible`. The bench path calls it with `benchmark = true` (16 ms).
- `app/src-tauri/src/main.rs:276-285` `request_idle_suspend`: returns early when `expanded`.
- `app/src-tauri/src/placement.rs`: when the WebView isn't allowed (gaming), it sets `view.expanded = false`, so a pin can't
  survive a game.
- `crates/ice-core/src/media.rs:14-21`: `PlaybackStatus::{Playing, Paused, Stopped, Changing, Opened, Closed}`. L4 can key
  on `Playing`.
- `docs/spikes/phase0-overlay.md:35`: `3_notch_visible_warm` 77.10 MB WS / 0.059 % CPU.

## Position

Ship L1 and L6 with the Settings work, and L2 if the spike passes. Offer the extra cost as two small opt-ins (pin, awake while
playing) rather than a fourth profile.
