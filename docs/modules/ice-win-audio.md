# ice-win audio engine

> Paths: `crates/ice-win/src/{audio_engine.rs, audio_engine/*, endpoints, notify, volume, policy, spatial, props, hotkeys, file_watch, meter, com}.rs` · Owner: Claude · Verified: 2026-11-23

## Purpose
The real `ice_core::engine::Engine` on Core Audio. It owns device enumeration, default-device switching, volume/mic state,
global hotkeys, per-device spatial sound and `config.toml` hot reload. The only `unsafe` for audio lives here, each block with a
`SAFETY:` comment. It never installs hooks (§1.1) and never blocks in a COM callback.

## Files
| File | What it does |
|---|---|
| `audio_engine.rs` | `AudioEngine::start(ConfigStore)`; the `ice-audio` worker (`Worker`), `Msg` channel, subscribers, snapshot, debounce constants |
| `audio_engine/devices.rs` | Enumerate → `DeviceSummary`, volume and mic state, snapshot device fields, `disconnected` list |
| `audio_engine/switching.rs` | `command()` dispatch, `follow` (resolve saved ref), `cycle` favourites, `switch` (set → re-read defaults → confirm → re-link), `apply_spatial` |
| `audio_engine/hotkeys.rs` | Registers bindings, toggle pairs, favourites cycle and F31 knob gestures with `HotkeyService` |
| `audio_engine/settings.rs` | Hot reload, re-link persistence, safe saves, settings commands (calls `Config::apply_setting`) |
| `endpoints.rs` | MMDevice enumeration → `EndpointInfo` (IDs, names, form factor, container, devnode, stable ID) |
| `notify.rs` | `IMMNotificationClient` → `AudioEvent` into a caller-supplied non-blocking sink (`register`) |
| `volume.rs` | Endpoint master volume get/set/mute and `VolumeWatch` (`IAudioEndpointVolumeCallback`): how hardware-key changes arrive without a keyboard hook |
| `policy.rs` | `set_default(endpoint_id, roles)` via **undocumented** `IPolicyConfig`; per-role result list; isolated and fail-soft (§1.6) |
| `spatial.rs` | Spatial format per endpoint via WinRT `SpatialAudioDeviceConfiguration` (`status`, `selected`, `active`, `set_default_within`); bounded writes |
| `props.rs` | Property-store helpers and the property keys we read (`FRIENDLY_NAME`, `CONTAINER_ID`, `ENDPOINT_DEVNODE`, `FORM_FACTOR`, `STABLE_ID`…) |
| `hotkeys.rs` | `HotkeyService`: `RegisterHotKey` on its own message-loop thread (thread-level, no window, no hook) |
| `file_watch.rs` | `DirWatch`: event-driven directory change notification for config reload (no polling) |
| `meter.rs` | `MeterHub`: `IAudioMeterInformation` peaks + optional [`loopback::LoopbackCapture`] 14-band spectrum on one `ice-meter` thread (runs only while subscribed; automatic peak-only fallback and rebind reset) |
| `loopback.rs` | `LoopbackCapture`: shared-mode WASAPI render loopback (`AUDCLNT_STREAMFLAGS_LOOPBACK`, 100 ms buffer) drained non-blockingly on the 30 Hz `ice-meter` tick into `ice_core::spectrum::SpectrumAnalyzer` (§1.9 in-memory reduction only) |
| `com.rs` | `ComGuard` (MTA init) |
| `call_mode.rs` | D-30 `CallModeService`: the `ice-call` thread (exists only while call mode is on), the dip (lower the now-playing app's session by `Ducker`'s gain, user change wins), and the crash-restore file `%LOCALAPPDATA%\IceAudioManager\call-mode-restore.json` |
| `call_mode/sessions.rs` | `SessionWatch` / `Session`: `IAudioSessionManager2` on every active playback device, "session created" + per-call-app "state changed" notifications (signal only), session peak (`IAudioMeterInformation`) and volume (`ISimpleAudioVolume`, own event-context GUID), owner exe via `proc_metrics::process_exe_name` |

## How it works
- **One `ice-audio` thread (MTA)** owns every COM object. Everything else posts a `Msg`: `Command`, `Audio(AudioEvent)`, `Mic`,
  `Hotkey(id)`, `ConfigChanged`, `Media(MediaUpdate)`, `Quit` — except `Command::Media`, which fast-paths straight to `ice-media-cmd` via `MediaCommandSender` so media skips/seeks never wait behind device refreshes or spatial writes.
- **Debounce:** device-change bursts (one unplug ≈ state + 3 default changes + dozens of property changes) collapse into one refresh:
  sliding 120 ms quiet, never later than 600 ms after the first event. Config reload waits 300 ms after the last write.
- **"Current playback device" = the Multimedia default** (what media plays on, what volume events follow). `RoleSet::Default` =
  Console + Multimedia; `Both` = all three roles. Defaults are **re-read after every switch**, and partial role failures become `SwitchFailed`.
- **Switch flow:** resolve target (`ice_core::resolve`) → `policy::set_default` → re-read defaults → emit `Switched` → persist re-link if the
  endpoint ID changed → apply per-device spatial preference (read-before-write, read-back; failure = `SpatialUnavailable`, the switch still stands).
- **Subscribers** get a `sync_channel(1024)`. One that falls behind is dropped and must re-subscribe + re-snapshot (§6, ENG-12).
  Persistent problems live in `snapshot.notices`; nothing is replayed.
- **Spatial writes** are bounded to 3 s (`SPATIAL_WRITE_LIMIT`) so a hung provider can't stall the thread (ENG-02).
- **Meter:** `Engine::meter()` hands out a `MeterSubscription`; the `ice-meter` thread starts on the first and exits after the last is dropped,
  re-binding when the default device changes.
- **Mic:** the communications microphone's volume/mute is watched; Alt+knob changes mic volume in 2% steps.
- **Call mode (D-30):** `Worker::sync_call_mode` starts `CallModeService` when `call_mode.enabled` turns on and drops it when
  it turns off (the drop restores the music, then joins). The worker forwards the now-playing `app_id` (only when it changes)
  and device refreshes. On `ice-call`: blocks on its channel while no call app plays; while one does (or the music is still
  lowered) it reads the call sessions' peaks every `CALL_TICK` (66 ms) into `ice_core::call_mode::Ducker` and writes
  `base × gain` to the now-playing app's sessions. Stop condition: no active call session and the ducker at rest; "active"
  is re-read every tick, so a missed notification can't keep it polling. Each wake drains the whole queue (one re-scan per
  burst) and the tick runs whenever it's due, so messages never delay it. A level changed elsewhere leaves the dip; the
  next talk onset takes the app in again. Music-app and device changes release the dip without resetting the ducker.
- **Art cache:** the last 8 PNGs (`ART_CACHE`) serve `Engine::art`.

## Traps
- COM callbacks must **copy and return**: no calls back into the same COM objects (deadlock). Re-acquire interfaces after
  `AUDCLNT_E_DEVICE_INVALIDATED`.
- Resolve against **Active** endpoints only: Windows keeps NotPresent ghosts with identical names (D-16).
- Hotkeys: thread-level registration is safe only because that thread never runs a modal loop (ENG-01).
- Conflicts (`HotkeyConflict`) are reported as notices, never silent.
- Show-disconnected lists only *Unplugged* endpoints, never ghosts or disabled ones, and they are never switch targets.
- New `Command` handling goes through `switching.rs::command` (actions) or `settings.rs` (settings).
- Call mode: `RegisterSessionNotification` only fires after a first `GetSessionEnumerator` (done in `register_manager`).
  Session volumes persist in Windows per app, so a dip must always end with `release_dip` / `end_dip` (off, quit, device
  change, music app change); the restore file covers crashes. YouTube Music in a browser lowers that whole browser.
  Another app writing the same session (Discord's Attenuation) shows up as an outside change; keep only one of them on.
  Real-host `#[ignore]` probes in `sessions.rs`: `real_sessions_list`, `real_call_peaks`, `real_notify_rate`,
  `real_watch_dip` (read-only) and `real_spotify_echo` (writes Spotify's level for ~4 s, then restores it).

## Tests
Unit tests sit in each file; real-device paths use `#[ignore]` tests and the [manual QA checklist](../qa/manual-checklist.md)
(sections B, C, E). Spike evidence: [phase0-audio-media](../spikes/phase0-audio-media.md). Reviews: [engine-review](../reviews/engine-review.md).

## Change log
- 2026-11-23 Claude: initial recap.
- 2026-11-24 Gemini: D-26 `loopback.rs` shared-mode WASAPI render loopback + `MeterHub::set_spectrum_enabled` (rebind state reset, DRM/silent loopback auto-fallback, Privacy switch wiring).
- 2026-11-24 Gemini: `AudioEngine::send` fast-paths `Command::Media` straight to `MediaService`'s `MediaCommandSender` (`ice-media-cmd`).
- 2026-11-25 Claude: hot reload and `sync_with_disk` go through `Worker::adopt_disk_config`, which keeps the live D-27 hibernate value (`Config::keep_session_state`); `edit_settings` still uses `set_config` directly. `SetHibernateLook` is a plain setting.
- 2026-11-25 Claude: `summary_basic` fills `DeviceSummary.lyrics_delay_ms` from `Config::lyrics_delay_for` (playback only); `SetLyricsDelay` is a plain setting.
- 2026-11-25 Claude: D-30 call mode: `call_mode.rs` + `call_mode/sessions.rs` (`ice-call` thread, session watch, dip, restore file), `Worker::sync_call_mode`, `proc_metrics::process_exe_name`; `SetCallMode` is a plain setting.
- 2026-11-25 Claude: call mode glitch fixes: queue drained per wake (no tick starvation), `call_active` re-read per tick, re-dip on talk onset after an outside change, `release_dip` keeps the ducker on app/device change, restore file follows the dip; real-host probes.
