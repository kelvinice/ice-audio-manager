# D-23 · Claude's vote

> Sealed at proposal time (§0.4 rule 2). Claude wrote the entry; it contains no scores or verdicts.

## Scores

| Option | Score | One-line reason |
|---|---|---|
| V1 · Blurred album-art backdrop | +2 | The reference look, achievable inside D-12's constraints: it blurs our own image, not the desktop |
| V2 · Art gradient wash | +1 | Cheaper, and the base for a tonal light variant and for themes that switch V1 off |
| V3 · Translucent controls | +1 | Part of the look, but icon contrast on busy covers needs care |
| V4 · Cleanliness pass | +2 | The strongest lesson from both references; a 384×178 card can't afford clutter |
| V5 · See-through desktop blur | −1 | D-12's finding stands (CSS can't, Acrylic jitters on a morphing window); not worth an undocumented API |
| V6 · Lyrics styling | +1 | Only meaningful if D-22 adopts lyrics |

## Strongest argument against my top choices (V1, V4)

The references are full-screen phone players; ours is a strip at the top of someone's desktop. A vivid, blurred colour wash
there could be distracting ("one quiet companion", as Auralis put it), and at 384×178 a blurred cover may read as a
muddy blob rather than an atmosphere. A restrained gradient (V2) might look cleaner at notch scale. And a "cleanliness
pass" that hides the device row behind "…" would undo D-18, which put picked devices in the card on purpose.

## New option or amendment

- **V1 in the expanded card only**, never in the compact pills or activities, which stay calm. An intensity token
  (`--ice-art-backdrop-opacity`) with a subtle default.
- **Automatic scrim:** compute the art's average luminance in Rust next to the accent (`ice_core::media`), so light covers
  get a stronger scrim and text keeps WCAG AA without per-theme guessing.
- **V4 keeps D-18's device row visible** and only quiets its styling (smaller, lower-contrast chips).

## Evidence I checked myself

- `docs/theming.md:14`: the D-12 note. `backdrop-filter` can't blur desktop pixels behind a transparent WebView2 window;
  DWM Acrylic jitters on a morphing HWND. It says nothing against blurring an image inside the WebView.
- `crates/ice-core/src/media.rs:110,165,187`: `dominant_bgra`, `readable_accent`, `accent_for_art` already exist. A palette
  or a luminance value is a small extension.
- `crates/ice-win/src/media.rs` tests: art is fitted to ≤ 128 px (`fit(640, 640, 128) == (128, 128)`), so the backdrop
  source is tiny. Blur hides that, and scaling cost is negligible.
- `app/ui/src/lib/notch/geometry.ts`: the expanded card is 384×178.
- Reference images: `docs/reference/Echo-Music/Screenshots/lyrics.png` (art wash + scrim + fading lyric lines + tinted
  round buttons), `docs/reference/BitChord/Banner.png` (blurred cover backdrops, translucent controls).

## Position

Adopt V1 (expanded card only, subtle by default, auto-scrim), V2 as its fallback and light-variant base, V4 with D-18 intact,
V3 where contrast allows. V6 waits for D-22. Leave D-12's no-see-through-blur decision as it is.

## Revision round (after reveal, 2026-11-23)

| Option | Score | Reason |
|---|---|---|
| V1 · Blurred art backdrop | **+1 (was +2, changed after reveal)** | Gemini's 384×178 mocks: light and white covers blur into pale slate unless the scrim nearly hides the image. That's the risk I raised myself; now it's shown |
| V2 · Art gradient wash | **+2 (was +1, changed after reveal)** | Keeps WCAG AA on bright and dark covers, needs no CSS blur over `clip-path`, and gives Fluent light its tonal variant |
| V3, V4, V5, V6 | unchanged (+1, +2, −1, +1) | |

Adopt Gemini's amendment (V2 default, V1 opt-in via `--ice-art-backdrop`, scrim clamped from the art's luminance) and Codex's
(one backdrop treatment per theme; art layer has no pointer events; crossfade only on art-ID change; immediate swap with reduced
motion). V4 keeps D-18's device row visible (both reviewers raised this).
