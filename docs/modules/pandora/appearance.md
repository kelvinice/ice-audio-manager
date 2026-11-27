# Pandora appearance and shell

> Paths: `pandora/ui/src/{App.svelte,app.css,palettes.css}`, `lib/theme.ts`, components `TitleBar`, `Sidebar`,
> `ThemePicker`, `SettingsView`, `PageHero`, `MoreMenu`, `ConfirmModal`, `Icon` · Part of [Pandora](README.md)

## Themes and accents

- `settings.theme`: Midnight (default), Ice, Light Ice (`iceLight`, Ice's `ice_light` palette), Daylight, Ember, Use
  Windows (follows the system scheme live). Light themes (Daylight, Light Ice) blend backdrop orbs with `multiply`. `App.svelte`
  sets `data-theme` on the root; `palettes.css` defines every `--p-*` token with `light-dark()`.
- Accent: `--p-tone` → `--p-accent`, `--p-accent-bright`, `--p-accent-soft`, `--p-glow-1`, `--p-surface-tint`. It is the
  theme's `--p-seed` unless **Album artwork accents** (`settings.artworkAccents`, off by default) is on; then
  `setArtworkAccents` puts `data-artwork-accents` on the root and `--p-tone` follows `--art-primary`.
- Cover colours: `applyMaterialThemeFromArt` (`lib/theme.ts`) decodes the cover through the `/art` proxy into
  `--art-primary/secondary/tertiary` on every track change; only the latest decode may write them. The Now Playing
  backdrop always uses them.
- `ThemePicker` previews use the same tokens with the `--art-*` values cleared.
- Liked hearts use `--p-love`, apart from the accent. Use tokens in components, never colour literals.

## Shell

- `TitleBar`: native frame off (`decorations: false`). App menu (incl. Quit), Back / Forward over the route history in
  `App.svelte` (also Alt+Left/Right and mouse buttons 4/5), Home, search, Identify, the Ice snowflake (`ice_wake_and_peek`
  starts Ice and peeks its notch for 2 s), minimize / maximize / close. Empty space is `data-tauri-drag-region`.
- Close-to-tray (`closeToTray`): closing hides the window; Quit really exits.
- `Sidebar`: navigation panel and "Your Library" panel; optional Ice audio card (`showIceCardInSidebar`).
- `SettingsView` sections: Appearance & Lyrics, Playback, Ice Audio Bridge, Storage (`StorageSettings`). The tabs are a
  segmented control (icons, a sliding highlight, arrow keys); the chosen section slides in from the side you moved
  towards. Library tab content fades in on change. Both animations are off with reduced motion.

## Building blocks

Buttons in `app.css`: solid accent `btn-primary` / `btn-play`, translucent pill `btn-ghost`, bare `btn-icon` with
`.solid` (dark circle) for the one key control in a spot. Closing uses icons, not "Close" labels. `PageHero` is the
playlist / album header; `MoreMenu` is every "…" menu (`checked`, `hint`, `heading`, `danger`). Font: Figtree
(bundled, OFL-1.1), with Segoe UI / Yu Gothic UI / Malgun Gothic for other scripts. Windows icons and the sidebar mark
share the Pandora diamond; regenerate them as described in [asset credits](../../../assets/CREDITS.md).
