# app shell (Tauri host)

> Paths: `app/src-tauri/**` (tray has its own doc: [app-tray](app-tray.md)) · Owner: Codex · Verified: 2026-11-23

## Purpose
Wires everything together: starts the engine, owns the notch and settings windows, decides when the WebView exists, forwards engine
state to the UI, and exposes a small set of Tauri commands. Thin by rule: validation and glue only, no business logic (§6).
`#![forbid(unsafe_code)]`.

## Files
| File | What it does |
|---|---|
| `main.rs` | **1386 lines (over limit, split planned).** `SharedRuntime` (all shared state), `#[tauri::command]` handlers, `create_notch_window`, idle suspend/resume, cursor poll, **`--bench` benchmark harness**, `run()`/`main()` |
| `engine_bridge.rs` | `EngineSlot` (engine or "starting" error), `start_audio` (config at `%APPDATA%\IceAudioManager\config.toml`), `retry_start` (1 s → 30 s backoff), `follow` (subscribe → snapshot → deltas, re-snapshot on gap), `project` (engine snapshot → `NotchSnapshot`), `TrackKey`, `watch` |
| `settings.rs` | Settings window (920×640, min 720×480, transparent over Mica, title bar follows light/dark via `WindowEvent::ThemeChanged`) and its commands: `apply_setting` (only `Command::is_setting`), `get_launch_at_sign_in`, `set_launch_at_sign_in` |
| `startup.rs` | Launch at sign-in, shared by the tray and Settings; reads the registry each time (no cached copy) |
| `ipc.rs` | `NotchSnapshot`, `NotchDelta`, `ActivityKind`, `MonitorSummary` (index, id, `display_path`, `name`, position, size, scale, Hz), `HitRect` (TS bindings generated into `app/ui/src/lib/ipc/types.ts`) and the `open_music_service` command (notch window only; runs `ice_win::music_service::open` on a blocking task) |
| `placement.rs` | `Placement` (profile, pinned monitor, coverage, `gaming`, `generation`), `watch` (fullscreen + display changes), `reconcile` (apply profile/monitor/gaming to windows and the native pill) |
| `meter.rs` | `Visualizer`: the shell owns the `audio://meter/v1` subscription; the WebView only *requests* frames |
| `art.rs` | `iceart://<id>` handler: engine PNGs only, no paths/queries/other methods |
| `logging.rs` | File logs in `%LOCALAPPDATA%\IceAudioManager\logs` (`--verbose` for debug) |
| `tray.rs`, `tray/preferences.rs` | See [app-tray](app-tray.md) |
| `capabilities/{default,notch,settings}.json`, `permissions/shell.toml`, `tauri.conf.json` | Least-privilege per window; CSP: `default-src 'self'`, `img-src … iceart:` , no remote origins |

## Events and commands
- **Events to the WebView:** `engine://snapshot/v1`, `engine://delta/v1` (raw `Envelope`), `notch://delta/v1` (shell's own `NotchSnapshot`, monotonic `revision`),
  `audio://meter/v1` (`MeterFrame`). Sent with `emit_to("notch"|"settings", …)`.
- **Commands (UI → shell):** `get_notch_snapshot`, `get_engine_snapshot`, `report_hit_rect`, `report_first_paint`, `report_settings_ready`, `report_event_rendered`,
  `adjust_volume`, `toggle_mute`, `toggle_mic_mute`, `trigger_switch`, `switch_to_device`, `set_notch_devices`, `media_command`, `toggle_expand`,
  `request_idle_suspend`, `open_settings`, `set_visualizer`, `apply_setting`, `get_launch_at_sign_in`, `set_launch_at_sign_in` (Settings window only, permission `settings-controls`), `open_music_service`. A new command also needs a
  permission in `permissions/shell.toml` and a capability entry.
- **Flags:** `--fake-engine` (FakeEngine), `--start-minimized`, `--show-notch`, `--verbose`, `--bench` (+ `--sample-secs`, `--out`).

## How it works
- **Startup:** single-instance guard → logging → `EngineSlot` (real engine retried in the background; tray stays up with "Starting Windows audio…") →
  `SharedRuntime` → tray → native pill → `placement::watch` → `engine_bridge::watch` → notch window created on demand.
- **Performance profiles (D-15.1):** `Instant` keeps the WebView warm and `TrySuspend`s when idle; `Balanced` creates it on demand;
  `Lite` never creates it (native pill + tray only). `Placement::allows_webview()` is the single gate; gaming mode releases the WebView and a
  `generation` guard stops late events from re-creating it.
- **Idle suspend:** UI calls `request_idle_suspend` after its linger; the shell hides the notch, shows the native pill, suspends the WebView and bumps `suspend_gen`.
  A wake (`pending_wake_seq`) resumes it and re-sends the snapshot. A recreated/woken WebView rebuilds from the snapshot, never replaying expired activities.
- **Native pill:** `NativePillSubscription`/`native_dot_snapshot` keep it in sync from the same engine stream; pill input events (`PillInput`) wake or expand the notch.
- **Cursor poll:** bounded (≤ 30 Hz), runs only while the cursor is near the pill, toggles click-through. D-21.1 L2 plans to replace it with event-driven hit-testing.
- **Only a *new* track wakes the notch** (`TrackKey`: app + title + artist + album); art/position/status updates do not.

## Traps
- Never keep logic here that the app needs while the WebView is released; the engine, tray, hotkeys and native pill must work alone.
- Every `update_and_emit` bumps `NotchSnapshot.revision`; the UI drops older ones.
- `iceart://` must stay in the production CSP and serve nothing but engine-cached IDs.
- `main.rs` is a hotspot for conflicts: keep edits small and rebase right before merging (plan.md parallel rules).

## Tests
Unit tests in `art.rs`, `engine_bridge.rs`, `ipc.rs`, `meter.rs`, `placement.rs`, `tray.rs`. Windows/GUI behaviour: [manual-checklist](../qa/manual-checklist.md). Review: [shell-review](../reviews/shell-review.md).

## Change log
- 2026-11-23 Claude: initial recap.
- 2026-11-23 Claude: Settings window moved to `settings.rs` (resizable, Mica, live theme) with `apply_setting` + launch-at-sign-in commands; `startup.rs`; `MonitorSummary` gains display path, name and position.
