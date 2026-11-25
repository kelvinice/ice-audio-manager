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
| `engine_bridge.rs` | `EngineSlot` (engine or "starting" error), `start_audio` (config at `%APPDATA%\IceAudioManager\config.toml`), `retry_start` (1 s → 30 s backoff), `follow` (subscribe → snapshot → deltas, re-snapshot on gap), `project` (engine snapshot → `NotchSnapshot`), `TrackKey`, `watch`; guards closed-window emits (`emit_to_open`) and fast-paths `LyricsChanged` / non-waking `MediaChanged` |
| `settings.rs` | Settings window (1020×680, min 720×480, transparent over Mica, title bar follows light/dark via `WindowEvent::ThemeChanged`) and its commands: `apply_setting` (only `Command::is_setting`), `get_launch_at_sign_in`, `set_launch_at_sign_in`, plus async `spawn_blocking` lyrics storage commands |
| `startup.rs` | Launch at sign-in, shared by the tray and Settings; reads the registry each time (no cached copy) |
| `ipc.rs` | `NotchSnapshot`, `NotchDelta`, `ActivityKind`, `MonitorSummary` (index, id, `display_path`, `name`, position, size, scale, Hz), `HitRect` (TS bindings generated into `app/ui/src/lib/ipc/types.ts`) and the `open_music_service` command (notch window only; runs `ice_win::music_service::open` on a blocking task) |
| `show_notch.rs` | D-19: `show` (hotkey toggles the pinned notch, pill click opens it; Lite → tray quick-switch menu; nothing over a game), `wire_native_pill` |
| `session.rs` | D-21.1 L3: `ice_win::session_watch` → clears the notch pin (`expanded`) on session lock or sleep |
| `placement.rs` | `Placement` (profile, pinned monitor, `anchor`, coverage, `gaming`, `generation`), `watch` (fullscreen + display changes), `reconcile` (apply profile/monitor/anchor/gaming to windows and the native pill, syncing `NotchSnapshot.anchor`) |
| `drag.rs` | D-20 drag & snap loop (`start`, `Host::{WebView, Native}`): bounded 120 Hz worker moving either host via `SWP_NOACTIVATE` + showing `DragGhosts` across all monitors, springing to nearest slot on drop or back on Esc |
| `meter.rs` | `Visualizer`: the shell owns the `audio://meter/v1` subscription and drains queued frames to the latest reading before emitting; the WebView only *requests* frames |
| `art.rs` | `iceart://<id>` handler: engine PNGs only, no paths/queries/other methods |
| `logging.rs` | File logs in `%LOCALAPPDATA%\IceAudioManager\logs` (`--verbose` for debug) |
| `tray.rs`, `tray/preferences.rs` | See [app-tray](app-tray.md) |
| `capabilities/{default,notch,settings}.json`, `permissions/shell.toml`, `tauri.conf.json` | Least-privilege per window; CSP: `default-src 'self'`, `img-src … iceart:` , no remote origins |

## Events and commands
- **Events to the WebView:** `engine://snapshot/v1`, `engine://delta/v1` (raw `Envelope`), `notch://delta/v1` (shell's own `NotchSnapshot`, monotonic `revision`),
  `audio://meter/v1` (`MeterFrame`). Sent with `emit_to("notch"|"settings", …)`.
- **Commands (UI → shell):** `get_notch_snapshot`, `get_engine_snapshot`, `report_hit_rect`, `report_first_paint`, `report_settings_ready`, `report_event_rendered`,
  `adjust_volume`, `set_volume`, `toggle_mute`, `toggle_mic_mute`, `trigger_switch`, `switch_to_device`, `media_command`, `toggle_expand`,
  `request_idle_suspend`, `open_settings`, `set_visualizer`, `set_hibernate`, `start_notch_drag`, `apply_setting`, `get_launch_at_sign_in`, `set_launch_at_sign_in` (Settings window only, permission `settings-controls`), `open_music_service`, `open_language_settings`. A new command also needs a
  permission in `permissions/shell.toml` and a capability entry.
- **Flags:** `--fake-engine` (FakeEngine), `--start-minimized`, `--show-notch`, `--verbose`, `--bench` (+ `--sample-secs`, `--out`).

## How it works
- **Startup:** single-instance guard → logging → `EngineSlot` (real engine retried in the background; tray stays up with "Starting Windows audio…") →
  `SharedRuntime` → tray → native pill → `placement::watch` → `engine_bridge::watch` → notch window created on demand.
- **Performance profiles (D-15.1):** `Instant` keeps the WebView warm and awake at idle (it never hands over to the native pill;
  owner request 2026-11-24, `Placement::suspends_on_idle`); `Balanced` creates it on demand and `TrySuspend`s when idle;
  `Lite` never creates it (native pill + tray only). `Placement::allows_webview()` is the single gate; gaming mode releases the WebView and a
  `generation` guard stops late events from re-creating it.
- **Idle suspend (Balanced only):** UI calls `request_idle_suspend` after its linger; the shell hides the notch, shows the native pill, suspends the WebView and bumps `suspend_gen`.
  A wake (`pending_wake_seq`) resumes it and re-sends the snapshot. A recreated/woken WebView rebuilds from the snapshot, never replaying expired activities.
- **Native pill:** `NativePillSubscription`/`native_dot_snapshot` keep it in sync from the same engine stream; pill input events (`PillInput`) wake or expand the notch.
- **Cursor poll:** bounded (≤ 30 Hz), runs only while the cursor is near the pill, toggles click-through. D-21.1 L2 plans to replace it with event-driven hit-testing.
- **Only a *new* track wakes the notch** (`TrackKey`: app + title + artist + album); art/position/status updates do not.

## Traps
- Cursor poll (`spawn_cursor_poll`): the hit rect is scaled by the notch window's own width / `NOTCH_CANVAS_W`, never a
  stored monitor `scale_factor`. Click-through goes through **tao's `set_ignore_cursor_events`** (`ClickThrough`), never
  hand-set bits: Windows needs `WS_EX_TRANSPARENT | WS_EX_LAYERED`, and tao rewrites the whole extended style from its
  own flags on every show/hide, so only a flag tao knows survives. Each tick compares the real style
  (`overlay_win::ignores_cursor`) and asks tao only on a mismatch. The notch is built `.focusable(false)` so those
  rewrites keep `WS_EX_NOACTIVATE`.
- Never keep logic here that the app needs while the WebView is released; the engine, tray, hotkeys and native pill must work alone.
- Every `update_and_emit` bumps `NotchSnapshot.revision`; the UI drops older ones.
- `iceart://` must stay in the production CSP and serve nothing but engine-cached IDs.
- `main.rs` is a hotspot for conflicts: keep edits small and rebase right before merging (plan.md parallel rules).

## Tests
Unit tests in `art.rs`, `engine_bridge.rs`, `ipc.rs`, `meter.rs`, `placement.rs`, `tray.rs`. Windows/GUI behaviour: [manual-checklist](../qa/manual-checklist.md). Review: [shell-review](../reviews/shell-review.md).

## Change log
- 2026-11-23 Claude: initial recap.
- 2026-11-23 Claude: Settings window moved to `settings.rs` (resizable, Mica, live theme) with `apply_setting` + launch-at-sign-in commands; `startup.rs`; `MonitorSummary` gains display path, name and position.
- 2026-11-24 Claude: `Instant` no longer idle-suspends (`Placement::suspends_on_idle`); switching to Instant wakes a suspended notch; `set_volume` command;
  notch canvas 440×210 (was 420×190) so the drop shadow isn't clipped.
- 2026-11-24 Claude: removed `set_notch_devices` (the notch has no device picker; Settings uses `apply_setting`).
- 2026-11-24 Claude: `session.rs` clears the notch pin on session lock or sleep (`ice_win::session_watch`).
- 2026-11-24 Claude: `show_notch.rs` (D-19): the "show notch" hotkey (`EngineEvent::ShowNotch`) and the native pill click/middle click; `tray::open_quick_menu` is shared with the tray's left click.
- 2026-11-24 Gemini: `settings.rs` default Settings window size increased from `920×640` to `1020×680`.
- 2026-11-24 Gemini: `NOTCH_CANVAS_H` raised to `292` (for bottom lyrics extension); added Settings-only `get_lyrics_storage_stats`, `clear_lyrics_cache`, and `open_lyrics_folder` commands (`permissions/shell.toml`).
- 2026-11-24 Gemini: `meter::forward` drains queued frames to latest, `SharedRuntime::update_and_emit` / `engine_bridge` skip unchanged `NotchSnapshot` bumps and closed-window emits (`LyricsChanged`, non-waking `MediaChanged` fast paths), and `settings.rs` lyrics storage commands run via `spawn_blocking`.
- 2026-11-24 Gemini: `NOTCH_CANVAS_W` raised to `648` for Edge Nook side-by-side layout + concave ears, `NotchSnapshot.notch_style`, and `engine_bridge` `native_dot.set_notch_style` sync.
- 2026-11-25 Claude: cursor poll scales the hit rect from the window width, re-checks `WS_EX_TRANSPARENT` every tick and resets it on exit (fix: clicks around the pill / music pill blocked); UI resends `report_hit_rect` after wake or a failed send.
- 2026-11-25 Claude: D-27 hibernate: `engine_bridge` drops every activity while hibernating (no pop-up, no WebView wake), syncs the native pill (`native_dot_appearance`) and applies capture exclusion to the notch window (`apply_capture_privacy`, also on window creation); the cursor poll treats an empty hit rect as fully click-through.
- 2026-11-25 Claude: `set_hibernate` command (notch-controls permission): sends `Command::SetHibernate` and unpins the card; the only setting the notch window may change.
- 2026-11-25 Claude: click-through fixed for real: the poll set only `WS_EX_TRANSPARENT`, which doesn't pass clicks through a non-layered window, so the whole 648 × 292 canvas (and the hibernating notch) blocked clicks. Now `ClickThrough` drives tao's `set_ignore_cursor_events` (TRANSPARENT + LAYERED, kept in tao's flags); notch window `.focusable(false)`. Verified on the running app with `WindowFromPoint` beside the pill: old build = notch, new build = window below.
- 2026-11-25 Claude: `project` sends `effective_idle_style()` as `NotchSnapshot.idleStyle` (hidden while hibernating with the hidden look) and unpins the card (`expanded = false`) while hibernating, however hibernate was turned on; `set_hibernate` no longer unpins itself. `native_dot_appearance` sets the idle style before `set_plain`. Restored the `…` in "Starting Windows audio…" (mis-encoded in the D-27 commit). `HibernateLook` added to `export_ts_bindings`.
- 2026-11-25 Claude: `open_language_settings` command (settings-controls permission, fixed target `ms-settings:regionlanguage`, no input).
- 2026-11-25 Claude: `engine_bridge::Update::Delta` is boxed too (a media delta grew with the art palette; clippy `large_enum_variant`).
- 2026-11-25 Claude: `Placement.anchor` (from `settings.notch_anchor`; a change repositions both hosts like a repin); every `move_top_center` call is `move_notch`; `NOTCH_CANVAS_W/H` come from `ice_core::placement`.
- 2026-11-25 Claude: `wire_native_pill` handles `PillInput::Wheel`: ±`WHEEL_VOLUME_STEP` (4) % per notch, Shift → next device (`preview_target`), like the WebView notch.
- 2026-11-25 Claude & Gemini: D-20 drag & snap + bottom anchors (`drag.rs`, `start_notch_drag` command, `NotchSnapshot.anchor`, `show_notch::wire_native_pill` `DragStart`, `engine_bridge` `set_drag_enabled`, `spawn_cursor_poll` skips click-through updates while `state.dragging`, expanded-card drag collapses around collapsed pill centre, `placement::reconcile` syncs `view.anchor`).
