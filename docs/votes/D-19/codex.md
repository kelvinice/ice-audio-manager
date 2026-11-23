# D-19 · Codex's vote

## Scores

| Option | Score | One-line reason |
|---|---:|---|
| A1 · Hit-testable pill | +2 | A click on the painted pill is discoverable and delivered a complete click without foreground transfer in my fixture. |
| A2 · Optional hotkey | +1 | A user-bound chord gives keyboard users an equivalent route and leaves scarce global chords free by default. |
| A3 · Poll while shown | −2 | The present far-cursor path wakes about ten times per second. Extending it across native idle conflicts with the §2 near-cursor limit. |
| A4 · Invisible top strip | −1 | It intercepts the top row beyond the visible target, precisely where a maximised browser exposes its tabs. |
| Part B · B1–B3 together | +1 | Shared content and side-by-side QA are worth doing; the renderer change earns its place only if the cold native footprint passes its gate. |

Within Part B I score B1 **+2** for one priority rule across renderers, B2 **+1** for legible glyphs subject to the RAM gate, and B3 **+2** because DPI and transparency differences survive a unit test.

## Strongest argument against my top choice

Even a perfectly non-activating pill consumes clicks over somebody else's title bar. A mistaken click can then pay the full Balanced cold-start cost. An optional hotkey avoids both problems for an owner who wants the entire edge click-through.

## New option or amendment

Start A1 with an explicit primary click and immediate native pressed feedback. Route Lite clicks to quick switch. Leave hover-to-open and wheel adjustment as separate gestures until tested with inactive-window scrolling both on and off.

## Evidence I checked myself

- On Windows build 26300.9974 I placed a layered topmost popup over a backing HWND on another UI thread. I kept `WS_EX_NOACTIVATE`, omitted `WS_EX_TRANSPARENT`, returned `MA_NOACTIVATE`, and issued button down/up over an opaque client point. In three instrumented trials I received one `WM_LBUTTONDOWN` and one `WM_LBUTTONUP` each. The popup received zero positive `WM_ACTIVATE` messages during those clicks and the original foreground HWND was still foreground after each click. An earlier exploratory run lost button-up; I excluded it from the completed trials. Wheel and cold WebView wake were outside this fixture.
- The production native window sets both style bits at [native_dot.rs](../../../crates/ice-win/src/native_dot.rs#L943). The shell already answers the activation message on the WebView host at [overlay_win.rs](../../../crates/ice-win/src/overlay_win.rs#L81). Windows documents how per-pixel alpha and the transparent style affect layered-window input. [Microsoft Window Features](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features)

## Position

Ship a clickable visible pill plus the optional keyboard route. Keep fully transparent pixels pass-through and measure the response from press to feedback independently of WebView startup.
