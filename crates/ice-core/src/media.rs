//! Now-playing types and album-art colour maths (Phase 3, F15–F16). No Windows dependencies.
//!
//! The notch tints its glow with the album art's accent colour. Raw art colours are often too dark, too
//! grey or too neon to read on the dark Ice surface, so [`readable_accent`] adjusts them until text in that
//! colour meets WCAG AA (4.5:1) on the notch background (Gemini's D-12 refinement).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
    Changing,
    Opened,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// `#rrggbb`.
    #[must_use]
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// The Ice theme's dark notch surface (`--bg` dark, docs/theming.md).
pub const ICE_SURFACE: Rgb = Rgb::new(0x0e, 0x11, 0x16);
/// The brand accent, used when there's no album art (docs/theming.md).
pub const ICE_ACCENT: Rgb = Rgb::new(0x6b, 0xc5, 0xff);
/// Calibration surface for art accents, a little lighter than the lightest built-in dark notch surface
/// (`fluent`'s translucent `--ice-surface`, `rgba(36, 36, 40, 0.94)`, over a white desktop ≈ `#313135`).
/// An accent readable here is readable on every darker theme surface too (review MED-04).
pub const ACCENT_SURFACE: Rgb = Rgb::new(0x3d, 0x3d, 0x3d);
/// WCAG crossover: below this surface luminance, white text contrasts more than black.
const DARK_SURFACE_MAX_LUMINANCE: f64 = 0.179_13;

/// Album art, downscaled and cached by content hash (served to the UI as `iceart://<id>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct ArtInfo {
    /// Content hash of the downscaled image; stable across re-reads of the same art.
    pub id: String,
    pub width: u32,
    pub height: u32,
    /// Readable accent colour derived from the art.
    pub accent: Rgb,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    /// Source app, e.g. `Spotify.exe`.
    pub app_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub status: PlaybackStatus,
    pub position_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub art: Option<ArtInfo>,
}

/// FNV-1a 64-bit hash as 16 hex digits. Small, dependency-free and stable; used to name cached art.
#[must_use]
pub fn content_id(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Cache id for downscaled art: the size and the pixels, so a transposed image never shares an id (MED-08).
#[must_use]
pub fn art_id(width: u32, height: u32, bgra: &[u8]) -> String {
    let mut bytes = Vec::with_capacity(8 + bgra.len());
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(bgra);
    content_id(&bytes)
}

/// The most prominent *colourful* colour of a BGRA8 image (4 bytes per pixel), or `None` if the image has
/// essentially no colour (black-and-white photos, white covers). Colours are bucketed at 4 bits per channel
/// and weighted by saturation, so a small vivid area beats a large grey one, but near-black and near-white
/// pixels barely count. Colourless art should use the brand accent, not a white or grey "tint"
/// (seen with the owner's Spotify, 2026-11-22: a mostly white cover gave #fffffe).
#[must_use]
pub fn dominant_bgra(pixels: &[u8]) -> Option<Rgb> {
    let mut weight = vec![0f64; 4096];
    let mut sums = vec![[0f64; 3]; 4096];
    let mut n = 0f64;
    let (pixels, _) = pixels.as_chunks::<4>();
    for &[b, g, r, a] in pixels {
        if a < 128 {
            continue;
        }
        let (rf, gf, bf) = (f64::from(r), f64::from(g), f64::from(b));
        n += 1.0;
        let (_, s, l) = rgb_to_hsl(Rgb::new(r, g, b));
        // Saturation counts most; very dark/light pixels (text, borders, sky) count little.
        let w = s * (1.0 - (2.0 * l - 1.0).abs()).max(0.05);
        if w < 0.05 {
            continue;
        }
        let i = (usize::from(r >> 4) << 8) | (usize::from(g >> 4) << 4) | usize::from(b >> 4);
        weight[i] += w;
        sums[i][0] += rf * w;
        sums[i][1] += gf * w;
        sums[i][2] += bf * w;
    }
    let total: f64 = weight.iter().sum();
    // Less than ~3% "colour weight" per pixel = a colourless image; a few stray coloured pixels don't count.
    if n == 0.0 || total / n < 0.03 {
        return None;
    }
    let to_u8 = |v: f64| v.round().clamp(0.0, 255.0) as u8;
    let i = (0..weight.len()).max_by(|&a, &b| weight[a].total_cmp(&weight[b]))?;
    let w = weight[i];
    Some(Rgb::new(to_u8(sums[i][0] / w), to_u8(sums[i][1] / w), to_u8(sums[i][2] / w)))
}

/// WCAG relative luminance.
#[must_use]
pub fn luminance(c: Rgb) -> f64 {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio (1–21).
#[must_use]
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Adjusts an accent so it reads on `surface` with at least 4.5:1 contrast (WCAG AA for text). A colour that
/// already passes is returned unchanged; otherwise the hue is kept, neon saturation is tamed, and lightness
/// moves in small steps towards white (dark surface) or black (light surface) until it passes.
#[must_use]
pub fn readable_accent(c: Rgb, surface: Rgb) -> Rgb {
    if contrast(c, surface) >= 4.5 {
        return c;
    }
    let (h, s, mut l) = rgb_to_hsl(c);
    let s = s.min(0.85);
    let lighten = luminance(surface) < DARK_SURFACE_MAX_LUMINANCE;
    let mut out = hsl_to_rgb(h, s, l);
    // 0.005 steps: at most 200 to reach white/black, which always passes against a surface on that side.
    for _ in 0..=200 {
        if contrast(out, surface) >= 4.5 {
            break;
        }
        l = if lighten { (l + 0.005).min(1.0) } else { (l - 0.005).max(0.0) };
        out = hsl_to_rgb(h, s, l);
    }
    out
}

/// The accent to use for some art: readable on every built-in dark notch surface ([`ACCENT_SURFACE`]), or the
/// brand accent without (colourful) art.
#[must_use]
pub fn accent_for_art(bgra: Option<&[u8]>) -> Rgb {
    bgra.and_then(dominant_bgra).map_or(ICE_ACCENT, |c| readable_accent(c, ACCENT_SURFACE))
}

fn rgb_to_hsl(c: Rgb) -> (f64, f64, f64) {
    let (r, g, b) = (f64::from(c.r) / 255.0, f64::from(c.g) / 255.0, f64::from(c.b) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < f64::EPSILON {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if (max - r).abs() < f64::EPSILON {
        ((g - b) / d).rem_euclid(6.0)
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(to(r), to(g), to(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(pixels: &[(Rgb, usize)]) -> Vec<u8> {
        pixels.iter().flat_map(|(c, n)| std::iter::repeat_n([c.b, c.g, c.r, 255], *n).flatten()).collect()
    }

    #[test]
    fn vivid_area_beats_a_larger_grey_one() {
        let img = image(&[(Rgb::new(128, 128, 128), 900), (Rgb::new(220, 40, 40), 100)]);
        let c = dominant_bgra(&img).unwrap();
        assert!(c.r > 180 && c.g < 80 && c.b < 80, "{c:?}");
    }

    #[test]
    fn colourless_art_has_no_dominant_colour_and_uses_the_brand_accent() {
        let grey = image(&[(Rgb::new(100, 100, 100), 50), (Rgb::new(140, 140, 140), 50)]);
        assert_eq!(dominant_bgra(&grey), None);
        assert_eq!(dominant_bgra(&[]), None);
        assert_eq!(accent_for_art(Some(&grey)), ICE_ACCENT);
        // A white cover with a few stray coloured pixels is still colourless.
        let white = image(&[(Rgb::new(250, 250, 248), 990), (Rgb::new(200, 60, 60), 10)]);
        assert_eq!(accent_for_art(Some(&white)), ICE_ACCENT);
    }

    #[test]
    fn readable_accent_meets_wcag_on_the_ice_surface() {
        for c in [
            Rgb::new(10, 20, 80),
            Rgb::new(0, 0, 0),
            Rgb::new(255, 0, 255),
            Rgb::new(120, 120, 120),
            Rgb::new(250, 250, 250),
        ] {
            let a = readable_accent(c, ICE_SURFACE);
            assert!(contrast(a, ICE_SURFACE) >= 4.5, "{c:?} -> {a:?} ({:.2})", contrast(a, ICE_SURFACE));
        }
        // Hue is kept: a dark navy becomes a lighter blue, not grey.
        let a = readable_accent(Rgb::new(10, 20, 80), ICE_SURFACE);
        assert!(a.b > a.r && a.b > a.g, "{a:?}");
        // And it works for light surfaces too.
        let light = Rgb::new(0xf6, 0xf7, 0xf9);
        assert!(contrast(readable_accent(Rgb::new(255, 230, 0), light), light) >= 4.5);
    }

    #[test]
    fn mid_tone_surfaces_get_dark_accents() {
        // Above the WCAG crossover (L ≈ 0.179) black contrasts more than white, so the accent darkens (MED-04).
        for surface in [Rgb::new(0x80, 0x80, 0x80), Rgb::new(0x99, 0x99, 0x99), Rgb::new(0xb0, 0xb0, 0xb0)] {
            for c in [Rgb::new(255, 255, 255), Rgb::new(255, 200, 0), Rgb::new(100, 150, 255)] {
                let a = readable_accent(c, surface);
                assert!(contrast(a, surface) >= 4.5, "{c:?} on {surface:?} -> {a:?} ({:.2})", contrast(a, surface));
            }
        }
    }

    #[test]
    fn art_accents_read_on_every_dark_theme_surface() {
        // Composited over a white desktop (the worst case): ice --ice-bg, ice --ice-surface, the review's old
        // fluent --ice-bg, today's fluent --ice-surface; plus the calibration and the opaque Ice surface.
        let surfaces = [
            Rgb::new(0x26, 0x29, 0x2d),
            Rgb::new(0x2a, 0x2f, 0x38),
            Rgb::new(0x3b, 0x3b, 0x3b),
            Rgb::new(0x31, 0x31, 0x35),
            ACCENT_SURFACE,
            ICE_SURFACE,
        ];
        for c in [Rgb::new(10, 20, 80), Rgb::new(180, 20, 20), Rgb::new(220, 40, 40), Rgb::new(130, 30, 180)] {
            let a = accent_for_art(Some(&image(&[(c, 100)])));
            for s in surfaces {
                assert!(contrast(a, s) >= 4.5, "{c:?} -> {a:?} on {s:?} ({:.2})", contrast(a, s));
            }
        }
        assert!(contrast(ICE_ACCENT, ACCENT_SURFACE) >= 4.5);
    }

    #[test]
    fn accents_that_already_pass_are_left_alone() {
        // Gold already reads on the darkest surface; it must not be washed out (MED-04).
        let gold = Rgb::new(0xb8, 0x86, 0x0b);
        assert_eq!(readable_accent(gold, ICE_SURFACE), gold);
    }

    #[test]
    fn art_ids_include_the_size() {
        let px = [0u8; 128 * 64 * 4];
        assert_ne!(art_id(128, 64, &px), art_id(64, 128, &px));
        assert_eq!(art_id(128, 64, &px), art_id(128, 64, &px));
    }

    #[test]
    fn no_art_means_brand_accent_and_ids_are_stable() {
        assert_eq!(accent_for_art(None), ICE_ACCENT);
        assert_eq!(content_id(b"abc"), content_id(b"abc"));
        assert_ne!(content_id(b"abc"), content_id(b"abd"));
        assert_eq!(content_id(b"").len(), 16);
        assert_eq!(Rgb::new(0x6b, 0xc5, 0xff).hex(), "#6bc5ff");
    }
}
