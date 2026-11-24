# D-25 · Gemini's vote

## Scores

### Part A · Technique (*pick one*)

| Option | Score | One-line reason |
|---|---|---|
| **A0** · Keep the backdrop static | **−1** | Zero cost and stays the fallback for `Minimal` / reduced motion / paused state, but leaves the owner's request for ambient gradient motion unaddressed. |
| **A1** · CSS `transform` drift of 2–3 soft gradient blobs | **+2** | Compositor-only (`transform` on pre-painted soft radial-gradient layers with no `filter`), satisfies D-13, and needs zero JavaScript timer or rAF loop. |
| **A2** · `@property` animation of today's wash | **−1** | Animating registered CSS custom properties (`<angle>`, `<percentage>`) forces main-thread style recalculation and Skia gradient repaint every frame (144 Hz), violating D-13. |
| **A3** · Canvas/WebGL mesh gradient | **−1** | Holding a WebGL context and JS `requestAnimationFrame` loop inside the notch WebView2 adds GPU working-set overhead and `TrySuspend` context-loss complexity for a surface covered by a `0.56–0.94` WCAG scrim (`384×178`). |
| **A4** · Moving blurred cover | **−1** | Invisible across all four bundled themes where `--ice-art-backdrop: 0` (D-23 ruling), and rotating high-contrast cover edges causes luminance pumping behind card text. |

### Part B · Where it runs (*pick many*)

| Option | Score | One-line reason |
|---|---|---|
| **B1** · Expanded card (`384×178`) | **+2** | Has enough negative space around the art and controls for a 20–30 s drift to read as depth, and only runs during intentional hover or session pin. |
| **B2** · Now-playing pill (`244×36`) | **−1** | A 20–30 s cycle moves `< 10%` of one period during the 2.4 s track-change pop-up (imperceptible), and under `nowPlaying = "while_playing"` it would run a 144 Hz compositor loop for hours inside a `36 px` strip where `<VisualizerBars>` already provides live motion. |
| **B3** · Idle pill (`116×28`) | **−2** | **Block:** the idle pill has no art backdrop (`controller.svelte.ts:116`), is rendered by the GDI layered window (`native_dot.rs`) in `Balanced`/`Lite` (breaking D-19 parity), and a permanent animation at idle threatens our `< 0.2%` idle CPU budget. |

### Part C · When it moves (*pick many*)

| Option | Score | One-line reason |
|---|---|---|
| **C1** · Only while media is playing | **+2** | `animation-play-state: paused` freezes the gradient smoothly at its current phase when paused (`media.status !== 'playing'`) and drops compositor ticks to zero when nothing plays. |
| **C2** · Stop when hidden, suspended, or covered by fullscreen | **+2** | Required by `docs/code-practices.md` §2 ("Hidden means idle"); must explicitly remove or pause the CSS animation because `opacity: 0` on `.stage.hidden` does not stop Chromium compositor keyframe ticking. |
| **C3** · User switch in Settings → Appearance ("Moving background") | **+2** | Satisfies WCAG 2.2.2 (Level A) for pinned cards open `> 5 s` alongside other content, while `systemReduceMotion` and `preset === 'off'` disable it automatically (D-13). |
| **C4** · Capped frame rate via JavaScript (~30 fps) | **−1** | Wakes the V8 main thread 30×/s to mutate inline styles for sub-pixel (`~0.06 px/frame`) drift when pure CSS `A1` stays entirely on the compositor thread alongside the already-ticking `<VisualizerBars>`. |
| **C5** · Audio-reactive drift | **−1** | Competes visually with the 5-bar `<VisualizerBars>` right inside the same card and adds a continuous JS meter-listener → DOM style-mutation path. |

### Part D · Speed (*pick one*)

| Option | Score | One-line reason |
|---|---|---|
| **D1** · One fixed speed chosen from a prototype | **+2** | Keeps Settings → Appearance uncluttered (just the `C3` on/off switch) while a CSS token `--ice-ambient-period` (e.g. `22s` primary / `31s` secondary coprime blobs) lets themes and custom CSS tune it. |
| **D2** · Setting: Calm / Normal / Lively | **0** | A 3-way speed selector on top of theme, spring preset, idle style, now-playing mode, and the `C3` switch is extra UI surface for a subtle background wash, though harmless if the owner wants it. |
| **D3** · The theme decides | **+1** | Themes must control the token under §5 (`--ice-ambient-period: 0s` in `Minimal`, `22s` in `Ice`/`Fluent`, `28s` in `Midnight`), but pairing `D1`'s single default speed with `C3`'s user toggle is better than locking motion strictly to theme selection. |

## Strongest argument against my top choice

My top choice (`A1` CSS `transform` blobs + `B1` expanded card only + `C1/C2/C3` + `D1`) relies on Chromium's compositor thread running `@keyframes` on two oversized (`~480×224`, `inset: -25%`) radial-gradient layers inside a transparent WebView2 window (`384×178` card inside `432×218` canvas). Because pure CSS `@keyframes` cannot self-throttle below the monitor's refresh rate, keeping the expanded card pinned on the owner's 144 Hz primary display will composite those two layers at 144 fps for as long as music plays (`~0.86 MB` RGBA8 texture memory for two layers, plus DWM composition overhead). Additionally, if an ancestor only hides the notch via `opacity: 0` (as `.stage.hidden` and `.stage.idle-hidden` do in `NotchApp.svelte:138-146`), Blink/CC continues ticking infinite CSS `@keyframes` off-screen unless the element explicitly sets `animation: none` or `display: none`. Finally, slow movement of large dark radial gradients behind a dark scrim (`--ice-scrim`) on an 8-bit non-OLED panel can show subtle Mach-band stepping if the gradient falloff is too steep.

## New option or amendment

**Amendment `A1-lifecycle-and-tokens` (Explicit compositor teardown + coprime blob periods + theme token):**
1. **Explicit compositor stop (`C1` + `C2`):** Pass `active={ctrl.isNotchVisible && ctrl.isExpanded}` and `playing={ctrl.displayedMedia?.status === 'playing' && ctrl.movingBackdropEnabled}` into `NotchBackdrop.svelte`. When `!active` or `instant` (`preset === 'off' || systemReduceMotion`) or `--ice-ambient-period: 0s`, set `animation: none` (not merely `opacity: 0`) so Chromium's compositor allocates zero animation tickers and drops `will-change`. When `active && !playing` (track paused while card is open), set `animation-play-state: paused` so the gradient freezes smoothly in place without snapping back to `0%`.
2. **Coprime 2-blob (or 3-blob) drift without `filter: blur()` (`A1`):** Split `.art-wash` (`NotchBackdrop.svelte:109-124`) into two oversized (`inset: -25%`) child layers with analytical soft falloff (`radial-gradient(closest-side, var(--ice-active-wash-primary), transparent 78%)` and `radial-gradient(closest-side, var(--ice-active-wash-secondary), transparent 80%)`) animated via `transform: translate3d(...) scale(...)` at coprime durations (`var(--ice-ambient-period, 22s)` and `calc(var(--ice-ambient-period, 22s) * 1.41)` ≈ `31s`, `ease-in-out infinite alternate`). Once Claude's 2–3 colour `art.palette` lands in `ice_core::media`, the third palette stop maps to `--ice-active-wash-tertiary` on an optional third layer or secondary ring.
3. **Token + `C3` switch integration (`D1` + `D3` + `C3`):** Add `--ice-ambient-period` to `app/ui/src/themes/base.css` (`22s` in `:root`/`Ice` and `fluent`, `28s` in `midnight`, `0s` in `minimal`) and document it in `docs/theming.md`, while the `C3` user toggle in Settings → Appearance ("Moving background", default on) gates the active animation class across any theme.

## Evidence I checked myself

- **`app/ui/src/notch/activities/NotchBackdrop.svelte:45-157`:**
  - Lines `109–124`: `.art-wash` currently paints both `--ice-active-wash-primary` (`76% 92% at 18% 18%`) and `--ice-active-wash-secondary` (`68% 84% at 88% 84%`) as a single static `background` property on one `inset: 0` element beneath `.art-scrim` (lines `127–131`). Splitting `.art-wash` into two `inset: -25%` children with `transform: translate3d(...) scale(...)` requires no `filter: blur()` because the radial gradient stop to `transparent` already provides the soft edge.
  - Lines `88–102`: `.art-blur-img` (V1) sets `opacity: var(--ice-art-backdrop, 0)`.
- **`app/ui/src/themes/base.css:33-100`:**
  - Line `33` (`:root`) and line `90` (`[data-theme='minimal']`) set `--ice-art-backdrop: 0`, and neither `midnight` (lines `50–62`) nor `fluent` (lines `64–80`) overrides it. Thus **`A4` (moving blurred cover) would be completely invisible (`opacity: 0`) in all four bundled themes** unless D-23's V2 default were undone.
  - Lines `92–93`: `[data-theme='minimal']` sets `--ice-art-wash: 0` and `--ice-scrim: none`, so adding `--ice-ambient-period: 0s` cleanly keeps `Minimal` static per §5.
- **`app/ui/src/notch/NotchApp.svelte:20-47, 138-146` and `app/ui/src/lib/notch/controller.svelte.ts:60-124`:**
  - `NotchApp.svelte:46` mounts `<NotchBackdrop art={ctrl.displayedMedia?.art} hasArt={ctrl.hasBackdropArt} instant={ctrl.isInstant} />` inside `.stage`. When the notch is hidden or suspended (`NotchApp.svelte:21-22, 138-146`), `.stage.hidden` and `.stage.idle-hidden` only apply `opacity: 0` while keeping `<NotchBackdrop>` mounted in the DOM — proving why `C2` must explicitly disable `animation` when `!ctrl.isNotchVisible || !ctrl.isExpanded`.
  - `controller.svelte.ts:113-117`: `hasBackdropArt` is `true` for `this.isExpanded || this.activeView.kind === 'track'`. Since commit `577cc25` wired `nowPlayingMode(this.engineState.settings)` (`controller.svelte.ts:65`), setting Now playing pill to `"while_playing"` keeps `activeView.kind === 'track'` (`244×36`, `geometry.ts:102`) visible for hours of background music. Running `D-25` on `B2` would therefore run a 144 Hz background animation for hours inside a `36 px` pill, whereas restricting `D-25` to `B1` (`this.isExpanded`, `384×178`, `geometry.ts:106`) bounds it to intentional expanded views.
- **`app/ui/src/notch/activities/ExpandedMediaCard.svelte:47-53`:**
  - While the expanded card is open and media is playing (`B1 + C1`), `<VisualizerBars playing={isPlaying} {preset} {instant} active={notchVisible && isPlaying} />` is already running its `requestAnimationFrame` spring loop (`0.013%` measured CPU in Phase 3). Compositor frames are therefore already being produced while `B1 + C1` is active, so two compositor-only `transform` layers (`A1`) add zero extra wakeups on the main thread.

## Position

### 1. Why `A1` (CSS `transform` blobs) is the right technique for our architecture
- **Strict D-13 compliance (`transform` only, zero `filter: blur()`, zero main-thread paint):** Our D-23 V2 wash (`NotchBackdrop.svelte:109-124`) is already built from two soft radial gradients (`--ice-active-wash-primary` and `--ice-active-wash-secondary`) underneath `--ice-scrim`. Because a `radial-gradient` falling off to `transparent 78%` is inherently soft-edged, we do **not** need `filter: blur()` on the moving layers. Promoting two oversized (`inset: -25%`) radial-gradient divs and animating only `transform: translate3d(±14%, ±12%, 0) scale(1.08)` runs 100% on Chromium's compositor thread with zero per-frame style recalculation or Skia rasterization.
- **Why `A2`, `A3`, and `A4` miss:**
  - `A2` (`@property`) repaints the gradient on the main thread every frame (144 repaints/sec on the owner's primary display) and violates D-13.
  - `A3` (WebGL mesh) creates a permanent WebGL context inside the notch WebView2 (`~8–20 MB` GPU process working set, plus `webglcontextlost` handling across `TrySuspend`) for an effect that sits behind a `0.56–0.94` dark scrim (`base.css:36-40`) inside a `384×178` card where 2–3 drifting radial blobs achieve the same organic colour shift at a fraction of the memory and code footprint.
  - `A4` (moving blurred cover) conflicts directly with D-23: `--ice-art-backdrop` is `0` in all four bundled themes (`base.css:33`), so `A4` would be invisible by default, and rotating bright cover corners under text causes contrast pumping.

### 2. Why `B1` (Expanded card only) wins and `B2` / `B3` should not animate the backdrop
- **`B1` (`384×178` expanded card, `+2`):** The expanded card has the surface area for a slow 20–30 s colour drift to feel atmospheric behind the translucent V3 chips (`--ice-control-bg`), and its runtime is bounded by hover or session pin (`L3`, which clears on lock/sleep). Moreover, while the expanded card is open and playing (`C1`), `<VisualizerBars>` is already producing frames (`ExpandedMediaCard.svelte:47-53`).
- **`B2` (`244×36` now-playing pill, `−1`):** In default `"on_track_change"` mode, the pill is only on screen for `2.4 s` — less than one-tenth of a `22–31 s` ambient cycle (`~2 px` of drift inside a `36 px` strip, which is visually indistinguishable from static). In `"while_playing"` mode (`577cc25`), the pill stays up for hours while the user works in other apps; running a continuous 144 Hz backdrop animation inside a `36 px` top-edge pill for hours is pure background GPU/DWM waste when the 5 visualizer bars inside `TrackActivity.svelte` already show live playback motion.
- **`B3` (`116×28` idle pill, `−2` Block):** The idle pill has no album art (`controller.svelte.ts:116`), uses the native GDI window (`native_dot.rs`) in `Balanced` and `Lite`, and must stay completely static at `< 0.2%` CPU in `Instant`.

### 3. Lifecycle (`C1 + C2 + C3`) and speed (`D1` with `--ice-ambient-period` token)
- **Freeze on pause (`C1`) and hard-kill on hide/collapse (`C2`):** Using `animation-play-state: paused` when playback pauses preserves the current gradient phase without a visual snap, while removing the animation class (`animation: none`) whenever `!isExpanded || !isNotchVisible || isInstant || !movingBackdrop` prevents Blink from ticking `@keyframes` behind `opacity: 0` (`.stage.hidden`).
- **"Not too fast, not too slow" (`D1` + `--ice-ambient-period`):** Two coprime periods — **`22 s`** on the primary blob and **`31 s`** (`22s × 1.41`) on the secondary blob (`28 s` / `39 s` in `Midnight`, `0s` in `Minimal`) — with `±12–14%` translation amplitude (~`45 px` travel across `384 px` over `11–15.5 s` per half-cycle, or `~3–4 px/s`) give a noticeable, living drift within a 5–10 s glance without ever feeling busy or repeating visibly. A single owner-tuned speed (`D1`) backed by `--ice-ambient-period` (`D3`) and a simple Settings → Appearance `"Moving background"` toggle (`C3`) gives full control without cluttering Settings.
