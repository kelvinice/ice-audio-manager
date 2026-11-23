# D-20 · Gemini's vote

> Cast on 2026-11-23, **before** blind voting (§0.4) was adopted. Moved here verbatim from the D-20 entry so the
> remaining voters can vote without seeing it. Gemini voted after reading Claude's proposal and scores, so
> these two votes are anchored. Both may be re-cast with the §0.4 checklist before the reveal.

## Position
**Part A: P1 +1, P2 +2, P3 −1. Part A2: +2. Part B: R1 +1, R2 +2, R3 −1, R4 −2.**
- **P2 (+2) & P1 (+1):** Drag-to-move with magnetic snap to fixed edge anchors resolves collisions with maximised titlebars and Windows 11 Snap Layouts while keeping cross-monitor re-pinning intuitive; `P1` stays available via tray/Settings for keyboard users. **P3 (−1):** Free placement anywhere creates off-screen clipping when the pill expands to `384×178` (`geometry.ts`) and arbitrary expansion-direction edge cases across DPI changes.
- **Part A2 (+2):** Six top/bottom work-area anchors (`top-{left,centre,right}`, `bottom-{left,centre,right}`) avoid Win11 side-snap collisions and avoid re-architecting horizontal activities into vertical layouts. In `NotchApp.svelte`, bottom anchors only need an edge flag (`top` vs `bottom`) on `.stage` to anchor `.pill-slot`, `.rim-svg`, and `clip-path` to the bottom edge, flip `.accent-glow`'s radial origin (`50% 0%` → `50% 100%`), and mirror flat outer corner radii. Centralising `ice_core::anchor_rect(anchor, work_area, size)` removes the placement twin between `overlay_win` and `native_dot`.
- **R1 (+1):** Attaching (`0 px` gap, flat edge-facing outer corners) vs floating island (gap + full capsule radius) maps cleanly to CSS and native corner tokens on both top and bottom edges.
- **R2 (+2):** Essential for `code-practices.md` §2: disabling a module in Settings → Activities must unmount its subtree and tear down its timers, IPC listeners, and subscriptions—not just hide it.
- **R3 (−1):** A file shelf/drop zone drags us out of audio scope and risks pulling Shell COM/thumbnail providers into the working set.
- **R4 (−2, block):** Monitoring clipboard history in an audio manager triggers AV/privacy heuristics and violates `code-practices.md` §1.

## Scores (rows as they were in the vote tables)

#### Vote · Part A *pick one* (placement)

| Gemini | +1 | +2 | −1 |

#### Vote · Part A2 *proposal* (top + bottom anchors, six in all; one `anchor_rect` for both windows)

| Gemini | +2 | Six top/bottom work-area anchors avoid side-snap collisions and vertical activity rewrites; WebView needs only top/bottom origin + glow flips, and `ice_core::anchor_rect` removes the §11 twin |

#### Vote · Late option A2b *proposal* (add left/right edges with a vertical layout, Auralis-style; added 2026-11-23)

| Gemini | | |

#### Vote · Part B *pick many* (ideas from other notch apps)

| Gemini | +1 | +2 | −1 | −2 *(§1: clipboard monitoring trips AV/trust rules & is outside audio scope)* |

