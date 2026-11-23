//! GDI glyph rasterizer for the native idle / `Lite` pill (D-19 Part B spike).
//!
//! Renders real Windows 11 typography (`Segoe UI Variable Text`, `Segoe UI` fallback) and `Segoe Fluent Icons` into a
//! greyscale alpha mask (`0.0..=1.0`) without loading Direct2D or DirectWrite DLLs. Drawing white (`0xffffff`) text
//! onto a pure-black scratch DIB with symmetric ClearType (`CLEARTYPE_NATURAL_QUALITY`) and averaging the RGB
//! subpixel channels gives crisp achromatic glyph coverage at 11–12 px without ClearType color fringes over the
//! translucent pill surface.

use core::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{COLORREF, SIZE};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLEARTYPE_NATURAL_QUALITY, CLIP_DEFAULT_PRECIS, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, DeleteDC, DeleteObject, ETO_OPTIONS,
    ExtTextOutW, FF_SWISS, FONT_QUALITY, FW_MEDIUM, FW_NORMAL, GetTextExtentPoint32W, HBITMAP, HDC, HFONT, HGDIOBJ,
    OUT_TT_PRECIS, SelectObject, SetBkColor, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::core::{PCWSTR, w};

/// Horizontal alignment inside a logical bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Align {
    Center,
    Right,
}

/// Which font stack to select when rasterizing a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FontFace {
    Ui,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "verified for Segoe Fluent Icons spike; wired when IdleContent lands")
    )]
    FluentIcon,
}

struct DcGuard(HDC);
impl Drop for DcGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: owns the memory DC created by `CreateCompatibleDC`.
            let _ = unsafe { DeleteDC(self.0) };
        }
    }
}

struct SelectGuard {
    dc: HDC,
    old: HGDIOBJ,
}
impl Drop for SelectGuard {
    fn drop(&mut self) {
        if !self.old.is_invalid() {
            // SAFETY: restores the prior GDI object before deleting the temporary bitmap/font.
            let _ = unsafe { SelectObject(self.dc, self.old) };
        }
    }
}

struct ObjGuard(HGDIOBJ);
impl Drop for ObjGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: owns the temporary DIB bitmap or GDI `HFONT`.
            let _ = unsafe { DeleteObject(self.0) };
        }
    }
}

/// Text/icon coverage buffer matching the pill DIB dimensions (`w * h`, `0.0..=1.0` per pixel).
pub(super) struct GlyphMask {
    w: i32,
    h: i32,
    scale: f32,
    cov: Vec<f32>,
}

impl GlyphMask {
    /// Rasterizes one run (`text` or a `Segoe Fluent Icons` codepoint) into an empty `w * h` mask.
    pub(super) fn new(w: i32, h: i32, scale: f32) -> Self {
        let len = usize::try_from(w.saturating_mul(h)).unwrap_or(0);
        Self { w, h, scale: scale.max(1.0), cov: vec![0.0; len] }
    }

    pub(super) fn coverage_at(&self, px: i32, py: i32) -> f32 {
        if px < 0 || py < 0 || px >= self.w || py >= self.h {
            return 0.0;
        }
        self.cov[(py * self.w + px) as usize]
    }

    /// Draws `text` inside `(box_x, box_w, cy, font_px)` in logical coordinates, accumulating coverage.
    pub(super) fn draw_run(
        &mut self,
        text: &str,
        face: FontFace,
        (box_x, box_w, cy, font_px): (f32, f32, f32, f32),
        align: Align,
    ) {
        if text.is_empty() || self.w <= 0 || self.h <= 0 {
            return;
        }
        let _ = self.render_gdi(text, face, (box_x, box_w, cy, font_px), align);
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn render_gdi(
        &mut self,
        text: &str,
        face: FontFace,
        (box_x, box_w, cy, font_px): (f32, f32, f32, f32),
        align: Align,
    ) -> Option<()> {
        // SAFETY: creates an offscreen memory DC compatible with the desktop screen.
        let dc = unsafe { CreateCompatibleDC(None) };
        if dc.is_invalid() {
            return None;
        }
        let _dc_guard = DcGuard(dc);

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).unwrap_or(40),
                biWidth: self.w,
                biHeight: -self.h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits_ptr: *mut c_void = core::ptr::null_mut();
        // SAFETY: allocates a top-down 32-bit BGRA DIB section zero-initialized to pure black.
        let bmp: HBITMAP =
            unsafe { CreateDIBSection(Some(dc), &raw const bmi, DIB_RGB_COLORS, &raw mut bits_ptr, None, 0).ok()? };
        if bits_ptr.is_null() {
            return None;
        }
        let _bmp_obj = ObjGuard(bmp.into());
        // SAFETY: `bmp` stays alive for the scope of `_bmp_sel`.
        let _bmp_sel = SelectGuard { dc, old: unsafe { SelectObject(dc, bmp.into()) } };

        let px_height = -((font_px * self.scale).round().max(8.0) as i32);
        let weight = match face {
            FontFace::Ui => FW_MEDIUM.0 as i32,
            FontFace::FluentIcon => FW_NORMAL.0 as i32,
        };
        let face_name: PCWSTR = match face {
            // `--ice-font`: Windows 11 ships `Segoe UI Variable Text`; GDI falls back seamlessly if absent.
            FontFace::Ui => w!("Segoe UI Variable Text"),
            FontFace::FluentIcon => w!("Segoe Fluent Icons"),
        };
        // SAFETY: static null-terminated wide face name + standard Win32 GDI font descriptor.
        let font: HFONT = unsafe {
            CreateFontW(
                px_height,
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_TT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                FONT_QUALITY(CLEARTYPE_NATURAL_QUALITY as u8),
                (DEFAULT_PITCH.0 | FF_SWISS.0) as u32,
                face_name,
            )
        };
        if font.is_invalid() {
            return None;
        }
        let _font_obj = ObjGuard(font.into());
        // SAFETY: `font` outlives `_font_sel`.
        let _font_sel = SelectGuard { dc, old: unsafe { SelectObject(dc, font.into()) } };

        let wide: Vec<u16> = text.encode_utf16().collect();
        let wide_len = u32::try_from(wide.len()).ok()?;
        let mut wide_nul = wide.clone();
        wide_nul.push(0);
        let mut extent = SIZE::default();
        // SAFETY: reads pixel dimensions of `wide` using the selected font.
        unsafe {
            let _ = SetBkMode(dc, TRANSPARENT);
            let _ = SetBkColor(dc, COLORREF(0));
            let _ = SetTextColor(dc, COLORREF(0x00FF_FFFF));
            let _ = GetTextExtentPoint32W(dc, &wide, &raw mut extent);
        };

        let left_px = (box_x * self.scale).round() as i32;
        let width_px = (box_w * self.scale).round() as i32;
        let x = match align {
            Align::Center => left_px + (width_px - extent.cx) / 2,
            Align::Right => left_px + (width_px - extent.cx),
        };
        let y = (cy * self.scale).round() as i32 - extent.cy / 2;

        // SAFETY: draws white glyphs onto the pure-black DIB; ClearType subpixels hold exact greyscale coverage.
        let _ = unsafe { ExtTextOutW(dc, x, y, ETO_OPTIONS(0), None, PCWSTR(wide_nul.as_ptr()), wide_len, None) };

        let len = usize::try_from(self.w * self.h).unwrap_or(0);
        // SAFETY: `bits_ptr` points to `self.w * self.h` 32-bit pixels owned by `bmp`.
        let pixels = unsafe { std::slice::from_raw_parts(bits_ptr.cast::<u32>(), len) };
        for (dst, &bgra) in self.cov.iter_mut().zip(pixels.iter()) {
            let r = (bgra >> 16) & 0xff;
            let g = (bgra >> 8) & 0xff;
            let b = bgra & 0xff;
            let linear = (r + g + b) as f32 / (3.0 * 255.0);
            // GDI ClearType pre-weights strokes for gamma-blended surfaces; lift slightly so small text over dark pills matches WebView weight.
            let cov = if linear > 0.0 { linear.powf(0.75) } else { 0.0 };
            if cov > *dst {
                *dst = cov.clamp(0.0, 1.0);
            }
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gdi_rasterizes_ui_text_and_fluent_icons_with_antialiased_coverage() {
        for scale in [1.0f32, 1.5f32] {
            let w = (116.0 * scale).round() as i32;
            let h = (28.0 * scale).round() as i32;
            let mut ui = GlyphMask::new(w, h, scale);
            ui.draw_run("MUTE", FontFace::Ui, (28.0, 76.0, 14.0, 11.0), Align::Right);

            let mut solid = 0usize;
            let mut edge = 0usize;
            for py in 0..h {
                for px in 0..w {
                    let c = ui.coverage_at(px, py);
                    if c > 0.75 {
                        solid += 1;
                    } else if c > 0.02 {
                        edge += 1;
                    }
                }
            }
            assert!(solid > 10 && edge > 5, "expected antialiased UI glyphs at {scale}x: solid={solid}, edge={edge}");

            // `Segoe Fluent Icons` mute glyph (\uE74F) must also rasterize into the mask.
            let mut icon = GlyphMask::new(w, h, scale);
            icon.draw_run("\u{E74F}", FontFace::FluentIcon, (8.0, 18.0, 14.0, 12.0), Align::Center);
            let icon_px = (0..h)
                .flat_map(|py| (0..w).map(move |px| (px, py)))
                .filter(|&(px, py)| icon.coverage_at(px, py) > 0.1)
                .count();
            assert!(icon_px > 5, "expected Fluent icon pixels at {scale}x: icon_px={icon_px}");
        }
    }
}
