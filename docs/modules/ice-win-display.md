# ice-win display, windows and process helpers

> Paths: `crates/ice-win/src/{overlay_win,monitors,fullscreen,webview2,proc_metrics,desktop,autostart,music_service}.rs` · Owner: Gemini (overlay, monitors, webview2, proc_metrics), Codex (fullscreen, autostart, desktop) · Verified: 2026-11-23

## Purpose
Safe wrappers over the Win32/WebView2 pieces the shell needs around the notch window: window styles and positioning, monitor
topology, "is a game covering the notch's monitor", WebView2 suspension, process memory/CPU sampling, single instance, tray
integration and autostart. All `unsafe` stays here. No hooks into other processes (§1.1/§1.2).

## Files
| File | What it does |
|---|---|
| `overlay_win.rs` | `apply_overlay_styles` (transparent, topmost, `WS_EX_NOACTIVATE`/tool window, `MA_NOACTIVATE`), `remove_overlay_subclass`, `set_click_through`, `notch_origin` / `notch_origin_at` / `canvas_origin` / `move_notch` (D-20 clamped `(anchor, x_ratio, style)` in work area), `ensure_topmost`, `move_pos_no_resize`, `move_window_async`, `apply_settings_mica`, `cursor_pos`, `primary_button_down`, `escape_down`, `drag_threshold`, `window_rect`, `foreground_hwnd`, `reduce_motion_enabled` |
| `drag_ghosts.rs` | `DragGhosts`: temporary non-activating, click-through layered slot overlays shown on all monitors during a D-20 drag (with `IceDragGhost` pill outline highlighted in accent under the nearest slot) |
| `monitors.rs` | `MonitorInfo` (id, persistent display path, size, DPI scale, refresh Hz, work area), `list_monitors`, `resolve_target_monitor`, `PlacementMode` (`Pinned` default, `Primary`, `Cursor`) |
| `fullscreen.rs` | `FullscreenWatcher::start(sink)` on a hidden-window thread: WinEvent notifications + `WM_DISPLAYCHANGE` → `FullscreenEvent::{Changed, DisplaysChanged}`; `snapshot()`, `is_monitor_covered(snapshot, monitor)`. `SHQueryUserNotificationState` is a hint only |
| `session_watch.rs` | `SessionWatcher::start(sink)`: same hidden top-level window pattern; `WTSRegisterSessionNotification` + `WM_POWERBROADCAST` → `SessionEvent::{Locked, Unlocked, Suspending, Resumed}` (2026-11-24, for the notch pin) |
| `webview2.rs` | `try_suspend`, `resume`, `process_infos` (PIDs of the app's WebView2 tree), `on_process_exit` |
| `proc_metrics.rs` | `sample_pids`, `cpu_percent_between`, `TreeSnapshot`: Private Bytes, private working set, CPU for the host + WebView2 processes |
| `desktop.rs` | `InstanceGuard::acquire` (single instance), `finish_tray_menu`, `notify_tray` (tray balloon by GUID) |
| `autostart.rs` | `Autostart`: per-user launch at sign-in (HKCU `…\Run`). `enabled` / `set_enabled` use `ICE_STARTUP`; `enabled_as` / `set_enabled_as` take a `StartupEntry` (value name + args) so Pandora registers its own entry |
| `music_service.rs` | `open(MusicServiceId)`: refuses `Pandora` (the shell opens it through `pandora.rs`); on user click, launches the installed Spotify / YouTube Music app (Store app id, `spotify:` protocol, or Chrome/Edge PWA id) and falls back to the official website. STA COM, no network by the app itself |

## How it works
- **Gaming mode:** the shell starts one `FullscreenWatcher`. `Changed` / `DisplaysChanged` → `placement::reconcile` (see [app-shell](app-shell.md)).
  Coverage = foreground window bounds vs the target monitor rect; borderless and exclusive fullscreen both counted. Exclusive-mode
  transitions can be missed by window notifications (Phase 2 follow-up, [phase0-fullscreen](../spikes/phase0-fullscreen.md)).
- **Pinned monitor** is saved by display path (stable across reconnects), not by the session-local GDI name. Falls back while disconnected.
- **Overlay:** the notch must never take focus. Click-through is toggled by the shell's bounded cursor poll (≤ 30 Hz, only while the cursor is
  near the pill); the native pill instead takes real input messages ([ice-win-native-pill](ice-win-native-pill.md)).
- **Memory accounting** uses the dedicated WebView2 user-data folder so `process_infos` only sees our own processes. Budgets use Private Bytes
  / private working set (D-15.1).

## Traps
- Placement has one rule (`ice_core::placement` via `overlay_win::notch_origin`) for the WebView notch and the native pill; never compute a notch position anywhere else.
- `reduce_motion_enabled` reads the Windows setting; the UI must honour it as well as `prefers-reduced-motion` (§5).
- `fullscreen.rs` is 585 lines; split before adding features (§11.2).

## Tests
Unit tests for geometry/coverage logic in each file; real displays are covered by [manual-checklist](../qa/manual-checklist.md) (sections on placement and gaming mode)
and `examples/fullscreen_spike.rs`.

## Change log
- 2026-11-28 Claude: `autostart::StartupEntry` / `ICE_STARTUP`, `enabled_as` / `set_enabled_as` (Pandora's own Run value); Ice's behaviour unchanged.
- 2026-11-23 Claude: initial recap.
- 2026-11-25 Claude: `overlay_win::set_capture_excluded` (`SetWindowDisplayAffinity`, `WDA_EXCLUDEFROMCAPTURE` / `WDA_NONE`) for D-27 hibernate.
- 2026-11-25 Claude: `overlay_win::ignores_cursor` (both `WS_EX_TRANSPARENT` and `WS_EX_LAYERED` set); `set_click_through` documented as for already-layered windows only (the native pill).
- 2026-11-25 Claude: `move_top_center` → `notch_origin` / `move_notch` (D-20 anchors; work area, so a top taskbar is no longer covered).
- 2026-11-25 Claude: D-20 drag & snap helpers in `overlay_win.rs` (`canvas_origin`, `move_window_async`, `primary_button_down`, `escape_down`, `drag_threshold`) and `drag_ghosts.rs` (`DragGhosts` layered slot overlays on the drag thread).
- 2026-11-25 Gemini: removed `drag_ghosts.rs` (markless drag); `overlay_win::{notch_origin_at, canvas_origin, move_notch}` accept `(anchor, x_ratio, style)` with flush bottom support for `NotchStyle::Nook`.
- 2026-11-27 Claude: `music_service::open` refuses `MusicServiceId::Pandora` (no shell app; the app shell opens it through the companion bridge).
