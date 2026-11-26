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

## Checks

The library store test covers migration, theme persistence and reset. The offline demo provides cover art
and lyrics for visual checks. See the [Pandora guide](../pandora.md) for launch commands.

## Change log

- 2026-11-27 · Added theme presets, appearance previews and size-specific Windows icons.
