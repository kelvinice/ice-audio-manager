# Ice Audio Manager

A lightweight Windows 11 audio switcher with an animated, Dynamic-Island-style notch.

> **Status: Phase 1 (private app shell).** The previous Python/PyQt5 version is preserved at the git tag
> `legacy-python-v0`. Planning lives in [`docs/`](docs/):
> [discussion & decisions](docs/planning-discussion.md) · [implementation plan](docs/plan.md) ·
> [code practices](docs/code-practices.md).

## Layout

```
crates/ice-core   pure Rust domain types and logic (no Windows deps, no unsafe)
crates/ice-win    all Windows FFI: Core Audio, PolicyConfig, spatial sound, media (GSMTC)
spikes/audio      Phase 0.3 CLI: list/watch/switch endpoints, peak meter, spatial sound
spikes/media      Phase 0.4 CLI: now playing via GSMTC (Spotify)
app/              Tauri shell and Svelte notch/Settings preview
```

## Build (Windows 11, cmd or PowerShell, not Git Bash)

**One click:** double-click `build.cmd` (or run it from a terminal):

```
build          release build of the app
build debug    fast debug build
build check    everything CI runs (UI check/tests, fmt, clippy -D warnings, all tests)
build run      release build, then start the app
```

It installs UI packages when needed, finds `cargo` even in terminals opened before Rust was installed, closes a running copy
of the same build first (Windows locks the exe), and stops at the first failing step. The first release build spends
several minutes on the last step (full link-time optimisation on one core); `build debug` is much faster for trying changes.

Or step by step. Run each line on its own (no `;` chaining, which cmd doesn't support).

Once per fresh clone/worktree, because the Tauri crate embeds the built UI (gitignored `ui/dist`):

```
npm --prefix app/ui ci
npm --prefix app/ui run build
```

Build, test, and look around:

```
cargo build --workspace
cargo test --workspace
cargo run -p audio-spike -- list
cargo run -p media-spike -- list
```

Try the real engine with the owner's setup (Ctrl+Alt+F12 toggles Arctis ↔ Speakers). These two don't need the UI build:

```
cargo run -p audio-spike -- make-config
cargo run -p audio-spike -- engine
```

## Run the private shell

```powershell
cargo build --release -p ice-audio-manager
.\target\release\ice-audio-manager.exe --start-minimized
```

Left-click the tray icon and open a playback or recording device to switch to it or pair it with the
current default on **Ctrl+Alt+F12**. Pairing replaces the saved **main** pair. Playback device menus
also let you set a spatial sound preference.
The left-click menu shows saved toggle pairs, volume, active notices, and the **Knob gestures** setting.
If the engine needs help matching a saved device, open **Choose device** to confirm or cancel.

Right-click for Settings, the notch preview, launch at sign-in, and Quit. A normal launch stays in the tray,
and a second launch exits without creating another instance.

The app uses Windows audio and reads `%APPDATA%\IceAudioManager\config.toml`. It uses the default
configuration when the file is absent. If Windows audio is unavailable, the tray shows the error
and retries in the background. Use `--fake-engine` to simulate devices and volume changes.
Use `--verbose` to log engine events in `%LOCALAPPDATA%\IceAudioManager\logs`.
The existing `--bench` mode writes its report to `app/out/bench.json` by default.

The Windows CI run provides an unsigned portable ZIP with the release executable and license.

## License

[MIT](LICENSE)
