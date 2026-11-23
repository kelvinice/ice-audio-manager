# D-23 · Codex's vote

## Scores

| Option | Score | One-line reason |
|---|---|---|
| V1 · Blurred cover | +1 | The existing local art can supply the soft photographic field in both banners with a single filtered image behind the controls. |
| V2 · Palette wash | +1 | A gradient gives Fluent light a way to retain its light surface even when a cover is almost black. |
| V3 · Translucent controls | +1 | A shared control tint can group secondary actions while the scrim keeps icons legible on a bright cover. |
| V4 · Cleanliness pass | +2 | At 384 × 178, keeping the track, transport and favourite outputs immediately identifiable matters more than adding another effect. |
| V5 · Desktop blur | −2 | CSS has no desktop pixels to filter in this host, and applying a native material to the fixed rectangular HWND would give it a different outline from the animated pill. |
| V6 · Lyric emphasis | +1 | If D-22 passes lyrics, one bright current line with a quieter neighbor fits a dedicated lyrics view and leaves the main controls reachable. |

## Strongest argument against my top choice

A cleaner screenshot can make a worse audio manager. Moving the favourite outputs into an overflow menu would add a click to every switch. Keep the current output and favourite device actions directly accessible and judge V4 with a long title and several favourites present.

## New option or amendment

Give each theme one backdrop treatment at a time. Try the clipped cover in Ice, a darker scrim in Midnight, a tonal wash in Fluent light and a solid surface in Minimal. Give the art layer no pointer events. Crossfade opacity only on an art-ID change, discard the outgoing layer on completion or interruption, and swap immediately for reduced motion. Missing art uses the theme surface.

## Evidence I checked myself

- I inspected both banners and Echo's lyrics screenshot. Echo puts the bright current line over a broad dark patch, and BitChord keeps the title legible below a bright yellow portrait. The large quiet patch behind text is worth preserving at notch scale. Their phone layouts devote substantially more vertical space to lyrics than our card can spare.
- The notch builder in `app/src-tauri/src/main.rs` creates a transparent WebView. `app/ui/src/notch/NotchApp.svelte:28` clips the pill inside that window and line 128 applies a stage shadow. `crates/ice-win/src/overlay_win.rs` applies Mica to Settings. A window-level material would need separate geometry synchronized with the pill. I found no change that overturns D-12.
- The CSS draft constructs the backdrop from painted document ancestors. WebView2's transparent background lets underlying host content show through at composition time. From those contracts I infer that a CSS backdrop filter on this page cannot sample an unrelated desktop window. This was a source review. [CSS Filter Effects, Backdrop Root](https://drafts.csswg.org/filter-effects-2/#BackdropRoot) · [WebView2 background contract](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2controller2#put_defaultbackgroundcolor)
- `AlbumArtTile.svelte` resolves an art ID through `iceart`, and `app/src-tauri/src/art.rs` serves PNG bytes. V1 can reuse the same URL without a new command, HWND, capture session or frame stream. Another image request could still reach that handler. In the renderer, enlarging and filtering the image can require an intermediate surface, and a crossfade composites two images temporarily. At 150%, the same card covers 2.25 times as many physical pixels. Actual allocation, caching and GPU time are unmeasured and belong to Phase 6.

## Position

Ship the hierarchy and theme tokens first. Keep art effects local to the expanded WebView. The native idle pill and window placement need no changes for that design.
