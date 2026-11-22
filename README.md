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

Run each line on its own (no `;` chaining, which cmd doesn't support).

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

Left-click the tray icon to choose a playback or recording device. Right-click for Settings,
the notch preview, launch at sign-in, and Quit. A normal launch stays in the tray, and a
second launch exits without creating another instance.

This build uses the fake engine. Device and volume changes are simulated in the app.
Use `--verbose` for debug messages in `%LOCALAPPDATA%\IceAudioManager\logs`.
The existing `--bench` mode writes its report to `app/out/bench.json` by default.

## License

[MIT](LICENSE)
