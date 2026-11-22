# Phase 0.5 · Fullscreen detection

The probe checks the foreground window against each monitor. Phase 2 can use the coverage
of the notch's pinned monitor to decide whether to enter gaming mode.

## Run from PowerShell

```powershell
cargo run --release -p ice-win --example fullscreen_spike
cargo run --release -p ice-win --example fullscreen_spike -- watch 45
```

The first command takes a snapshot. The second reports changes for the requested number of
seconds. Look for `fullscreen=true` on the intended display. The adjacent shell hint applies
to the whole desktop session. `DISPLAY1` and `DISPLAY2` are diagnostic labels for this session.
The persistent monitor choice still uses the display path (F14).

`ice_win::fullscreen::snapshot` takes a per-monitor DPI-aware sample. It prefers the visible DWM
frame over the Win32 window rectangle, uses the full monitor rectangle, and excludes ordinary
captioned maximized windows. During an inconclusive focus transition, callers keep their previous
state. The watcher sleeps between out-of-context WinEvents for foreground, window state, and
window position changes. [DWM window bounds](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect)
[WinEvent delivery](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook)

## Observed results

Tested with a temporary native Win32 and Direct3D 9 fixture on the 1920×1080 displays at
144 Hz (primary) and 75 Hz (secondary). The fixture used the display's current mode and the
fullscreen D3D9 creation calls succeeded on both adapters.

| Case | Result |
| --- | --- |
| Captioned maximized window covering the primary display bounds | Both displays stayed clear. |
| Change the same foreground window from framed to borderless fullscreen | Only the primary display became covered. The shell hint stayed false. |
| Move the same fullscreen window to the secondary display | Coverage moved to the secondary display without requiring a focus change. |
| Three rapid focus round trips between fullscreen and a normal window | The watcher reported all six changes with about 140 ms between inputs. |
| Fullscreen D3D9 swap chain on either monitor | Direct snapshots correctly identified only the covered display. The shell hint was true. |
| Start primary D3D9 fullscreen directly from a background fixture | The watcher missed the takeover even though a simultaneous snapshot detected it. The secondary transition was observed while the fixture was already foreground. |

The focus fixture exercised rapid foreground changes programmatically. Recheck with keyboard
Alt+Tab, the owner's games, and mixed display scaling when testing the real notch. Unit tests
cover negative desktop coordinates, partial coverage, and maximized captions.

## Phase 2 handoff

Use monitor geometry for placement-specific decisions. A false shell hint can coexist with
borderless fullscreen, and the hint can stay true during a transition after the foreground
window changes. The shell query does not send start or stop notifications.
[Windows shell notification state](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shqueryusernotificationstate)

Treat the spike's WinEvent watcher as a partial trigger. Resample at startup, on a pin or display
change, and before revealing the notch in response to a new activity. A confirmed covered target
must suppress the reveal even if an earlier window notification was missed. The host should
handle `WM_DISPLAYCHANGE` for resolution changes. Keep manual gaming mode usable while validating
long-lived overlay suppression against real games.
[Windows display change message](https://learn.microsoft.com/en-us/windows/win32/gdi/wm-displaychange)
