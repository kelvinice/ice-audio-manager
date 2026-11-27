# Pandora desktop player

> Paths include `pandora/src-tauri/`, `pandora/ui/`, and the shared desktop artwork in `assets/src/`.

## Appearance

Pandora saves its theme with the other player preferences in `library.json`. Rust owns the default and
deserializes older settings. The Svelte shell applies the selection to the document so the library, player,
overlays and settings share one palette. Use Windows follows the system color scheme live. The album artwork
switch controls accent colors independently. Only the most recent artwork decode can supply those colors.

The palette definitions are in `pandora/ui/src/palettes.css`. Settings previews use the same tokens as the
window. The Windows icon family and the sidebar mark share the Pandora diamond. Regenerate the native assets
with the script documented in [asset credits](../../assets/CREDITS.md).

## Ice companion bridge

`src-tauri/src/companion.rs` lets Ice drive Pandora (experimental): a loopback listener with a per-run token in
`%LOCALAPPDATA%Pandoraridge.json`, answering `ping`, `show` and `recognize` from `ice_core::companion`. Each
connection runs on its own short thread (at most four), because a recognition blocks for about 10 s. `install.json`
is written at every start so Ice can launch Pandora. `--background` keeps the window hidden. The window starts hidden
(`tauri.conf.json`) and `setup` shows it otherwise. `recognize_now` in `main.rs` is the single recognition path for
the Recognize view and for Ice. It refuses a second capture while one runs and emits `library://changed/v1`. A
recognition that Ice asked for also emits `companion://recognized/v1`, so the UI shows a notice. "Launch Ice" reads
Ice's own `install.json` first (`ice_bridge::find_standalone_ice_exe`).

## Checks

The library store test covers migration, theme persistence and reset. The offline demo provides cover art
and lyrics for visual checks. See the [Pandora guide](../pandora.md) for launch commands.

## Change log

- 2026-11-27 · Added theme presets, appearance previews and size-specific Windows icons.
- 2026-11-27 · Ice companion bridge (`companion.rs`), `--background` start with a hidden-by-default window, one guarded `recognize_now`, `library://changed/v1` + `companion://recognized/v1` UI events, and Ice's `install.json` in "Launch Ice".
