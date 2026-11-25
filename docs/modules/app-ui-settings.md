# app UI: Settings window

> Paths: `app/ui/settings.html`, `app/ui/src/settings/**`, `app/ui/src/lib/settings/**`, `app/ui/src/themes/settings.css` · Owner: Gemini (UI), built by Claude · Verified: 2026-11-23

## Purpose
The Phase 4 Settings window (F19–F21): every setting the engine supports, without editing `config.toml`. A Windows 11
Settings-style layout (navigation rail, page title, cards) drawn over Mica, following Windows' light/dark mode. It owns no state:
each change is a settings `Command`, and the page re-renders from the `SettingsChanged` the engine sends back. The window is
created when opened and destroyed when closed (shell: `app/src-tauri/src/settings.rs`).

## Files
| File | What it does |
|---|---|
| `src/settings/main.ts` | Picks the backend (Tauri, or `devBackend.ts` in a plain browser during `npm run dev`), mounts `SettingsApp` |
| `src/settings/SettingsApp.svelte` | Frame: navigation (spring-driven indicator), page title + "Saved" flash, pinned error banner, page switch (`#page` hash or last page in `localStorage`) |
| `src/settings/pages/*Page.svelte` | General · Devices · Shortcuts · Notch · Hibernate · Activities · Appearance · Privacy · Storage · About |
| `src/settings/pages/DeviceCard.svelte` | One device: star = show in notch, alias, spatial sound, hide |
| `src/settings/pages/ShortcutCard.svelte` | A toggle pair or device shortcut, saved or a new draft (draft stays open if the engine refuses it) |
| `src/settings/controls/*` | `SettingCard` (expander), `Row`, `Toggle`, `Select` (native), `ChoiceGroup` (radio cards with preview), `HotkeyField` (recorder), `KeyCaps`, `InfoBar`, `Section`, `Icon` (Segoe Fluent glyphs, `icons.ts`), `BrandMark` (snowflake SVG) |
| `src/settings/previews/*` | `NotchPreview` (notch drawn with a theme's real `--ice-*` tokens), `SpringDemo` (the notch spring module, on hover), `MonitorMap` (displays laid out like Windows' Display settings) |
| `src/settings/devBackend.ts` | **Dev only**, never bundled: fake devices and a mini settings reducer so the UI runs in a browser |
| `src/lib/settings/client.svelte.ts` | `SettingsClient`: listeners → snapshots (D-03), `apply(command)` → `Promise<saved>`, banners from `SettingRejected` / `HotkeyConflict`, launch at sign-in, version |
| `src/lib/settings/model.ts` | Pure logic: `applyEnvelope`, `deviceLabel`, `savedLabel`, `hiddenDevices`, `moveItem`, `nextId`, `recordKeys` (keydown → `Ctrl+Alt+F12`), `chordParts` |
| `src/themes/settings.css` | `--ice-set-*` tokens (dark + light), reduce-motion rules, shared `.btn` / `.icon-btn` |

## Pages → commands
| Page | Settings | Command |
|---|---|---|
| General | Launch at sign-in; performance profile; default roles; show disconnected | `set_launch_at_sign_in`; `SetPerformanceProfile`; `SetDefaultRoles`; `SetShowDisconnected` |
| Devices | Notch order (up/down/remove); per device: star, alias, spatial, hide; hidden list → Show | `SetFavourites`; `SetDevicePrefs` |
| Shortcuts | Toggle pairs, device shortcuts (devices, chord, roles, remove); knob gestures | `SetTogglePair`/`RemoveTogglePair`; `SetBinding`/`RemoveBinding`; `SetKnobGestures` |
| Notch | Notch style (`Edge Nook` default vs `Floating`); idle style; Now playing (`While playing` default, `Brief, then small`, `Off`); pinned display | `SetNotchStyle`; `SetIdleStyle`; `SetNowPlayingPill`; `SetPinnedMonitor` (display path) |
| Hibernate | Hibernate notch (session only: the app always starts awake); while hibernating: `Plain notch` (default) or `Hide the notch`; hide from screen sharing | `SetHibernate`; `SetHibernateLook`; `SetHibernateHideFromCapture` |
| Appearance | Theme (`Ice`, `Ice Light`, `Midnight`, `Fluent`, `Minimal`); animation preset (`Bouncy` first with `Default` badge); Background → `Moving background` toggle | `SetTheme`; `SetPreset`; `SetMovingBackdrop` |
| Activities | Music services (`Spotify`, `YouTube Music`); Synced lyrics display (`Show synced lyrics`, pill/card extensions, active word highlight, Japanese Romaji) | `SetMusicService`; `SetLyricsEnabled`; `SetLyricsDisplay` |
| Privacy | Master network off switch; opt-in LRCLIB (`lrclib.net`) disclosure & toggle; local audio loopback spectrum visualizer disclosure & meter-only fallback switch | `DisableAllNetworkFeatures`; `SetLyricsOnlineLookup`; `SetVisualizerSpectrum` |
| Storage | Live lyrics cache usage bar, max cache size (`5–500 MB`), clear cache, open local `.lrc` overrides folder | `SetLyricsCacheMaxMb`; `get_lyrics_storage_stats`; `clear_lyrics_cache`; `open_lyrics_folder` |
| About | Version, trust promises | `plugin:app|version` |

## How it works
- **Sync:** listens to `engine://delta/v1`, `engine://snapshot/v1`, `notch://delta/v1` first, then fetches `get_engine_snapshot` +
  `get_notch_snapshot`; buffered deltas apply by revision, a gap re-snapshots. Monitors and the Windows reduce-motion flag come from the notch snapshot.
- **Saving:** `client.apply` sends `apply_setting` (the shell refuses anything that isn't `Command::is_setting`). It resolves `true`
  on `SettingsChanged` (and flashes "Saved"), `false` on `SettingRejected` (banner), and `true` after 1.5 s of silence (an unchanged setting gets no reply).
- **Hidden devices** are absent from the snapshot's device lists, so the Devices page lists them from `settings.devices`.
- **Hotkey recorder:** reads keys typed into this focused window only (no hook, §1.1). Plain letters, digits, arrows and Space need Ctrl/Alt/Win;
  the engine stays the authority. A chord already registered (by us or another app) never reaches the page; the engine reports the conflict.
- **Motion:** the nav indicator and `SpringDemo` use `lib/motion` springs; controls use short CSS transitions. All of it stops when Windows'
  animation effects are off (`data-reduce-motion`) or `prefers-reduced-motion`.

## Traps
- **Never put literal colours in components**; add a `--ice-set-*` token to `settings.css` and `docs/theming.md` §2.1.
- Previews depend on `[data-theme='…']` matching any element (base.css), not only `:root`.
- Spatial choices are a twin of `tray.rs` `SPATIAL_OPTIONS`; change both.
- `Select` is a native `<select>` on purpose (keyboard, screen readers, Windows drop-down). `<option>` colours come from `--ice-set-solid-bg`.
- The WebView2 browser process owns a hidden window with the same title as the page; UI automation must filter by the app's PID.
- Not yet built: user theme files + live preview, import/export, Activities page, live cost readout (L6), favourites cycle hotkey.

## Tests
`src/lib/settings/model.test.ts` (envelope order/gap, chord recording, list moves, ids). Visual QA: `npm run dev`, open
`/settings.html#devices` in a browser (dev backend), light/dark and 100 %/150 %; real window needs the app (tray → Settings).

## Change log
- 2026-11-23 Claude: Settings window built (General, Devices, Shortcuts, Notch, Appearance, About).
- 2026-11-24 Claude: Activities page (Spotify / YouTube Music switches, D-22); Shortcuts → Notch ("show notch" hotkey, D-19); General → "Sleep the notch after" (Balanced); Notch → "Now playing pill".
- 2026-11-24 Gemini: `AppearancePage` adds `Ice Light` theme card, puts `Bouncy` leftmost with `Default` badge, and adds `Background → Moving background` (`SetMovingBackdrop`).
- 2026-11-24 Gemini: added `PrivacyPage` (D-22 network transparency + LRCLIB opt-in), `StoragePage` (lyrics cache usage, `cache_max_mb` selector, clear cache, local `.lrc` folder), and synced lyrics display controls on `ActivitiesPage`.
- 2026-11-24 Gemini: D-26 `PrivacyPage` "Local audio analysis" section with loopback spectrum disclosure and meter-only fallback switch (`SetVisualizerSpectrum`).
- 2026-11-24 Gemini: added "Notch style" (`Floating` / `Edge Nook`) choice group on `NotchPage` and flush concave-ear silhouette support in `NotchPreview`.
- 2026-11-24 Gemini: made `Edge Nook` and `While playing` default on `NotchPage` and renamed "When the song changes" to "Now playing".
- 2026-11-25 Claude: Notch page: "Hibernate" section (Hibernate notch, Hide from screen sharing toggles); `devBackend` handles both commands.
- 2026-11-25 Claude: Hibernate moved to its own **Hibernate** page (`HibernatePage.svelte`, nav after Notch, new `moon` glyph U+E708): hibernate toggle, "While hibernating" choice (`Plain notch` / `Hide the notch`, `SetHibernateLook`), hide from screen sharing; `devBackend` handles `setHibernateLook`.
