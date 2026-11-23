# D-20 · Gemini's vote

> Re-cast on 2026-11-23 under blind voting (§0.4), including the **Addendum for late option A2b**, after inspecting `docs/reference/screens/auralis-store-*.png`, `notchbox-expanded.png`, `geometry.ts`, `NotchApp.svelte`, `overlay_win.rs`, `native_dot.rs`, and `docs/code-practices.md` §1. *(Replaces the pre-§0.4 anchored vote; adds A2b = `−1` and corrects our earlier unverified §1 clipboard citation on R4.)*

## Scores

#### Part A *pick one* (placement)

| Option | Score | One-line reason |
|---|---|---|
| P1 · Fixed anchors (tray/Settings) | +1 | Safe, accessible baseline that solves top-centre collisions with zero Win32 drag-loop risk |
| P2 · Drag + snap to anchors | +2 | Fulfills the owner's tactile drag-and-re-pin request while keeping layout bounded to testable anchors |
| P3 · Free placement anywhere | −1 | Creates off-screen clipping when expanding to `384×178` and ambiguous expansion directions across DPI changes |

#### Part A2 *proposal* (six top/bottom anchors; one `anchor_rect` for both windows)

| Option | Score | One-line reason |
|---|---|---|
| A2 · Six top/bottom work-area anchors | +2 | Keeps all 6 horizontal activities intact, avoids Win11 side-snap, and removes the `overlay_win`/`native_dot` placement twin |

#### Addendum · Late option A2b *proposal* (add left/right edges with a vertical layout, Auralis-style; added 2026-11-23)

| Option | Score | One-line reason |
|---|---|---|
| A2b · Left/right edges + vertical layout | −1 | Doubles layout/QA surface across 6 WebView activities + native pill, forces dynamic HWND canvas resizing, and creates a dual-monitor inner-edge hover trap |

#### Part B *pick many* (ideas from other notch apps)

| Option | Score | One-line reason |
|---|---|---|
| R1 · Attached vs floating island | +1 | Clean style choice on top/bottom anchors, with a small SVG `<path>` + native SDF corner-radius update |
| R2 · Optional modules | +2 | Enforces §2 directly: disabled activities unmount and run zero timers, subscriptions, or IPC |
| R3 · File shelf / drop zone | −1 | Outside D-05 audio scope and risks pulling Shell/OLE thumbnail providers into our working set |
| R4 · Clipboard history | −2 | **Block:** subscribing to global clipboard changes reads passwords/2FA codes with zero audio purpose, contradicting our core user-trust pillar (§1.3) and D-05 scope |

## Strongest argument against my top choice

- **Against P2 (drag + snap to anchors):** Both `overlay_win.rs:42` (`WS_EX_NOACTIVATE` + `WM_MOUSEACTIVATE → MA_NOACTIVATE`) and `native_dot.rs:943` (`WS_EX_LAYERED | WS_EX_NOACTIVATE`) are strictly non-activating. Invoking Windows' default `WM_SYSCOMMAND(SC_MOVE)` loop risks activating the window (breaking zero-focus-steal QA `F4`), while writing a manual pointer-capture drag loop (`SetCapture` + `WM_MOUSEMOVE` + `SetWindowPos`) has to handle mixed-DPI transitions (`100%` ↔ `150%`), avoid fighting short left-clicks on a `92×18` pill, and depends on D-19 A1 landing first. P1 delivers 80% of the collision relief immediately with none of that drag-loop complexity.

## New option or amendment

1. **Two-step rollout (P1 first, P2 gated on spike):** Land `ice_core::anchor_rect` + tray/Settings anchor picker (**P1**) first so users can move off top-centre right away; ship **P2** drag-to-snap once the non-activating mixed-DPI drag spike passes `F4` zero-focus-steal checks.
2. **Design takeaway from Auralis & NotchBox (no side-edge rewrite needed):** From `auralis-store-horizontal.png` and `auralis-store-violet.png`, the visual appeal comes largely from **whole-surface colour themes** (including a crisp Light / Frosted Ice theme using existing `--ice-*` tokens in `docs/theming.md`) and a clean pin button—not vertical side docks. Adding a Light theme preset + NotchBox's calm empty-media wording in Phase 3/4 polish captures that polish cheaply.

## Evidence I checked myself

- `docs/reference/screens/auralis-store-vertical-anchors.png` & `auralis-store-horizontal.png`: Auralis's side-edge card is a `~240×320` floating portrait media player widget, not a narrow transient pill. By contrast, our notch has 6 horizontal pills (`app/ui/src/lib/notch/geometry.ts:20-68`: `116×28`/`196×28` idle, `236×38` volume, `296×44` switch, `268×44` plug, `204×38` mic, `296×48` track), a `384×178` expanded card (`geometry.ts:30`) with horizontal D-18 device chips (`ExpandedDeviceRow.svelte`), and 4 horizontal native pills (`crates/ice-win/src/native_dot.rs:38-47`), all rendered inside a fixed `CANVAS_W = 420, CANVAS_H = 190` (`geometry.ts:4-5`) WebView canvas.
- `app/ui/src/notch/NotchApp.svelte:107-120` & `:153-178`: verified two concrete UI facts:
  1. Bottom anchors (`A2`) need only flipping `TOP_PAD` → bottom alignment on `.pill-slot`, `<rect y>`, `computeClipPath` (`geometry.ts:79-88`), and `.accent-glow`'s radial origin (`50% 0%` → `50% 100%`, `NotchApp.svelte:159`).
  2. For `R1` ("Attached" flat outer corners), `<svg><rect rx ry>` (`NotchApp.svelte:108-119`) forces uniform corner radii on all 4 corners, so `"Attached"` requires replacing `<rect>` with an SVG `<path>` (CSS `clip-path: inset(... round 0px 0px R R)` already supports asymmetric corners) plus per-corner radii in `native_dot.rs`'s `sd_round_box` (`native_dot.rs:1319`).
- `crates/ice-win/src/overlay_win.rs:124-125` vs `crates/ice-win/src/native_dot.rs:946-947, 1156-1157`: confirms the duplicate top-centre math (`monitor.left + (monitor.width - w) / 2`, `top + offset`) that `ice_core::anchor_rect` will eliminate.
- `docs/planning-discussion.md:1710` (Q-07): owner runs dual monitors (`DISPLAY1` 144 Hz + `DISPLAY2` 75 Hz), meaning an inner left/right edge anchor sits directly on the cross-monitor cursor path.
- `docs/code-practices.md:9-23`: verified directly that §1 does **not** literally list clipboard APIs (correcting my earlier anchored vote); the block on R4 rests on §1.3's user-trust goal and D-05 scope, not a literal §1 ban.

## Position

**Part A & A2:** I prefer **P2 (`+2`)** built incrementally on **P1 (`+1`)** and **A2 (`+2`)**. Top and bottom work-area anchors solve both titlebar/browser-tab overlap and Windows 11 top-centre Snap Layouts while preserving every horizontal activity layout and eliminating the `overlay_win`/`native_dot` placement twin via `ice_core::anchor_rect`.

**Addendum — Late option A2b (`−1`):** Prefer not to build left/right vertical anchors in Phase 4. Looking at `auralis-store-vertical-anchors.png`, vertical side anchoring only looks natural as a tall portrait media card, which doesn't translate cleanly to our short horizontal device-switch toasts (`"Headphones (Arctis Nova 7)"`), D-18 device chip row, or `native_dot.rs` pills. It would double our UI/native rendering and screenshot QA matrix, require dynamically resizing the `420×190` WebView HWND whenever switching between top/bottom and left/right, collide with Win11 Aero Snap on outer edges, and trigger constant accidental hovers on the shared inner edge between dual monitors. It's not a hard block (`−2`), so defer unless users ask for a portrait side dock after A2 ships.

**Part B:** Strong support for **R2 (`+2`)** (disabled activities unmount and drop subscriptions/timers completely) and **R1 (`+1`)** (noting the SVG `<path>` + `sd_round_box` asymmetric-corner work). **R3 (`−1`)** is unrelated scope. **R4 (`−2`, block):** Even though §1 doesn't explicitly list clipboard APIs, an audio manager listening to every Windows clipboard change silently touches passwords and private data for a non-audio feature—an unacceptable trust liability under §1.3.
