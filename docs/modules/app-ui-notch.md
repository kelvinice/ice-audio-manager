# app UI (notch and settings WebViews)

> Paths: `app/ui/**` · Owner: Gemini · Verified: 2026-11-23

## Purpose
Presentation only: Svelte 5 + TypeScript (`strict`). It renders the engine's state as the notch (idle pill, short activities, expanded card
with media and device row) and, later, the Settings window. It owns **no authoritative state**; it can be destroyed and rebuilt from a snapshot at
any time (D-15). Heavy work (image decode, colour extraction, device sorting) happens in Rust.

## Entry points
| File | What it does |
|---|---|
| `index.html` → `src/notch/main.ts` → `notch/NotchApp.svelte` | The notch: a 420×190 transparent canvas holding one clipped pill |
| `settings.html` → `src/settings/main.ts` → `settings/SettingsApp.svelte` | Settings window. **Placeholder today** (reads `notch://delta/v1`, reports `report_settings_ready`); the real pages are Phase 4 |
| `src/themes/base.css` | All `--ice-*` tokens (documented in [theming.md](../theming.md): every new token must be added there) |

## `src/lib/` (logic, no markup)
| File | What it does |
|---|---|
| `ipc/engine.ts` | **Generated** from `ice-core` (never edit; fix the Rust type) |
| `ipc/types.ts` | **Generated** shell types (`NotchSnapshot`, `NotchDelta`, `ActivityKind`, `HitRect`, `MonitorSummary`) |
| `motion/spring.ts` | `Spring1D`: closed-form damped oscillator `(stiffness, damping, mass)`, frame-rate independent (tested at 75/144/240 Hz); `SPRING_PRESETS` snappy/smooth/bouncy/off |
| `motion/spring.svelte.ts` | `SpringGroup`: thin Svelte adapter, drives the loop only while something is still moving |
| `notch/scheduler.ts` | `NotchActivityScheduler` + `ActivityView` (`idle`, `volume`, `switch`, `plug`, `mic`, `track`): priorities, durations, merge rules (plug+switch within 1.5 s), snapshot sync and monotonic envelope handling (`NeedsResnapshot` on gap). **604 lines, at the limit** |
| `notch/controller.svelte.ts` | `NotchController` (`createNotchController`): listens to `engine://*` and `notch://delta/v1`, owns springs, hover intent, wheel/aux-click, timers, calls Tauri commands |
| `notch/geometry.ts` | Canvas/pill sizes (twin of the native pill constants), `targetGeometry`, `computeHitRect`, `computeClipPath`, `resolveRimStroke` |
| `notch/devices.ts` | `buildNotchDeviceRow`: favourites in order, current default first, max 4 chips (D-18); `toggleFavouriteDeviceIds` |
| `notch/backdrop.ts` | D-23 backdrop maths (WCAG luminance/contrast, gradient stops, scrim) and `resolveMusicServices` (D-22 quick-launch items) |
| `notch/visualizer.ts` | `VisualizerBank`, `mapFrameToBars`, `subscribeVisualizer`: 4–6 bars from `MeterFrame.channels` with attack/decay springs, loop stops when settled |

## `src/notch/activities/` (one component per job)
`IdlePill`, `VolumeActivity`, `SwitchActivity`, `PlugActivity`, `MicActivity`, `TrackActivity`, `ExpandedNotch` (container), `ExpandedMediaCard`,
`ExpandedDeviceRow`, `MusicServicesWidget`, `AlbumArtTile`, `NotchBackdrop`, `VisualizerBars`, `DeviceIcon`.

## How it works
- **Wake protocol:** on load, register listeners first, buffer deltas, fetch `get_engine_snapshot` + `get_notch_snapshot`, apply only newer revisions,
  re-snapshot on a gap. A recreated WebView never replays expired activities (`scheduler.syncSnapshot`).
- **Two state streams:** the engine's (`EngineSnapshot`/`Envelope`: devices, volume, media, settings) and the shell's `NotchSnapshot` (idle style,
  expanded, suspended, monitors, system reduce-motion). The controller merges them; the scheduler decides what activity shows.
- **Animation:** springs only, animating `transform`/`opacity`/`filter`/`clip-path`; the pill morph drives an SVG `<rect rx ry>` plus `clip-path: inset(… round …)`,
  never `width/height`. No loop while hidden or settled. Reduce motion (Windows flag via the shell) or preset `off` = instant, no fade.
- **After idle linger** (`IDLE_SUSPEND_LINGER_MS` = 15 s today; D-21.1 L1 moves it to config) the UI calls `request_idle_suspend`; the shell suspends or releases the WebView.
- **Reports to shell:** `report_hit_rect` (click-through area), `report_first_paint`, `report_event_rendered`. Visualizer: `set_visualizer({ active })`, frames on `audio://meter/v1`.
- Album art via `iceart://<id>`; accent/glow from `art.accent`.

## Traps
- Components use only `--ice-*` variables, no literal colours or radii (§5).
- Twin logic with the native pill (volume %, glyph choice, sizes) carries `// Twin:` comments; keep both in sync.
- `isMediaActive` / `isSameTrack` mirror Rust (`engine_bridge::media_active`, `TrackKey`); change both together.
- Durations in `scheduler.ts` (`VOLUME_DURATION_MS` …) are the only copy; if one becomes a setting, move it to `ice-core::config`.
- `npm test` lists test files explicitly in `package.json`; a new `*.test.ts` must be added there.

## Tests
`spring.test.ts`, `scheduler.test.ts`, `visualizer.test.ts`, `devices.test.ts`, `backdrop.test.ts` (node `--test`). Visual QA: 100%/150% scaling, light/dark, both monitors ([manual-checklist](../qa/manual-checklist.md)).

## Change log
- 2026-11-23 Claude: initial recap.
