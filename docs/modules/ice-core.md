# ice-core

> Paths: `crates/ice-core/**` · Owner: Claude · Verified: 2026-11-23

## Purpose
Pure domain logic and the **engine contract** that every other layer talks through. No Windows, no Tauri, no `unsafe`
(`#![forbid(unsafe_code)]`). Everything here is unit-testable on any machine. Cargo feature `ts` derives `ts_rs::TS` on the
contract types, and `cargo test -p ice-core --all-features` exports them to `app/ui/src/lib/ipc/engine.ts` (camelCase,
`type`-tagged unions; never hand-edit that file).

## Files
| File | What it does |
|---|---|
| `engine.rs` | The contract: `Engine` trait, `Command`, `EngineEvent`, `Envelope`, `EngineSnapshot`, `Notice`, `SettingsView`, `RoleSet`, `PerformanceProfile`, `SpatialMode`, `MediaAction` |
| `engine/fake.rs` | `FakeEngine`: in-memory engine with the owner's devices, three fake Spotify tracks and stereo meter levels. Used by `--fake-engine`, UI work and tests |
| `config.rs` | `config.toml` schema (v4), `Config`, `ConfigStore` (load/save/read), `Config::apply_setting` (every settings command), `Config::view` → `SettingsView` |
| `resolve.rs` | D-16 device identity: `DeviceRef`, `MatchMode`, `resolve`, `candidates`, `toggle`, `normalize_name` |
| `device.rs` | `EndpointInfo`, `Flow` (render/capture), `Role`, `DeviceState`, `hardware_key_from_devnode` |
| `event.rs` | `AudioEvent`: what COM callbacks push into a channel (device added/removed/state/default/property, volume) |
| `hotkey.rs` | `Chord`, `Mods`, `ChordError`: parse and display chords like `Ctrl+Alt+F12` (no Windows calls) |
| `appearance.rs` | `Appearance { theme, preset, idle_style }` with `ThemeKind`, `SpringPreset`, `IdleStyle` (shared by tray, WebView, native pill) |
| `media.rs` | `NowPlaying`, `ArtInfo`, `PlaybackStatus`, `MusicServiceId` (Spotify, YouTube Music), `Rgb`, dominant colour, `readable_accent` (WCAG AA ≥ 4.5:1 on `ICE_SURFACE`), `art_id`/`content_id` |
| `meter.rs` | `MeterFrame { peak, channels }`, `channel()` → (`MeterSink`, `MeterSubscription`), `METER_INTERVAL` = 33 ms |

## The contract (what other modules use)
- **`Engine` trait:** `snapshot()`, `send(Command)` (fire-and-forget; results arrive as events), `subscribe()` → `Receiver<Envelope>`
  (subscribe **before** reading the snapshot), `art(id)` → PNG bytes of the last 8 tracks, `meter()` → `MeterSubscription`.
- **`Command`:** actions (`SwitchTo`, `SwitchBinding`, `TogglePair`, `CycleFavourites`, `SetVolume`, `StepVolume`, `ToggleMute`,
  `ToggleMicMute`, `Media`, `SetSpatial`, `ResolveChoice`, `Refresh`) and settings (`SetTogglePair`/`Remove…`, `SetBinding`/`Remove…`,
  `SetFavourites`, `SetDevicePrefs`, `SetTheme`, `SetPreset`, `SetIdleStyle`, `SetPerformanceProfile`, `SetPinnedMonitor`,
  `SetKnobGestures`, `SetShowDisconnected`, `SetDefaultRoles`). Settings results: `SettingsChanged` or `Notice::SettingRejected`.
- **`EngineEvent`:** `DevicesChanged`, `VolumeChanged`, `MicMuteChanged`, `MicVolumeChanged`, `MediaChanged`, `Switched`
  (`relinked`, `spatial_applied`), `DeviceArrived`, `DeviceLeft`, `Notice`, `NoticesChanged`, `SettingsChanged`.
  Every delta is wrapped in `Envelope { revision, event }`.
- **`Notice`:** `NotConnected`, `ChooseDevice` (picker; answer with `ResolveChoice`), `SwitchFailed`, `SpatialUnavailable`,
  `HotkeyConflict`, `ConfigReplaced`, `ConfigInvalid`, `ConfigSaveFailed`, `SettingRejected`.
- **`EngineSnapshot`:** `revision`, `playback`/`recording` (`DeviceSummary`), `disconnected`, `volume`, `mic_muted`,
  `mic_volume`, `media`, `notices`, `settings` (`SettingsView`).

## How it works
- **Snapshot + delta (D-03):** one monotonic `revision` on every snapshot and delta. Consumers apply only newer revisions in
  order and re-read the snapshot on a gap.
- **Identity (D-16):** bindings store a `DeviceRef`, not an index or name. `resolve` tries endpoint ID → stable ID →
  (`MatchMode::SameModel` only) hardware key/container + same logical endpoint. **Only Active endpoints are candidates.**
  Ambiguous → `ChooseDevice`; missing → `NotConnected`. After a confirmed switch the engine re-links the saved ref.
- **Config:** every field has a default; `deny_unknown_fields`; unknown future `schema_version` is rejected. Bad file → backed
  up as `config.invalid-<ts>.toml` and defaults used (see `Loaded`). Saves are atomic with lock retries.
- **Meter:** levels never enter the snapshot or revisions. `MeterSink::offer` sends only changed frames (silence once) and reports
  when the subscription is closed, which is the engine's stop condition.

## Traps
- A new `Command`/`Notice`/`EngineEvent` variant must also be handled in `fake.rs`, the real engine (`ice-win-audio`), the tray
  (`app-tray`) and the UI scheduler (`app-ui-notch`). Grep the variant name in all four.
- Adding a config field means: default, validation in `apply_setting`, `SettingsView`, **schema bump + migration test** (code-practices §7).
- `percent` is 0–100, `scalar` is 0.0–1.0 (§11.2). Core Audio mixes both.
- Windows renames re-installed devices ("2- Arctis Nova 7"); `normalize_name` strips that prefix. Never key on names alone.
- `ts-rs` types for `u64` need `ts(type = "number")` (see `Envelope.revision`).

## Tests
Inline `#[cfg(test)]` in each file. `resolve.rs` is built from the owner's real endpoints (incl. new-USB-port case, QA B3/B4).
`config.rs` holds the migration fixtures (v2 → v4). Run: `cargo test -p ice-core --all-features`.

## Change log
- 2026-11-23 Claude: initial recap.
