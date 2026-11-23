# D-23 · Gemini's vote

> Cast 2026-11-23 blind (§0.4) from the D-23 entry and my own visual mockups + inspection of `ExpandedMediaCard.svelte`, `ExpandedDeviceRow.svelte`, `NotchApp.svelte`, `AlbumArtTile.svelte`, `crates/ice-core/src/media.rs`, `docs/theming.md`, `docs/reference/Echo-Music/assets/banner.png`, `Screenshots/lyrics.png`, and `docs/reference/BitChord/Banner.png`. I have not opened any other agent's vote file for D-23.

## Scores

| Option | Score | One-line reason |
|---|---|---|
| V1 · Blurred album-art backdrop | +1 | Looks lush on dark covers like *Shameless*, but bright/white covers wash into pale slate behind blur unless capped by a heavy scrim that nearly erases the image |
| V2 · Art-derived 2-colour gradient wash | +2 | Preserves WCAG AA contrast cleanly on both bright/white and dark covers, adapts cleanly to the `384×178` strip, and avoids CSS `filter: blur()` over `clip-path` |
| V3 · Translucent controls over backdrop | +2 | Replacing heavy bordered surfaces on chips/transport with low-alpha tints cuts `384×178` visual density dramatically while staying token-driven |
| V4 · Cleanliness pass on expanded card | +2 | Larger title (`13 px`), unified scrubber, inverted Play/Pause pill, and borderless chips give Echo/BitChord polish while keeping volume & D-18 devices one click away |
| V5 · True see-through blur behind notch | −2 | **Block:** CSS `backdrop-filter` cannot blur desktop pixels on a transparent Win32 WebView2 HWND and DWM Acrylic jitters during 144 Hz spring morphs (D-12, `docs/theming.md:14`) |
| V6 · Lyrics styling | +1 | Echo-Music's active-bright / distance-fade hierarchy works well only as an expanded toggle sub-view (not crammed inside `384×178` alongside devices) |

## Strongest argument against my top choice

- **Against V2 (`+2`) + V3 (`+2`) + V4 (`+2`):**
  1. **V2 vs V1:** Looking closely at my 384×178 mock comparisons, V2's dual radial gradients look noticeably more synthetic than V1's blurred cover photo when rich multi-tone covers (gold + crimson + shadow, like *Shameless* or *Blinding Lights*) fill the card. If a user opens the expanded notch expecting Echo-Music's photographic *"blurred album-art glass"* feel, pure 2-stop CSS gradients feel flatter than an enlarged `128 px` art `<img>` blur.
  2. **V3 + V4 at `384×178` & over light/high-contrast covers:** Translucent borderless device chips (`rgba(255,255,255,0.06)`–`0.10`) look sleek on dark backgrounds, but make hit targets less obvious in mouse-heavy workflows and risk dropping below WCAG 3:1 UI component boundary contrast if `--ice-scrim` is too transparent or if a user uses a custom theme (`Minimal` or high-contrast). Also, unlike Echo-Music/BitChord's tall phone screen, `384×178` leaves zero room to demote D-18 device chips behind `"…"` without adding an extra click to the core product action.

## New option or amendment

1. **Backdrop amendment — V2 default with V1 opt-in via one theme token (`--ice-art-backdrop`) + luminance-adaptive scrim:**
   - Extend `crates/ice-core/src/media.rs` to extract a **2-colour palette** (primary + secondary accent, keeping `dominant_bgra`'s saturation weighting and colourless fallback so white covers like `#fffffe` use theme accents rather than white/grey mud).
   - Ship **V2** (`0` CSS blur, 2-stop radial art wash over `--ice-bg`) as the fast, contrast-guaranteed base backdrop, and allow users/themes who want Echo-Music's photographic **V1** (`filter: blur(28px)` `128 px` cover + dark luminance-clamped `--ice-scrim`) to flip it via Appearance / theme token without extra layout code!
   - When V1 is active, clamp `--ice-scrim` opacity dynamically from `media.luminance` (e.g. `0.56→0.82` on dark covers vs `0.72→0.88` on light covers) so white `#edf1f7` text always passes `≥ 4.5:1` WCAG AA.
2. **V4 `384×178` layout boundary:** Apply Echo-Music/BitChord's hierarchy **without hiding audio controls**: keep the title (`13 px`, `600`), artist/spatial badge, scrubber, `3` transport buttons (with a clean high-contrast Play/Pause circle like BitChord), `Pin`, compact volume bar (relying on wheel/drag scrubber instead of `-`/`+` clutter or keeping `-`/`+` compact), D-18 device chips, `Mic On`, and `⚙` visible inside `384×178`.
3. **V6 container rule:** Render lyrics inside a toggleable expanded sub-view (expanding vertically `~224 px` or swapping the lower volume/device region temporarily with a `"Lyrics"` chip) so device switching and lyrics never compete inside `178 px` height.

## Evidence I checked myself

- **Live HTML/CSS mockup at exact `384×178` geometry rendered in headless Chrome at `2×` (`d23-mock.png` inspected visually):**
  - Compared 6 cards side-by-side (`Baseline`, `Minimal`, `V1+V3+V4` dark cover & bright light `#e8f3ff/#bce3ff` cover on `#F6F7F9` light desktop, `V2+V3+V4` dark & light covers).
  - **Visual finding:** On the dark crimson/amber cover (*Shameless*), V1 + V3 + V4 produces an unmistakably warmer photographic wash than baseline; however, on the bright light cover (*Skyline Dawn*), V1's blurred light cover lightens the whole backdrop into dusty pale slate (`~#2c3743–#3d4c5c`) unless darkened heavily, whereas V2 preserves deep `--ice-bg` depth and sharp foreground contrast on both dark and light covers while still clearly reflecting each cover's palette.
  - **V3 + V4 visual check:** Removing heavy `1px solid var(--ice-border)` outlines from every chip and switching transport/device controls to low-alpha translucent pills + a crisp light Play/Pause circle (`rgba(255,255,255,0.92)`) eliminates the "boxed grid" look at `384×178`.
- `app/ui/src/notch/activities/ExpandedMediaCard.svelte:37-201` & `ExpandedDeviceRow.svelte:99-160`: every button currently uses `border: 1px solid var(--ice-border); background: var(--ice-surface)` (solid opaque surface) and `media-title` is `12px`, while `ExpandedNotch.svelte` also adds `-`/`+` chips beside an interactive volume bar inside `384×178` (`geometry.ts:30`).
- `crates/ice-core/src/media.rs:104-142`: `dominant_bgra` already computes saturation-weighted BGRA buckets and warns (`line 108`) that Spotify frequently sends mostly-white covers (`#fffffe`), confirming why un-scrimmed light-cover blur breaks contrast.
- `app/ui/src/themes/base.css:3-55` & `docs/theming.md:14, 24-50, 90-96`: confirms `Midnight` relies on near-black `rgba(0,0,0,0.96)`, `Fluent` on dark zinc `rgba(24,24,27,0.93)`, `Minimal` disables rim/glow cleanly via `transparent` tokens, and `docs/theming.md:14` documents why WebView2 cannot blur desktop pixels behind the notch (`V5`).
- `docs/reference/Echo-Music/assets/banner.png`, `Screenshots/lyrics.png`, & `docs/reference/BitChord/Banner.png`: both apps get their calm feel from borderless translucent buttons over an art-tinted scrim, high-contrast central Play/Pause, thin scrubbers without heavy chrome, and active-bright / neighbour-dimmed lyrics.

## Position

**How V1–V4 and V6 fit our notch & 4 bundled themes (`base.css`, `docs/theming.md`):**
We can adopt **`V2 (+2)` + `V3 (+2)` + `V4 (+2)`** cleanly (keeping **`V1 (+1)`** as an optional token-gated upgrade and **`V6 (+1)`** for D-22's lyrics toggle) by introducing 4 documented `--ice-*` tokens in `base.css` and `docs/theming.md`:
- `--ice-art-wash-alpha` (`0.38` in `Ice`, `0.22` in `Midnight` to preserve OLED black, `0.32` in `Fluent`, `0` in `Minimal`)
- `--ice-art-blur-opacity` (`0` default for V2 gradient wash, or `0.85` if V1 blur is selected; `0` in `Minimal`)
- `--ice-scrim` (luminance-guarded gradient overlay protecting `--ice-fg` `≥ 4.5:1`)
- `--ice-control-tint` + `--ice-control-border` (`rgba(255,255,255,0.09)` / `transparent` in `Ice`/`Midnight`/`Fluent`; `var(--ice-surface)` / `var(--ice-border)` in `Minimal` so `Minimal` keeps crisp solid controls and lowest GPU overdraw).

Note on **V1 vs V2** for the WebView compositor: because `NotchApp.svelte` morphs its outer pill using animated `clip-path: inset(...)` at 144 Hz, V2's 2-stop CSS gradient avoids a large `filter: blur(28px)` offscreen pass during motion while solving bright Spotify covers cleanly. V5 (`−2`) stays blocked under D-12 (`docs/theming.md:14`).
