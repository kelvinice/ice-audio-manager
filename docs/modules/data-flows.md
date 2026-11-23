# Data flows (cross-layer traces)

> Verified: 2026-11-23 · Update a trace when a hop changes. Module details live in the per-module docs.

## 1. Hotkey switch (Speakers ↔ Arctis)
1. `HotkeyService` (`RegisterHotKey`, own thread) → `Msg::Hotkey(id)` to `ice-audio`.
2. `audio_engine/hotkeys.rs` maps the id to a toggle pair → `ice_core::resolve::toggle` picks the other side using each `DeviceRef` (Active endpoints only).
3. `switching.rs::switch`: `policy::set_default` per role → re-read defaults → confirm.
4. Persist a re-link if the endpoint ID changed (safe save to `config.toml`) → apply the device's spatial preference (read-back).
5. Emit `Switched { relinked, spatial_applied }` (+ `DevicesChanged`, maybe `SettingsChanged`) as `Envelope`s with a new `revision`.
6. Shell `engine_bridge::follow` → `engine://delta/v1` to the notch and settings windows, updates `NotchSnapshot`, the tray tooltip and the native pill.
7. UI scheduler turns `Switched` into a `switch` activity (caption "re-linked", Atmos badge only after confirmation).
Failures: unresolved → `Notice::NotConnected`; ambiguous → `ChooseDevice` (tray picker → `ResolveChoice`); partial roles → `SwitchFailed`.

## 2. Volume key / knob / other app changes volume
1. Windows → `IAudioEndpointVolumeCallback` (`volume::VolumeWatch`) → `AudioEvent::Volume` into the engine channel (callback only copies and returns).
2. Engine updates `snapshot.volume`, emits `VolumeChanged`.
3. Shell projects it into `NotchSnapshot` and the native pill; UI shows the `volume` activity (merged/bumped if already showing).
No keyboard hook is involved at any point (§1.1). Ctrl/Shift/Alt + knob gestures (F31) arrive as registered hotkey chords instead.

## 3. User action from the notch (scroll, click, middle-click)
UI controller → Tauri command (`adjust_volume`, `toggle_mute`, `trigger_switch`, `switch_to_device`, `media_command`…) → shell builds a `Command` →
`EngineSlot::send` → engine acts → event flow as above. The UI never changes its own state optimistically beyond springs.

## 4. Now playing
1. GSMTC event → `MediaWatcher` sink → dirty bit on `ice-media`; 60 ms settle; one read (track, status, timeline, art ≤ 128 px).
2. `MediaUpdate` → `ice-audio` merges into `snapshot.media`, caches the PNG, emits `MediaChanged`.
3. Shell: `TrackKey` decides if it is a *new* track (wakes/resumes the notch); `iceart://<id>` serves the PNG (`Engine::art`).
4. UI: `track` activity, idle "now playing" state, expanded media card; accent/glow from `art.accent`; backdrop from `art` via `backdrop.ts`.
5. Visualizer: UI `set_visualizer(true)` → shell holds `Engine::meter()` subscription → `ice-meter` thread reads peaks (≤ 30 Hz) → `audio://meter/v1`.
   Shell drops the subscription on hide, suspend, WebView release, gaming mode, Lite.

## 5. WebView lifecycle (D-15)
`Placement::allows_webview()` (profile ∈ Instant/Balanced and not gaming) gates everything.
- **Instant:** WebView created at start, hidden, `TrySuspend` after idle linger; native pill shows the idle state.
- **Balanced:** WebView created when an event needs it (cold start ≤ 250 ms target), released after linger.
- **Lite / gaming:** no WebView; native pill + tray only. Entering gaming mode (fullscreen covers the notch's monitor) releases it and bumps `generation`.
- Waking: shell resumes/creates the WebView → UI registers listeners, fetches snapshots, renders from the snapshot (no replay of old activities).

## 6. Settings change (tray today, Settings window in Phase 4)
Tray item → `Command::Set…` → `Config::apply_setting` (validate) → atomic save → `SettingsChanged` (or `Notice::SettingRejected`) → shell/tray/UI/native pill update.
Hand-editing `config.toml` is picked up by `DirWatch` (300 ms debounce): valid → applied; invalid → last valid config kept + `ConfigInvalid`.

## 7. Startup and recovery
Single-instance guard → logging → `EngineSlot` empty ("Starting Windows audio…") → `retry_start` with 1 s → 30 s backoff → engine ready → snapshot → `follow` loop.
The tray is up before the engine. If a subscriber lags (buffer 1024) the engine drops it and the shell re-subscribes and re-snapshots.
