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
| `--ice-drop-shadow` | Surface | `0 8px 18px rgba(0, 0, 0, 0.45)` | No | Composited `drop-shadow(...)` filter under the floating notch stage |
| `--ice-radius` | Surface | `22px` | No | Expanded card corner radius (compact pills compute capsule radii dynamically) |
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
| `--ice-ok` | Accent & Status | `#4ade80` | Yes | Positive status (device plugged in, mic live) |
| `--ice-warn` | Accent & Status | `#f59e0b` | No | Non-destructive warning state (device unplugged / disconnected) |
| `--ice-danger` | Accent & Status | `#f07148` | Yes | Muted/destructive status (speaker mute, mic mute) |
| `--ice-scale` | Geometry | `1` | No | User scale multiplier for notch geometry |
| `--ice-bar-height` | Geometry | `5px` | No | Thickness of the volume progress bar track/fill |

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

Four built-in themes ship out of the box (selected via config / Settings `Appearance` and applied via `:root[data-theme='<id>']`). Unlisted tokens inherit from `Ice` (`:root`).

| Theme ID | Character | Override Deltas vs. `Ice` |
| --- | --- | --- |
| **`ice`** *(default)* | Translucent obsidian-ice pill with cyan rim and ambient glow | Base defaults in §2 |
| **`midnight`** | Near-pure-black high-contrast OLED/DynamicWin look | `--ice-bg: rgba(0, 0, 0, 0.96)` · `--ice-surface: rgba(12, 12, 14, 0.96)` · `--ice-border: rgba(255, 255, 255, 0.14)` · `--ice-rim: rgba(255, 255, 255, 0.24)` · `--ice-accent-glow: rgba(255, 255, 255, 0.12)` |
| **`fluent`** | Neutral Windows 11 Dark-grey chrome matching stock Flyouts | `--ice-bg: rgba(24, 24, 27, 0.93)` · `--ice-surface: rgba(36, 36, 40, 0.94)` · `--ice-border: rgba(255, 255, 255, 0.12)` · `--ice-rim: rgba(107, 197, 255, 0.35)` · `--ice-accent-glow: rgba(107, 197, 255, 0.2)` |
| **`minimal`** | Solid opaque slate with zero glow or specular rim (lowest GPU overdraw) | `--ice-bg: rgb(18, 22, 29)` · `--ice-surface: rgb(26, 32, 42)` · `--ice-border: rgba(255, 255, 255, 0.14)` · `--ice-rim: transparent` · `--ice-shadow: 0 4px 12px rgba(0, 0, 0, 0.35)` · `--ice-drop-shadow: 0 4px 10px rgba(0, 0, 0, 0.3)` · `--ice-accent-glow: transparent` |

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
