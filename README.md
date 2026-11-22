# Ice Audio Manager

A lightweight Windows 11 audio switcher with an animated, Dynamic-Island-style notch.

> **Status: Phase 0 (spikes).** The previous Python/PyQt5 version is preserved at the git tag
> `legacy-python-v0`. Planning lives in [`docs/`](docs/):
> [discussion & decisions](docs/planning-discussion.md) · [implementation plan](docs/plan.md) ·
> [code practices](docs/code-practices.md).

## Layout

```
crates/ice-core   pure Rust domain types and logic (no Windows deps, no unsafe)
crates/ice-win    all Windows FFI: Core Audio, PolicyConfig, spatial sound, media (GSMTC)
spikes/audio      Phase 0.3 CLI: list/watch/switch endpoints, peak meter, spatial sound
spikes/media      Phase 0.4 CLI: now playing via GSMTC (Spotify)
app/              Tauri shell + Svelte UI (added in Phase 0.2 / Phase 1)
```

## Build (Windows 11, from PowerShell, not Git Bash)

```powershell
cargo build --workspace
cargo test --workspace
cargo run -p audio-spike -- list
cargo run -p media-spike -- list
```

## License

[MIT](LICENSE)
