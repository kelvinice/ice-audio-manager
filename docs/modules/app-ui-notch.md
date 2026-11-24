# app UI (notch and settings WebViews)

> Paths: `app/ui/**` · Owner: Gemini · Verified: 2026-11-23

## Purpose
Presentation only: Svelte 5 + TypeScript (`strict`). It renders the engine's state as the notch (idle pill, short activities, expanded card
with media and device row) and, later, the Settings window. It owns **no authoritative state**; it can be destroyed and rebuilt from a snapshot at
any time (D-15). Heavy work (image decode, colour extraction, device sorting) happens in Rust.

## Entry points
| File | What it does |
|---|---|
| `index.html` → `src/notch/main.ts` → `notch/NotchApp.svelte` | The notch: a 648×292 transparent canvas holding a sibling `.pill-shadow` + one `.pill-clip` container (≥ 24 px margin around Floating & Edge Nook split layouts) |
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
| `notch/controller.svelte.ts` | `NotchController` (`createNotchController`): listens to `engine://*` and `notch://delta/v1`, owns springs, unified optimistic seek (`effectiveMedia`), hover intent, wheel/aux-click, timers, calls Tauri commands (deduped `report_hit_rect`) |
| `notch/geometry.ts` | Canvas/pill sizes (`648×292`, twin of native pill constants, incl. `292×46` track pill), `effectivePreset` (`'bouncy'` fallback), `targetGeometry`, `idleGeometry`, `computeHitRect`, `computeClipPath`, `resolveRimStroke` |
| `notch/notchStyle.ts` | Strategy pattern (`NotchStyleStrategy`: `floatingStyleStrategy` vs `nookStyleStrategy`), SVG concave-ear `buildNookFillPath` / `buildNookRimPath`, surface shape & hit-rect calculation |
| `notch/devices.ts` | `buildNotchDeviceRow`: favourites in order, current default first, max 4 chips (D-18); `toggleFavouriteDeviceIds` |
| `notch/backdrop.ts` | D-23/D-25 backdrop maths: `resolveAmbientMotionState`, `adaptAccentForSurface` (for `ice_light`), `computeArtBackdrop` (5-stop hue-shifted wash + `washBaseCss` + luminance-clamped scrim) and `resolveMusicServices` (D-22 quick-launch items) |
| `notch/visualizer.ts` | `VisualizerBank` (14 frequency-graded `(k, c, m)` springs: heavier bass → snappier treble, fast attack, quick fall), `subscribeVisualizer` / `subscribeTempo` (one shared bank + tempo listener; rAF-coalesced DOM writes; loop stops when settled; 400 ms stop-grace across visible transitions), `withRestShape` (wide waveform's low resting line), `mapFrameToBars` (legacy mapping, tests). Bars render with `clip-path` (not `scaleY`) so caps stay round |
| `notch/beat.ts` | `BeatShaper`: dual-mode 14-bar target shaper — `A2 + B1` real spectrum (audible-energy-gated per-band AGC + pink tilt + low-band beat accent) when `frame.bands` is present, or `A1` multi-envelope travelling-wave peak fallback otherwise, plus smoothed `tempoRate`. `pickBadgeBars` (`B4a` symmetric centre-bass 5-bar badge) and `pickBars` |
| `notch/activities/NotchBackdrop.svelte` | V2 default 9-orb orbital lava-lamp `.art-wash` (`wash-base` + `screen` blending on dark themes, `normal` on `ice_light`, scaled smoothly by `subscribeTempo` and paused via `animation-play-state`) + opt-in V1 blurred cover & `.art-scrim` gated by `--ice-art-backdrop` |
| `notch/activities/AlbumArtTile.svelte` | Album art / placeholder tile with crossfade and optional `radius` prop (defaults to `--ice-radius-md` / `--ice-radius-sm`; track pill passes `size={34}` `radius="16px"`) |
| `notch/scrubber.ts` | Pure media-scrubber maths: `clampSeekPositionMs`, `liveMediaPositionMs` (extrapolates from `positionUpdatedMs`), `pointerSeekMs`, `stepSeekMs`, `nextSecondTickDelayMs`, and unified optimistic seek overlay (`applyOptimisticSeek`, `shouldClearOptimisticSeek`) |
| `notch/lyrics.ts` | Pure lyrics frame resolver (`resolveLyricFrame`, `shouldShowPillLyrics`, `shouldShowCardLyrics`): active line, neighbour lines, active word index, and next boundary wake delay (plus `fitLine.ts` cached per-line word measurements for zero-reflow word highlighting); `nookLadderY` places Edge Nook rows around the wrapped current line's measured height |
| `notch/activities/LyricsExtension.svelte` | Seamless bottom extension under the compact music pill (1 active line + Romaji/subtitle) and expanded card (3-line focus band, click-to-seek, active-word lift/glow via `--ice-lyric-*` tokens, plain fallback) |

## `src/notch/activities/` (one component per job)
`IdlePill`, `VolumeActivity`, `SwitchActivity`, `PlugActivity`, `MicActivity`, `TrackActivity`, `ExpandedNotch` (`Floating` container), `NookExpandedNotch` (`Edge Nook` flush top-edge split container), `ExpandedMediaCard`,
`ExpandedEmptyCard`, `MediaScrubber`, `ExpandedVolumeRow`, `ExpandedDeviceRow`, `NotchSideRail`, `MusicServicesWidget`, `AlbumArtTile`, `NotchBackdrop`, `VisualizerBars`, `DeviceIcon`.

**Expanded layout** (`EXPANDED_W` × `EXPANDED_H_MEDIA`/`EXPANDED_H_EMPTY` in `geometry.ts`): a main column (media or empty card, then
volume row and device chips) beside a side rail (`NotchSideRail`: pin, mic, music services while media plays, settings). The media card holds
only art, title/artist, transport and an interactive seek scrubber (`ExpandedMediaCard`, pointer capture + keyboard `±5s`/`±15s`/`Home`/`End` via `MediaAction` `seek`, live 1 Hz interpolation while playing and visible). The volume bar is a click/drag slider (`set_volume`, throttled to 40 ms). There is no
device picker in the notch: Settings → Devices chooses and orders the chips. The card heights are fixed, so changing a row's height means
updating the constants.

## How it works
- **Wake protocol:** on load, register listeners first, buffer deltas, fetch `get_engine_snapshot` + `get_notch_snapshot`, apply only newer revisions,
  re-snapshot on a gap. A recreated WebView never replays expired activities (`scheduler.syncSnapshot`).
- **Two state streams:** the engine's (`EngineSnapshot`/`Envelope`: devices, volume, media, settings) and the shell's `NotchSnapshot` (idle style,
  expanded, suspended, monitors, system reduce-motion). The controller merges them; the scheduler decides what activity shows.
- **Animation & backdrop:** springs only, animating `transform`/`opacity`/`filter`/`clip-path`; the pill morph drives an SVG `<rect rx ry>` plus `clip-path: inset(… round …)`,
  never `width/height`. Drop shadow (`--ice-drop-shadow`) sits on a sibling `.pill-shadow` outside `.pill-clip` so ambient backdrop layers do not get re-filtered each frame.
  `NotchBackdrop` runs D-25 9-orb lava-lamp `transform` keyframes (`--ice-ambient-period`: `9s`, `13s` on `midnight`; disabled on `minimal`) only while visible and playing on the expanded card or music pill, scaled by beat cadence via `subscribeTempo` and frozen on pause (`ambient-paused`).
  No loop while hidden or settled. Reduce motion (Windows flag via the shell) or preset `off` = instant, no fade. Includes five built-in `--ice-*` themes (`ice`, `ice_light`, `midnight`, `fluent`, `minimal`).
- **After idle linger** (`settings.idleLingerSecs`, config `idle_linger_secs`, default 15 s) the UI calls `request_idle_suspend` (Balanced only; Instant stays awake, so the notch never drops back to the static native pill); the shell suspends or releases the WebView.
- **Reports to shell:** `report_hit_rect` (click-through area), `report_first_paint`, `report_event_rendered`. Visualizer: `set_visualizer({ active })`, frames on `audio://meter/v1`.
- Album art via `iceart://<id>`; accent/glow from `art.accent`.

## Traps
- Components use only `--ice-*` variables, no literal colours or radii (§5).
- Twin logic with the native pill (volume %, glyph choice, sizes) carries `// Twin:` comments; keep both in sync.
- `isMediaActive` / `isSameTrack` mirror Rust (`engine_bridge::media_active`, `TrackKey`); change both together.
- Durations in `scheduler.ts` (`VOLUME_DURATION_MS` …) are the only copy; if one becomes a setting, move it to `ice-core::config`.
- `npm test` lists test files explicitly in `package.json`; a new `*.test.ts` must be added there.
- Classes added from script (`classList` in an action, e.g. `fitLine`'s `fitting`/`fade-l`/`fade-r`) are invisible to Svelte's
  scoped CSS: write their rules as `.x:global(.cls)` or Svelte drops them (the build warns `css_unused_selector`).

## Tests
`spring.test.ts`, `scheduler.test.ts`, `scrubber.test.ts`, `visualizer.test.ts`, `devices.test.ts`, `backdrop.test.ts` (node `--test`). Visual QA: 100%/150% scaling, light/dark, both monitors ([manual-checklist](../qa/manual-checklist.md)).

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
- 2026-11-24 Gemini: D-25 9-orb lava-lamp `.art-wash` (`NotchBackdrop.svelte`, `backdrop.ts`, `--ice-ambient-period: 9s`/`13s`), sibling `.pill-shadow` in `NotchApp.svelte`, `subscribeTempo` / `BeatShaper.tempoRate`, `ice_light` theme (`adaptAccentForSurface`), `292×46` music pill (`34×34` `16px` art), and `'bouncy'` default fallback.
- 2026-11-24 Gemini: interactive progress seeking on `ExpandedMediaCard` (`scrubber.ts`, `MediaAction` `seek`, 1 Hz playback interpolation while expanded and visible).
- 2026-11-24 Gemini: smooth 1 Hz playback timecode ticking using GSMTC `positionUpdatedMs` (`liveMediaPositionMs`) in `ExpandedMediaCard` and `TrackActivity`.
- 2026-11-24 Gemini: bottom lyrics extension (`LyricsExtension.svelte`, `lyrics.ts`, `--ice-lyric-*` tokens, `CANVAS_H = 292`) for both compact music pill and expanded card with active-word emphasis, Romaji/paired subtitles, and side-rail toggle.
- 2026-11-24 Claude: lyrics toggle moved from `NotchSideRail` to the `ExpandedMediaCard` transport row (the rail holds five buttons; a sixth cut off Settings); music-pill lyrics show two rows (current + next, `LyricFrame.followingLine`), `PILL_LYRICS_H` 42 / `PILL_LYRICS_SUB_H` 56.
- 2026-11-24 Claude: lyrics ladder (`lyricRows`/`ladderY`): line changes scroll in the pill and card; `lineSubtitle` (paired lines always, generated `romaji` only while on); Romaji sweep (`romajiLitCount`).
- 2026-11-24 Claude: `fitLine.ts` (+ test): the main lyric line shrinks to fit (≥ 82 %) or slides with the singing instead of losing its last characters (ellipsis can't apply to inline-block karaoke words); ladder `overflow: clip` + 2 px margin.
- 2026-11-24 Gemini: D-26 spectrum `BeatShaper` (`A2 + B1` audible-gated per-band AGC/pink tilt + low-band beat accent; `A1` travelling-wave fallback), `B2` frequency-graded springs (`visualizer.ts`), and `B4a` symmetric centre-bass 5-bar badge (`pickBadgeBars` in `VisualizerBars.svelte`).
- 2026-11-24 Gemini: unified optimistic seek overlay (`NotchController.effectiveMedia` + `scrubber.ts`), `visualizer.ts` rAF-coalesced writes & 400 ms visible stop-grace, `fitLine.ts` cached per-line word measurements, and `report_hit_rect` dedup.
- 2026-11-24 Gemini: added `Edge Nook` (`NotchStyle::Nook`) strategy layer (`notchStyle.ts`), extracted `MediaScrubber.svelte`, `NookExpandedNotch.svelte` 2-column media/lyrics split + concave-ear SVG clip/rim, and `CANVAS_W = 648`.
- 2026-11-24 Gemini: fixed `NookExpandedNotch.svelte` `onOpenMusicService` prop forwarding, adjusted Edge Nook expanded heights (`206`/`166`, `LADDER.nook.inner = 152`), and switched notch fallbacks to `nook` and `while_playing`.
- 2026-11-25 Claude: lyrics `fitLine` rules made `:global` (Svelte had dropped them: no fades, long centred card/nook lines lost their first characters); fitted row bleeds 4 px / 3 px (`FIT_BLEED_X_PX`) so the active word's lift isn't clipped, ladder clip margin 4 px; card main row stretches (short lines no longer shrink 2 %). Edge Nook current line wraps (balanced, ≤ 3 rows, `LADDER.nook.mainMax`) and rows are placed by `nookLadderY` from a `ResizeObserver` on the current row.
- 2026-11-25 Claude: D-27 hibernate: `controller.isHibernating` forces the idle view, no expansion/hover/wheel, `plainGeometryContext` (+ test) for a constant shape and `NO_HIT_RECT`; `PlainNotch.svelte` (mic dot only), no backdrop/shadow/rim, `--ice-hibernate-*` tokens.
