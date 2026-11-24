# Ice Audio Manager: Theming Guide

> **Applies to:** Windows 11 WebView notch (`TrySuspend`ed when idle, D-15.1 = A), lazy Settings window, and native wake / `Lite` pill (`crates/ice-win/src/native_dot.rs`).
> **Decisions:** D-04 (TOML + CSS custom properties), D-12 (Visual style & `Ice` default), D-15.1 (Option A: `WebView + TrySuspend` + native pill parity).

---

## 1. Architecture & Surface Model

Ice Audio Manager styles its UI using plain CSS custom properties (`--ice-*`) scoped to `:root`. Because CSS variables drive every surface, border, typography slot, and semantic status indicator, themes are portable plain CSS files with zero preprocessor or build step.

### Transparent Notch vs. Settings Window

- **Notch overlay:** Windows 11 DWM Acrylic cannot animate cleanly on a morphing HWND without frame jitter, and CSS `backdrop-filter: blur()` cannot blur desktop pixels behind a transparent Win32 WebView2 window. Instead, the notch renders as a **tinted translucent obsidian-ice surface** (`--ice-bg`, `--ice-surface`) framed by a subtle specular rim (`--ice-rim`), border (`--ice-border`), ambient glow (`--ice-accent-glow`), and soft elevation shadow (`--ice-shadow`).
- **Settings window:** Rectangular window rendered lazily on demand and destroyed on close, using **native Windows 11 Mica** (`DwmSetWindowAttribute` `DWMSBT_MAINWINDOW`) alongside the active light/dark theme tokens.
- **Native wake / `Lite` pill parity:** When the notch WebView is `TrySuspend`ed or completely torn down (`Lite` / gaming mode), `ice_win::native_dot` parses the eight synced `--ice-*` colour tokens in Rust (`NativePillTheme::from_css_overrides`) so the native dot and instant volume/mute pill match the active theme seamlessly before the WebView resumes.

---

## 2. Token API (`--ice-*`)

All `--ice-*` variables are defined with their `Ice` defaults in CSS `:root`. Any user theme may override a subset or all of them.

| Token | Category | Default (`Ice`) | Synced to Native Pill | Purpose |
| --- | --- | --- | --- | --- |
| `--ice-bg` | Surface | `rgba(14, 17, 22, 0.9)` | Yes | Primary pill / island backdrop |
| `--ice-surface` | Surface | `rgba(23, 29, 39, 0.92)` | Yes | Elevated cards, tracks, chips, expanded panel sections |
| `--ice-track` | Surface | `rgba(255, 255, 255, 0.14)` | No | Unfilled progress bar track inside volume / switch pills |
| `--ice-border` | Surface | `rgba(107, 197, 255, 0.22)` | No | Inner hairline border around pill and cards |
| `--ice-rim` | Surface | `rgba(154, 214, 255, 0.45)` | Yes | 1 px specular rim highlight around the notch capsule |
| `--ice-shadow` | Surface | `0 10px 28px rgba(0, 0, 0, 0.48), 0 2px 8px rgba(0, 0, 0, 0.32)` | No | Box shadow beneath Settings / card elements |
| `--ice-drop-shadow` | Surface | `0 6px 16px rgba(0, 0, 0, 0.42)` | No | Box shadow beneath the floating `.pill-shadow` sibling element (off the animated pill subtree so D-25 drift does not re-filter it). Offset + blur must stay inside the canvas margin (≥ 24 px around the expanded pill, `geometry.ts`) or the window edge clips it |
| `--ice-radius` | Surface | `22px` | No | Expanded card corner radius (compact pills compute capsule radii dynamically) |
| `--ice-radius-md` | Surface | `12px` | No | Large tiles: expanded album art and the empty-state music tile |
| `--ice-radius-sm` | Surface | `8px` | No | Small control, chip, and mini button corner radius |
| `--ice-radius-pill` | Surface | `999px` | No | Full-capsule radius for tracks, status dots, and spatial/status badges |
| `--ice-fg` | Typography | `#edf1f7` | Yes | Primary text, icons, and volume `%` readout |
| `--ice-fg-muted` | Typography | `#98a4b8` | Yes | Secondary captions, device subtitles, inactive icons |
| `--ice-font` | Typography | `'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif` | No | Primary UI font stack (ships with Win11; optional local `IBM Plex Sans`) |
| `--ice-mono` | Typography | `'Cascadia Mono', 'Consolas', monospace` | No | Tabular numeric percentages, timings, and technical readouts |
| `--ice-accent` | Accent & Status | `#6bc5ff` | Yes | Primary brand/active fill bar, active dot, highlights |
| `--ice-accent-strong` | Accent & Status | `#9ad6ff` | No | Hover/pressed emphasis and high-contrast accent highlights |
| `--ice-accent-soft` | Accent & Status | `#13283c` | No | Subtle tinted chip / badge fill |
| `--ice-accent-glow` | Accent & Status | `rgba(107, 197, 255, 0.28)` | No | Soft radial ambient glow behind active notch states |
| `--ice-active-accent` | Dynamic Runtime | `var(--ice-accent)` *(album-art accent while media is active)* | No | Contrast-safe active media accent for visualizer bars, scrubber fill, and active transport border |
| `--ice-active-glow` | Dynamic Runtime | `var(--ice-accent-glow)` *(album-art glow while media is active)* | No | Dynamic radial ambient glow tinted from the active track's album art |
| `--ice-art-backdrop` | Art Backdrop (D-23) | `0` *(set e.g. `0.85` to enable blurred art image)* | No | Opacity of enlarged blurred album-art backdrop (`V1` opt-in; `V2` gradient wash stays default) |
| `--ice-art-blur` | Art Backdrop (D-23) | `28px` | No | Gaussian blur radius applied to `.art-blur-img` when `--ice-art-backdrop > 0` |
| `--ice-art-wash` | Art Backdrop (D-23) | `1` | No | Opacity multiplier for the 2-stop art-derived radial gradient wash (`V2` default) |
| `--ice-ambient-period` | Art Backdrop (D-25) | `22s` | No | Primary cycle period for the expanded card's ambient wash drift (secondary/tertiary layers run at coprime `1.41×`/`1.73×` periods); `0s` disables motion (`minimal`) |
| `--ice-scrim` | Art Backdrop (D-23) | `linear-gradient(160deg, var(--ice-active-scrim-top, rgba(12, 15, 20, 0.56)) 0%, var(--ice-active-scrim-bottom, rgba(10, 13, 18, 0.84)) 100%)` | No | Luminance-clamped protective gradient scrim holding `--ice-fg` / `--ice-fg-muted` contrast at `≥ 4.5:1` WCAG AA over bright covers |
| `--ice-control-bg` | Translucent Controls (D-23) | `rgba(255, 255, 255, 0.09)` | No | Low-alpha button and chip fill over expanded-card backdrops (`var(--ice-surface)` in `minimal`) |
| `--ice-control-bg-hover` | Translucent Controls (D-23) | `rgba(255, 255, 255, 0.15)` | No | Hover fill for translucent controls |
| `--ice-control-bg-active` | Translucent Controls (D-23) | `rgba(255, 255, 255, 0.17)` | No | Active/selected chip fill (`Pinned`, current playback device, active service) |
| `--ice-control-border` | Translucent Controls (D-23) | `rgba(255, 255, 255, 0.08)` | No | Hairline border around translucent controls (`var(--ice-border)` in `minimal`) |
| `--ice-play-bg` | Translucent Controls (D-23) | `rgba(237, 241, 247, 0.94)` | No | High-contrast primary Play/Pause circular pill background |
| `--ice-play-fg` | Translucent Controls (D-23) | `#0e1116` | No | Foreground icon colour inside primary Play/Pause circular pill |
| `--ice-ok` | Accent & Status | `#4ade80` | Yes | Positive status (device plugged in, mic live) |
| `--ice-warn` | Accent & Status | `#f59e0b` | No | Non-destructive warning state (device unplugged / disconnected) |
| `--ice-danger` | Accent & Status | `#f07148` | Yes | Muted/destructive status (speaker mute, mic mute) |
| `--ice-scale` | Geometry | `1` | No | User scale multiplier for notch geometry |
| `--ice-bar-height` | Geometry | `5px` | No | Thickness of the volume progress bar track/fill |

### 2.1 Settings window tokens (`--ice-set-*`)

The Settings window (`app/ui/src/themes/settings.css`) is a normal Windows 11 window over Mica, so it follows Windows'
**light/dark app mode** instead of the notch theme. Its tokens are separate from the notch's, which describe the always-dark
notch surface. Light-mode values are redefined under `@media (prefers-color-scheme: light)`. Previews inside Settings
(theme and idle-style cards) still draw the notch with the `--ice-*` tokens above.

| Token | Dark | Light | Purpose |
| --- | --- | --- | --- |
| `--ice-set-solid-bg` | `#1c2027` | `#eef2f6` | Page background only where Mica can't show (dev preview); also native `<option>` lists |
| `--ice-set-layer` / `-border` | `rgba(255,255,255,0.035)` / `0.06` | `rgba(255,255,255,0.5)` / `rgba(0,0,0,0.06)` | Content panel to the right of the navigation |
| `--ice-set-card` / `-hover` / `-border` | `rgba(255,255,255,0.05)` / `0.075` / `0.065` | `rgba(255,255,255,0.72)` / `rgba(250,250,250,0.56)` / `rgba(0,0,0,0.07)` | Setting cards, choice cards, lists |
| `--ice-set-control` / `-hover` / `-border` | `rgba(255,255,255,0.06)` / `0.09` / `0.1` | `rgba(255,255,255,0.75)` / `rgba(249,249,249,0.55)` / `rgba(0,0,0,0.13)` | Buttons, selects, text fields |
| `--ice-set-control-strong` | `rgba(255,255,255,0.6)` | `rgba(0,0,0,0.58)` | Toggle and radio outlines when off |
| `--ice-set-nav-hover` / `-active` | `rgba(255,255,255,0.055)` / `0.085` | `rgba(0,0,0,0.04)` / `0.06` | Navigation item fills |
| `--ice-set-divider` | `rgba(255,255,255,0.08)` | `rgba(0,0,0,0.08)` | Rows inside an expanded card |
| `--ice-set-fg` / `-muted` / `-subtle` | `#fff` / `0.7` / `0.5` alpha | `#1b1b1b` / `0.64` / `0.47` alpha | Text: titles, descriptions, placeholders |
| `--ice-set-accent` / `-hover` / `-fg` / `-soft` | `#6bc5ff` / `#8dd3ff` / `#06121c` / 14 % | `#0a6ead` / `#1a7fc0` / `#fff` / 10 % | Selection, toggles on, links, badges. Light mode is darker for ≥ 4.5:1 on white |
| `--ice-set-focus` | `#fff` | `#000` | Keyboard focus ring |
| `--ice-set-ok` / `-warn` / `-danger` | `#6ccb5f` / `#fce100` / `#ff99a4` | `#0f7b0f` / `#8a5300` / `#c42b1c` | "Saved", warnings, errors |
| `--ice-set-info-bg` / `-warn-bg` / `-danger-bg` | tinted 8–10 % | tinted 8–16 % | Info bar fills |
| `--ice-set-keycap` / `-border` | `rgba(255,255,255,0.08)` / `0.16` | `rgba(0,0,0,0.035)` / `0.16` | Shortcut key caps |
| `--ice-set-shadow` | `0 2px 10px rgba(0,0,0,0.28)` | `0 2px 8px rgba(0,0,0,0.08)` | Raised elements |
| `--ice-set-desktop` | dusk gradient | daylight gradient | Wallpaper behind notch previews |
| `--ice-set-radius` / `-radius-sm` | `8px` / `4px` | same | Cards / controls |
| `--ice-set-ease` | `cubic-bezier(0.2, 0, 0, 1)` | same | Short control transitions (off with Windows' animation effects) |

### Supported Colour Formats (Native Pill Parser)

Tokens marked **Synced to Native Pill** are parsed directly by `ice_win::native_dot::NativePillTheme` in addition to the WebView CSS engine. Use any of these CSS colour notations so both renderers match:

- Hex: `#rgb`, `#rrggbb`, `#rrggbbaa` (e.g. `#6bc5ff`, `#6bc5ffff`)
- Functional RGB/RGBA: `rgb(18, 22, 29)`, `rgba(14, 17, 22, 0.9)`
- Keyword: `transparent` (`rgba(0, 0, 0, 0)`)

*(Avoid `color-mix()`, `oklch()`, or CSS variables referencing other variables in the 8 native-synced colour tokens; the WebView handles full CSS, while the native pill falls back to the base theme colour for unsupported functions.)*

---

## 3. Portfolio-Seeded `Ice` Brand Palette (§1.6)

The default `Ice` theme derives directly from the owner's personal brand identity (`kelvinice.github.io`, `planning-discussion.md` §1.6) so the app and brand share one visual family:

| Brand Token | Light Mode (Settings / Light accents) | Dark Mode (Notch default / Dark Settings) | Mapped `--ice-*` Token |
| --- | --- | --- | --- |
| `--accent` | `#026aa2` | `#6bc5ff` | `--ice-accent` |
| `--accent-strong` | `#015580` | `#9ad6ff` | `--ice-accent-strong` |
| `--accent-soft` | `#e6f3fb` | `#13283c` | `--ice-accent-soft` |
| `--bg` | `#f6f7f9` | `#0e1116` | `--ice-bg` |
| `--surface` | `#ffffff` | `#171d27` | `--ice-surface` |
| `--ink` | `#11161f` | `#edf1f7` | `--ice-fg` |
| `--muted` | `#4b5668` | `#98a4b8` | `--ice-fg-muted` |
| `--line` | `#d6dde6` | `#263041` | `--ice-border` (tinted in notch) |
| `--ok` | `#15803d` | `#4ade80` | `--ice-ok` |
| `--signal` | `#c94624` | `#f07148` | `--ice-danger` |

- **Radii scale:** `4 / 6 / 8 / 12 / 16 / 24 px` (compact elements use smaller steps; expanded notch card defaults to `--ice-radius: 22px`).
- **Fonts:** Defaults to `Segoe UI Variable Text` (built into Windows 11, crisp hinting at small notch sizes, zero bundle weight). Optionally use `IBM Plex Sans` / `IBM Plex Mono` for an exact portfolio typographic match.

---

## 4. Bundled Themes (`data-theme`)

Four built-in themes ship out of the box (selected via config / Settings `Appearance` and applied via `[data-theme='<id>']`). The selector matches any element, not only `:root`, so Settings → Appearance can preview every theme side by side. Unlisted tokens inherit from `Ice` (`:root`).

| Theme ID | Character | Override Deltas vs. `Ice` |
| --- | --- | --- |
| **`ice`** *(default)* | Translucent obsidian-ice pill with 2-stop art wash (`V2`) and translucent controls (`V3`) | Base defaults in §2 (`--ice-art-backdrop: 0`, `--ice-art-wash: 1`, `--ice-ambient-period: 22s`) |
| **`midnight`** | Near-pure-black OLED look with darker scrim preserving deep black backdrop | `--ice-bg: rgba(0, 0, 0, 0.96)` · `--ice-surface: rgba(12, 12, 14, 0.96)` · `--ice-rim: rgba(255, 255, 255, 0.24)` · `--ice-art-wash: 0.55` · `--ice-ambient-period: 28s` · `--ice-scrim: linear-gradient(160deg, rgba(0, 0, 0, 0.76) 0%, rgba(0, 0, 0, 0.94) 100%)` |
| **`fluent`** | Neutral Windows 11 Dark-grey chrome with softer tonal wash (`V2`) | `--ice-bg: rgba(24, 24, 27, 0.93)` · `--ice-surface: rgba(36, 36, 40, 0.94)` · `--ice-rim: rgba(107, 197, 255, 0.35)` · `--ice-art-wash: 0.78` · `--ice-ambient-period: 22s` · tonal zinc `--ice-scrim` |
| **`minimal`** | Solid opaque slate with zero art backdrop, zero glow, and bordered solid controls | `--ice-bg: rgb(18, 22, 29)` · `--ice-surface: rgb(26, 32, 42)` · `--ice-rim: transparent` · `--ice-art-backdrop: 0` · `--ice-art-wash: 0` · `--ice-ambient-period: 0s` · `--ice-scrim: none` · `--ice-control-bg: var(--ice-surface)` · `--ice-control-border: var(--ice-border)` |

---

## 5. Dynamic Accent Precedence & Accessibility (D-12)

1. **Accent resolution order:**
   `Album art accent (Phase 3)` → `Windows system accent (optional)` → `Active theme --ice-accent`.
2. **Contrast safety:** Rust extracts dynamic album-art accents in Phase 3 and clamps HSL lightness/saturation before sending them to the UI so text and icons maintain **WCAG AA contrast (≥ 4.5:1)** over dark surfaces and readable non-text contrast (≥ 3:1) over desktop wallpapers.
3. **Never rely on colour alone:** Speaker mute and microphone mute states always swap/slash distinct glyphs and show explicit status labels (`MUTE`, `MIC OFF`) alongside `--ice-danger`.

---

## 6. User Themes, Validation & Strict CSP (D-04, D-11)

- **Location:** Drop `.css` files into:
  ```text
  %APPDATA%\IceAudioManager\themes\<theme-name>.css
  ```
- **Validation before applying:** User themes are validated before activation. Invalid syntax falls back safely to the last valid theme and raises an in-app notice rather than breaking the notch.
- **Strict CSP (zero network privacy promise):** User stylesheets must contain **no remote `url(...)` or `@import`** rules. Tauri enforces a strict Content Security Policy with zero external network origins so loading any theme produces zero network requests.

---

## 7. Example User Theme (`arctic-mint.css`)

Save this file as `%APPDATA%\IceAudioManager\themes\arctic-mint.css`:

```css
/*
 * Arctic Mint — Example custom user theme for Ice Audio Manager
 * Overrides :root tokens so both the WebView notch and the native wake/Lite pill adopt the palette.
 */
:root {
  /* Deep emerald-slate surface + subtle teal specular rim */
  --ice-bg: rgba(11, 20, 22, 0.92);
  --ice-surface: rgba(18, 33, 36, 0.94);
  --ice-border: rgba(94, 234, 212, 0.24);
  --ice-rim: rgba(153, 246, 228, 0.5);
  --ice-shadow: 0 12px 30px rgba(0, 0, 0, 0.52), 0 2px 8px rgba(0, 0, 0, 0.36);
  --ice-radius: 20px;

  /* Readable cool-white foregrounds + tabular digits */
  --ice-fg: #ecfeff;
  --ice-fg-muted: #99b8bd;
  --ice-font: 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif;
  --ice-mono: 'Cascadia Mono', 'Consolas', monospace;

  /* Mint accent + coral mute warning (matched in native pill) */
  --ice-accent: #5eead4;
  --ice-accent-strong: #99f6e4;
  --ice-accent-soft: #133534;
  --ice-accent-glow: rgba(94, 234, 212, 0.26);
  --ice-ok: #4ade80;
  --ice-danger: #fb7185;

  /* Slightly thicker progress fill bar */
  --ice-scale: 1;
  --ice-bar-height: 6px;
}
```

---

## 8. Contributor Checklist (`code-practices.md` §5)

- Svelte components must **only reference `--ice-*` custom properties** — never hardcode literal hex/rgba colours or corner radii in `.svelte` `<style>` blocks.
- Keep native pill constants in `crates/ice-win/src/native_dot.rs` (`NativePillTheme::ice()`, `midnight()`, `fluent()`, `minimal()`) synchronized with `base.css`.
- Whenever a new `--ice-*` variable is added to `base.css`, document its name, default, category, and native-pill sync behaviour in this file in the same PR.
