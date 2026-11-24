# D-25 · Claude's vote

*Written sealed on 2026-11-24 from the entry and my own checks. I didn't open `codex.md` or `gemini.md` (or their
history) before this file existed.*

## Scores

### Part A · technique (pick one)
| Option | Score | One-line reason |
|---|---|---|
| A0 static | −1 | The owner asked for motion; static stays the reduce-motion / switched-off / Minimal state, not the answer |
| A1 CSS transform blobs | +2 | Compositor-only (`transform`), no JavaScript loop, pausable with `animation-play-state`, speed as a token |
| A2 `@property` wash | −1 | Style recalculation and repaint every frame at 144 Hz over the full canvas layer (entry fact 1) |
| A3 WebGL mesh | −1 | A GL context, a JavaScript frame loop and the most new code to own, for a 392 × 164 card |
| A4 moving blurred cover | 0 | V1 is off in every bundled theme; it could reuse A1's drift later as a per-theme opt-in |

### Part B · where (pick many)
| Option | Score | One-line reason |
|---|---|---|
| B1 expanded card | +2 | The largest surface, shown only while hovered or pinned, so run time is bounded by the user |
| B2 now-playing pill | +1 | Fine for the 2.4 s pop-up; with "While playing" it runs for hours, so only with C1–C3 and a measurement |
| B3 idle pill | −1 | Always visible in Instant (since 2026-11-24): hours of motion at idle against the < 0.2 % idle-CPU budget; it has no art backdrop to move anyway |

### Part C · when (pick many)
| Option | Score | One-line reason |
|---|---|---|
| C1 only while playing | +2 | Motion means "music is playing"; paused or nothing playing = still, which bounds run time |
| C2 stop when hidden/suspended/fullscreen | +2 | Already required by code-practices §2 ("Hidden means idle"); worth stating as a test |
| C3 user switch | +2 | WCAG 2.2.2 (Level A) needs a real off switch; reduce motion / preset Off stay static regardless |
| C4 capped frame rate | 0 | Only if A1 fails its measurement at 144 Hz: it needs a JavaScript loop that pure CSS avoids |
| C5 audio-reactive | −1 | The bars already show the beat (`beat.ts`, 2026-11-24); a pulsing backdrop behind them competes and needs per-frame JavaScript |

### Part D · speed (pick one)
| Option | Score | One-line reason |
|---|---|---|
| D1 one fixed speed | +1 | Simple; the owner picks it from a prototype |
| D2 Calm / Normal / Lively setting | 0 | A third knob for a background; C3's on/off covers the accessibility need |
| D3 theme decides | +2 | Fits §5: `--ice-ambient-period` per theme (Ice moves, Minimal static), user themes can tune it, C3 turns it off |

## Strongest argument against my top choice (A1)
A CSS animation can't cap its frame rate, so A1 composites at the display rate (144 Hz on the owner's main monitor)
for as long as it runs. The only data point for a never-ending CSS animation in Chromium is bad (entry fact 5:
VS Code at ~13 % idle CPU from a blinking cursor). On top of that, see my evidence below: the whole stage has a
`drop-shadow` filter, so an animated child makes the shadow re-run every frame too. If the measurement comes in high,
A1 has no lever except switching to a JavaScript driver (C4), which is exactly what it was meant to avoid.

## New option or amendment
**A1 with two conditions:**
1. **Take the shadow off the animated subtree first.** Today `.stage { filter: drop-shadow(...) }` wraps the pill, the
   rim and the backdrop. Moving the shadow to a separate static element (or a `box-shadow` on the clip layer) means the
   drift doesn't re-filter the stage each frame. This also helps every existing animation (springs, visualizer).
2. **Measurement gate with a fallback.** Run the entry's 60 s measurement on the 144 Hz monitor. If A1 costs more than
   +0.1 % process-tree CPU over the static backdrop, drive the same layers at ~30 fps from JavaScript (C4), still
   `transform`-only. Record the result in plan.md's budget table as a "visible and animating" row.

Also, the blobs should be sized to the expanded card (≤ 392 × 164), not the 440 × 210 canvas the backdrop fills today,
to keep the composited layers small.

## Evidence I checked myself
- `app/ui/src/notch/NotchApp.svelte:135`: `.stage { filter: drop-shadow(var(--ice-drop-shadow)) }` wraps the whole
  pill subtree, so any per-frame change inside is re-filtered.
- `app/ui/src/notch/activities/NotchBackdrop.svelte:70–74`: `.notch-backdrop { position: absolute; inset: 0 }` inside
  `.pill-surface` (`NotchApp.svelte:154–158`, also `inset: 0`), so the backdrop layer is canvas-sized (440 × 210).
  The only motion today is `backdrop-fade` 360 ms (`NotchBackdrop.svelte:83–84`).
- `app/ui/src/lib/notch/controller.svelte.ts:51–53`: `isInstant` = Windows reduce motion or preset Off, already passed
  to `NotchBackdrop` as `instant`. C3's reduce-motion half has a hook today; the user switch needs a new setting.
- `app/ui/src/lib/notch/geometry.ts:11–12`: the expanded card is 392 × 164.
- `app/ui/src/lib/notch/visualizer.ts:448–451`: the visualizer's `requestAnimationFrame` loop stops when the springs
  settle. That's the precedent for C1/C2: motion that ends by itself.
- `docs/plan.md:22`: idle CPU < 0.2 % over 60 s. `docs/code-practices.md:44`: "Hidden means idle".
- [W3C: Understanding SC 2.2.2](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide): the 5 s rule and
  "hover isn't a pause mechanism".

## Position
Yes to motion, built the cheapest way that can be paused: A1 soft blobs drifting on `transform` with long, unequal
periods, on the expanded card (and the pop-up), only while music plays, never when hidden, with a real off switch and
the speed as a theme token. Take the stage shadow off the animated subtree first, then measure at 144 Hz before
shipping, and keep a 30 fps JavaScript driver as the fallback if the numbers are bad.

## Revision round (after the reveal, 2026-11-24)
- **B2 now-playing pill: +1 → −1 (changed after reveal).** Gemini and Codex showed that a 20–30 s drift moves only a
  few pixels during the 2.4 s pop-up (it reads as static), while "While playing" would keep it composited for hours in
  a 48 px strip that already has the live waveform. My +1 was conditional on C1–C3; their point is that even with them
  the benefit is close to zero.
- No other changes. I back the lifecycle amendment (Gemini: `animation: none` when hidden or collapsed, because
  `.stage.hidden` is only `opacity: 0`, and `animation-play-state: paused` on pause) and the scrim/phase amendment
  (Codex: move only the colour layers, keep the scrim still, and don't key the loop to the art ID, so a track change
  doesn't snap the colours).
- **Size correction for the record:** since the 2026-11-24 relayout the expanded card is 392 × 164 (144 with nothing
  playing) and the now-playing pill is 296 × 48 (`geometry.ts`), not 384 × 178 / 244 × 36. No conclusion changes.
