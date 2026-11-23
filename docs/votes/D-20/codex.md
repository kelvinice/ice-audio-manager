# D-20 · Codex's vote

## Scores

| Option | Score | One-line reason |
|---|---:|---|
| P1 · Choose fixed anchors | +2 | A tray or Settings choice has a deterministic destination and works with keyboard input, Lite and an absent WebView. |
| P2 · Drag and snap | +1 | A deliberate move gesture is useful, but a completed native drag sent the popup an activation message even with the no-activate style. |
| P3 · Free placement | −1 | Every saved point needs clipping, expansion-direction and topology recovery rules for a benefit the anchor list already largely provides. |
| A2 · Six top/bottom anchors | +2 | A single calculation from the monitor work area handles taskbars and gives the native and WebView hosts the same edge. |
| R1 · Attached or floating appearance | +1 | Two silhouettes are a reasonable theme choice if their painted and interactive footprints agree. |
| R2 · Optional modules | +1 | Turning off the visualizer has a concrete resource payoff; each additional switch needs its own observable idle behavior. |
| R3 · File shelf | −1 | File drops would compete with listening controls for the same small target and add a new data-handling feature. |
| R4 · Clipboard history | −2 | Collecting unrelated clipboard contents, including possible passwords, crosses the trust boundary of an audio utility. |

### Sealed addendum · late option A2b

| Option | Score | One-line reason |
|---|---:|---|
| A2b · Left and right edges with vertical layout | −1 | Adding coordinates is cheap; a readable portrait card, hit regions and all activity transitions at mixed DPI are a second layout to support. |

## Strongest argument against my top choice

A list of six anchors requires knowing which monitor is pinned and imagining the result. A visible drag preview could select the monitor and position in one action, particularly when the displays have different scale factors.

## New option or amendment

Offer “Move notch…” from the tray as a preview mode. Show candidate anchors on the destination display, commit the display path and anchor together on confirmation, and let Escape restore the old pair. This can ship on top of P1 independently of ordinary click behavior.

## Evidence I checked myself

- On Windows build 26300.9974 my layered popup returned `MA_NOACTIVATE` and, only for dragging, `HTCAPTION`. With normal system handling and a synthetic pointer drag, each of three completed runs entered the system move loop once, delivered three `WM_MOVING` messages, exited once, and moved (+36,+12) physical pixels. **Each drag delivered `WM_ACTIVATE(WA_CLICKACTIVE)` to the popup.** The foreground HWND was unchanged at entry, sampled moves, exit and the end; it was not the popup even in the activation-message handler. This distinguishes a window activation message from a foreground transfer. I did not measure uninterrupted keyboard delivery or test a WebView2 child drag.
- I traced the two independent top-centre calculations at [overlay_win.rs](../../../crates/ice-win/src/overlay_win.rs#L124) and [native_dot.rs](../../../crates/ice-win/src/native_dot.rs#L1156). The monitor struct already supplies signed work-area coordinates and dimensions. [monitors.rs](../../../crates/ice-win/src/monitors.rs#L45)

## Position

Deliver the shared anchor selector first. Before enabling direct drag, run against both actual hosts with a text editor retaining keyboard input throughout, then check cross-monitor DPI changes and cancellation.
