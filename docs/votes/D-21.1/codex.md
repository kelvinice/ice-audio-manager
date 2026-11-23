# D-21.1 · Codex's vote

## Scores

| Option | Score | One-line reason |
|---|---|---|
| L1 · Per-profile linger | +1 | Keeping 15 seconds as the default lets users tune the pause between interactions without changing existing installations. |
| L2 · Event-driven hit-testing | +2 | A region on the actual notch HWND routed stationary points correctly across the WebView2 child boundary and gives us a credible way to retire the visible cursor poll. |
| L3 · Session pin | +1 | Someone actively comparing playback devices can choose when to end that interaction, provided the pin state and unpin action stay visible. |
| L4 · Awake while playing | −1 | A long background playlist can keep the renderer resident for hours after the last notch interaction. |
| L5 · Rich profile | −1 | Keeping a renderer awake through every idle period spends memory even between playback sessions and adds another profile to explain. |
| L6 · Live cost readout | +1 | A page-scoped process-tree measurement would let users see the cost of their chosen profile on their own machine. |

## Strongest argument against my top choice

A window region clips pixels as well as input. Updating it to the destination at the start of a spring could cut off a moving edge or deliver a click to the application underneath a visibly painted control. Updating it every animation frame also has a cost that the stationary probe cannot quantify.

## New option or amendment

Gate L2 on a moving-edge test at 100% and 150%. Sample a control just inside the painted edge and a point just outside throughout expansion, overshoot and collapse. Check glow clipping visually. Count region updates after settling and require zero until geometry or DPI changes.

## Evidence I checked myself

- I launched the app's Windows release executable with an isolated profile and `--fake-engine --show-notch`. On its 420 × 190 pixel notch HWND, I cleared `WS_EX_TRANSPARENT` for the comparison. `WindowFromPoint` at client offsets (202, 16) and (218, 16) initially returned a `Chrome_RenderWidgetHostHWND` rooted at the notch for both. After `SetWindowRgn(CreateRoundRectRgn(190, 4, 210, 32, 4, 4))`, only the first point still had the notch as its root. I verified the root HWND because the underlying window also had a Chromium child. This measured stationary point routing. The cursor poll remained compiled in, and the probe did not inject clicks or measure animation, focus or CPU.
- In `app/src-tauri/src/main.rs`, I traced `spawn_cursor_poll` from the visible-window path. It reads cursor and window coordinates, expands the reported rectangle by six logical pixels, and sleeps for 34 ms near the window or 100 ms far away in normal use. The worker caches click-through changes but still wakes for those samples.
- I checked the separate notch and Settings WebView builders in the same file. The Settings WebView has its own data directory. For L6 I would include its child processes and label the readout as the cost with Settings open.

## Position

Prioritize L2 and preserve the existing idle budgets. Ship bounded linger and explicit session pinning as user choices. Sample L6 only while General is visible, including stopping when the user switches Settings pages.
