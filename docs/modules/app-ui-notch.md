# app UI (notch and settings WebViews)

> Paths: `app/ui/**` · Owner: Gemini · Verified: 2026-11-23

## Purpose
Presentation only: Svelte 5 + TypeScript (`strict`). It renders the engine's state as the notch (idle pill, short activities, expanded card
with media and device row) and, later, the Settings window. It owns **no authoritative state**; it can be destroyed and rebuilt from a snapshot at
any time (D-15). Heavy work (image decode, colour extraction, device sorting) happens in Rust.

## Entry points
| File | What it does |
|---|---|
| `index.html` → `src/notch/main.ts` → `notch/NotchApp.svelte` | The notch: a 440×210 transparent canvas holding one clipped pill (≥ 24 px margin so the drop shadow isn't clipped) |
| `settings.html` → `src/settings/main.ts` | Settings window: see [app-ui-settings](app-ui-settings.md) |
| `src/themes/base.css` | All `--ice-*` tokens (documented in [theming.md](../theming.md): every new token must be added there) |

## `src/lib/` (logic, no markup)
| File | What it does |
|---|---|
| `ipc/engine.ts` | **Generated** from `ice-core` (never edit; fix the Rust type) |
| `ipc/types.ts` | **Generated** shell types (`NotchSnapshot`, `NotchDelta`, `ActivityKind`, `HitRect`, `MonitorSummary`) |
| `motion/spring.ts` | `Spring1D`: closed-form damped oscillator `(stiffness, damping, mass)`, frame-rate independent (tested at 75/144/240 Hz); `SPRING_PRESETS` snappy/smooth/bouncy/off |
| `motion/spring.svelte.ts` | `SpringGroup`: thin Svelte adapter, drives the loop only while something is still moving |
| `notch/scheduler.ts` | `NotchActivityScheduler` + `ActivityView` (`idle`, `volume`, `switch`, `plug`, `mic`, `track`): priorities, durations, merge rules (plug+switch within 1.5 s), snapshot sync and monotonic envelope handling (`NeedsResnapshot` on gap); `nowPlaying: off` creates no track pop-up. **621 lines, over the limit (split planned)** |
| `notch/nowPlaying.ts` | `resolveDisplayedView`: with `appearance.nowPlaying = while_playing` the idle notch shows the track pill while media plays (state-derived, no timer; reuses `lastTrackSeq` so it doesn't re-animate) |
| `notch/controller.svelte.ts` | `NotchController` (`createNotchController`): listens to `engine://*` and `notch://delta/v1`, owns springs, hover intent, wheel/aux-click, timers, calls Tauri commands |
| `notch/geometry.ts` | Canvas/pill sizes (twin of the native pill constants), `targetGeometry`, `computeHitRect`, `computeClipPath`, `resolveRimStroke` |
| `notch/devices.ts` | `buildNotchDeviceRow`: favourites in order, current default first, max 4 chips (D-18); `toggleFavouriteDeviceIds` |
| `notch/backdrop.ts` | D-23 backdrop maths (WCAG luminance/contrast, gradient stops, scrim) and `resolveMusicServices` (D-22 quick-launch items) |
| `notch/visualizer.ts` | `VisualizerBank` (14 springs: fast attack, quick fall), `subscribeVisualizer` (one shared bank; loop stops when settled), `withRestShape` (wide waveform's low resting line), `mapFrameToBars` (legacy mapping, tests). Bars render with `clip-path` (not `scaleY`) so caps stay round |
| `notch/beat.ts` | `BeatShaper`: meter frame → 14 bar targets from beats, not raw loudness (per-channel dB body level + recent peak; a new pattern per beat; centre swings, edges flick; silence = rest). `pickBars` for the 5-bar variants. The meter has no spectrum, so this is beat feel, not pitch |

## `src/notch/activities/` (one component per job)
`IdlePill`, `VolumeActivity`, `SwitchActivity`, `PlugActivity`, `MicActivity`, `TrackActivity`, `ExpandedNotch` (container), `ExpandedMediaCard`,
`ExpandedEmptyCard`, `ExpandedVolumeRow`, `ExpandedDeviceRow`, `NotchSideRail`, `MusicServicesWidget`, `AlbumArtTile`, `NotchBackdrop`, `VisualizerBars`, `DeviceIcon`.

**Expanded layout** (`EXPANDED_W` × `EXPANDED_H_MEDIA`/`EXPANDED_H_EMPTY` in `geometry.ts`): a main column (media or empty card, then
volume row and device chips) beside a side rail (`NotchSideRail`: pin, mic, music services while media plays, settings). The media card holds
only art, title/artist, transport and the scrubber. The volume bar is a click/drag slider (`set_volume`, throttled to 40 ms). There is no
device picker in the notch: Settings → Devices chooses and orders the chips. The card heights are fixed, so changing a row's height means
updating the constants.

## How it works
- **Wake protocol:** on load, register listeners first, buffer deltas, fetch `get_engine_snapshot` + `get_notch_snapshot`, apply only newer revisions,
  re-snapshot on a gap. A recreated WebView never replays expired activities (`scheduler.syncSnapshot`).
- **Two state streams:** the engine's (`EngineSnapshot`/`Envelope`: devices, volume, media, settings) and the shell's `NotchSnapshot` (idle style,
  expanded, suspended, monitors, system reduce-motion). The controller merges them; the scheduler decides what activity shows.
- **Animation:** springs only, animating `transform`/`opacity`/`filter`/`clip-path`; the pill morph drives an SVG `<rect rx ry>` plus `clip-path: inset(… round …)`,
  never `width/height`. No loop while hidden or settled. Reduce motion (Windows flag via the shell) or preset `off` = instant, no fade.
- **After idle linger** (`settings.idleLingerSecs`, config `idle_linger_secs`, default 15 s) the UI calls `request_idle_suspend` (Balanced only; Instant stays awake, so the notch never drops back to the static native pill); the shell suspends or releases the WebView.
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
- 2026-11-23 Claude: `NotchDeviceRowModel.pickerLabel` is `'More…' | null`; the "Choose devices…" button is gone (no favourites = current device + toggle partner only).
- 2026-11-23 Claude: theme blocks in `base.css` use `[data-theme='…']` (any element) so Settings can preview themes; Settings moved to its own doc.
- 2026-11-24 Claude: expanded relayout with a side rail (pin, mic, services, settings), draggable volume bar, `--ice-radius-md`; canvas 440×210 and a
  smaller `--ice-drop-shadow` so the shadow isn't clipped; Instant no longer requests idle suspend.
- 2026-11-24 Claude: "More…" and the in-notch picker removed (`NotchDeviceRowModel` is just `chips`); visualizer `BarShaper` + `clip-path` bars.
- 2026-11-24 Claude: "Now playing pill" setting (`nowPlaying.ts`, Settings → Notch, tray); Balanced only idle-suspends when the shown view is idle.
- 2026-11-24 Claude: `TrackActivity` has no transport buttons; a waveform scrubber (`VisualizerBars variant="wide"` + `spreadLevels`, accent up to the played position). Idle linger read from settings.
- 2026-11-24 Claude: `beat.ts` `BeatShaper` replaces `BarShaper` (beat-driven, 14 independent bars, quick fall, lower rest); `spreadLevels` → `withRestShape`.
