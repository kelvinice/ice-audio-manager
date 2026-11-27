# Pandora, Ice and song recognition

> Paths: `pandora/src-tauri/src/{ice_bridge,companion,recognition}.rs`, components `IceHubView`, `RecognizeView` ·
> Part of [Pandora](README.md). The user-level story (modes, launching each other, notch widget) is in
> [docs/pandora.md](../../pandora.md).

## Pandora → Ice (`ice_bridge.rs`, `IceHubView`)

- **Embedded:** no Ice running → Pandora runs `ice_win::audio_engine::AudioEngine` in process (devices, volume, mic,
  spatial, hotkeys, call mode, 14-band meter).
- **Connected:** Ice running (probed with `OpenMutexW` on `Local\IceAudioManager.Desktop`, closed at once, never held)
  → the embedded engine stops so Ice keeps hotkeys, call mode and the notch. Commands go over Ice's loopback bridge
  (`%LOCALAPPDATA%\IceAudioManager\bridge.json`), or for older Ice, direct `ice_win` calls plus `config.toml` edits.
- `ice_launch_standalone` stops the embedded engine, then starts Ice found via Ice's `install.json` (then
  `target\{release,debug}`; `find_standalone_ice_exe`). The mode preference is `library.iceModePref`.
- Events `ice://status/v1`, `ice://snapshot/v1`, `ice://meter/v1` (meter only while `ice_set_meter_active`).

## Ice → Pandora (`companion.rs`, experimental)

A loopback listener with a per-run token in `%LOCALAPPDATA%\Pandora\bridge.json` (removed on exit), protocol in
`ice_core::companion`: `ping`, `show`, `recognize`. One short thread per connection, at most four, since a recognition
blocks ~10 s. `install.json` is written at every start so Ice can launch Pandora (`--background` keeps it in the tray).
A recognition Ice asked for emits `companion://recognized/v1` so the UI shows a notice.

## Recognition (`recognition.rs`, `RecognizeView`)

WASAPI capture of system playback (loopback) or the microphone, a pure-Rust Shazam-compatible fingerprint (2048-point
FFT), then the Shazam discovery lookup. `recognize_now` in `main.rs` is the only path (Recognize view and Ice): it
refuses a second capture while one runs, saves the match in `recognitionHistory` and emits `library://changed/v1`.
Status streams on `recognition://status/v1`; `cancel_recognition` stops it. Matches can be played on YouTube Music,
liked or added to a playlist.
