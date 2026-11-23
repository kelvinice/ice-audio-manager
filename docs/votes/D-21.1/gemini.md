# D-21.1 · Gemini's vote

> Cast 2026-11-23 blind (§0.4). Written from the entry and my own checks of the code and measurements. I haven't opened
> any other agent's file for this decision.

## Scores

| Option | Score | One-line reason |
|---|---|---|
| L1 · Idle linger in config, per profile | +2 | Fixes a §3 magic number at zero behaviour change; the timer lives in the UI, so the value just rides the notch snapshot |
| L2 · Event-driven hit-testing (region) | +1 | The right end state, but only a window region can work, it clips the drop shadow, and it must follow the morph; spike first |
| L3 · Session pin | +1 | Already built (the Pin button holds the notch open); what's left is making its cost and release rules explicit |
| L4 · Keep awake while music plays | +1 | The only way a playing track gets the real WebView pill (art, title, bars); fits C0; must be an opt-in, default off |
| L5 · `Rich` profile | −1 | L1 + L3 + L4 cover its cases piecewise; a fourth profile multiplies the QA matrix and `Instant`/`Balanced` are already near-twins |
| L6 · Live cost readout | +1 | Cheap, honest, and the natural place to show what L3/L4 cost; sampling must end when the page closes |

## Strongest argument against my top choice

**Against L1:** the linger timer runs in the WebView's JS (`scheduler.ts:562-600`, armed from `controller.svelte.ts:125`).
Raising it to the 300 s the entry suggests as an upper bound means a user who nudges the volume every few minutes keeps
the ~71 MB WebView alive nearly all day without ever choosing "always awake", and the settings page then shows a harmless-looking
number slider. C0 still holds (77–80 MB WS ≤ 100 MB), but the *intent* of the 15 s default (Q-06: lower when not in use) quietly
erodes. A config-file-only value would satisfy §3 without inviting that.

## New option or amendment

1. **L1 amendment:** ship it config-only first (validated 5–120 s, default 15 s), and surface it in Settings only together with
   L6, so the cost of a long linger is visible next to the slider.
2. **L2 amendment:** the region is the bounding box of *(current ∪ target)* plus the Bouncy overshoot, set when a morph starts and
   shrunk to the target when springs settle. That is two calls per morph, not a per-frame report, and it solves the
   "target, not springs" problem. Plan for the drop shadow: either clip it on the compact pill (accept a flat shadow) or keep a
   non-clickable halo, which a region can't do; decide with screenshots.
3. **L2 non-option:** `WM_NCHITTEST → HTTRANSPARENT` on the notch window won't work, because WebView2's `Chrome_WidgetWin_*`
   children are hit-tested first and live in another process. Subclassing them would break `code-practices.md` §1.2. So L2
   means `SetWindowRgn`, nothing else.
4. **L4 amendment:** make it a setting (`keep awake while playing`, default off), and drop it automatically in gaming mode, which
   already releases the WebView.

## Evidence I checked myself

- `app/ui/src/lib/notch/scheduler.ts:22, 562-600` and `controller.svelte.ts:117-139`: the 15 s linger is a JS constant read by a
  JS timer, so L1 only needs the value delivered in the snapshot; `holdOpen` (expanded, hover, pointer inside) already postpones it.
- `app/ui/src/lib/notch/controller.svelte.ts:166` with `geometry.ts:70-77`: `report_hit_rect` sends the **target** size when the
  target is set, not the spring's current size. A region fed from that would clip a growing pill for the first frames.
- `app/src-tauri/src/main.rs:553-577`: the poll runs at 34 ms near the notch and 100 ms otherwise, with a 6 px pad
  (`main.rs:565`). Wakeups: about 600 per minute at rest, even when nothing is on screen but the settled pill. Measured CPU of
  that state is small (0.059 %, `phase0-overlay.md:35`, even with the 16 ms benchmark poll), so L2 is about §2 and idle wakeups,
  not about a CPU number.
- `app/ui/src/notch/NotchApp.svelte:128` (`.stage { filter: drop-shadow(var(--ice-drop-shadow)) }`) and `docs/theming.md:32`
  (`0 8px 18px`): the shadow extends up to ~26 px beyond the pill; the canvas is 420×190 with `TOP_PAD = 6`. A region that hugs
  the pill removes it.
- `ExpandedMediaCard.svelte` / `ExpandedNotch.svelte` ("Pin" / "Pinned" buttons) and `main.rs:280` (`request_idle_suspend`
  returns when `expanded`): L3's behaviour exists today; only its visibility and release rules are open.
- `crates/ice-win/src/native_dot.rs`: the native pill ignored `MediaChanged` (fixed in the same session) and cannot draw art or a
  title, so a track only gets the WebView pill if the WebView is awake. That is L4's real benefit.
- `app/src-tauri/src/placement.rs:20-22, 209`: `Instant` and `Balanced` share `allows_webview()`; they differ only in whether the
  notch is shown at start. That is why a third WebView profile (L5) adds little.
- `crates/ice-win/src/proc_metrics.rs:61-83`: `sample_pids` and `cpu_percent_between` already exist, so L6 needs no new Windows code.

## Position

Take L1 (config-first), L6 and L4 (as an opt-in setting); keep L3 as it is and make its cost visible; run the L2 spike with the
bounding-box region and decide on the shadow from screenshots; skip L5. Every piece stays inside C0 (≤ 100 MB WS, < 0.2 % idle
CPU) and each is event-driven: nothing here adds a loop that runs while the notch is idle.
