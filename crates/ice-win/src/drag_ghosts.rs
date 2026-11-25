//! D-20 drag & snap: the ghost slots shown on every monitor while the notch is dragged.
//!
//! One small layered window per slot (the idle pill's size and place), drawn once with per-pixel alpha: a translucent
//! pill outline, the slot nearest the cursor in the accent colour. They are click-through, never activate and never
//! appear in the taskbar. They exist only while a drag runs, on the drag thread, and are destroyed when it ends.

use std::ffi::c_void;
use std::mem::size_of;

use ice_core::placement::DropSlot;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HGDIOBJ, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, RegisterClassExW,
    SW_SHOWNOACTIVATE, ShowWindow, TranslateMessage, ULW_ALPHA, UpdateLayeredWindow, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{PCWSTR, w};

/// Accent for the slot under the cursor. Twin: `--ice-accent` (`#6bc5ff`) in `app/ui/src/themes/base.css`.
const ACCENT: (f32, f32, f32) = (107.0, 197.0, 255.0);
/// The other slots: a light outline, so they read on dark and light desktops.
const NEUTRAL: (f32, f32, f32) = (237.0, 241.0, 247.0);

unsafe extern "system" fn ghost_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: default handling for a window of this thread.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// The ghost windows of one drag. Dropping it destroys them (it must drop on the thread that created it).
pub struct DragGhosts {
    windows: Vec<(HWND, DropSlot)>,
    lit: Option<usize>,
}

impl DragGhosts {
    /// Shows a ghost for every slot. A slot whose window can't be created is skipped (the drag still works).
    #[must_use]
    pub fn show(slots: &[DropSlot]) -> Self {
        let class = w!("IceDragGhost");
        // SAFETY: registering a static class for this module's own window procedure; a repeat registration fails
        // harmlessly.
        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(80),
                lpfnWndProc: Some(ghost_proc),
                hInstance: GetModuleHandleW(None).unwrap_or_default().into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassExW(&raw const wc);
        }
        let ex = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
        let mut windows = Vec::with_capacity(slots.len());
        for slot in slots {
            // SAFETY: creates a hidden popup owned by this thread; it is destroyed in `Drop`.
            let Ok(hwnd) = (unsafe {
                CreateWindowExW(
                    ex,
                    class,
                    PCWSTR::null(),
                    WS_POPUP,
                    slot.x,
                    slot.y,
                    slot.w,
                    slot.h,
                    None,
                    None,
                    GetModuleHandleW(None).ok().map(Into::into),
                    None,
                )
            }) else {
                continue;
            };
            paint(hwnd, slot, false);
            // SAFETY: shows the window without activating it.
            let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
            windows.push((hwnd, *slot));
        }
        Self { windows, lit: None }
    }

    /// Highlights the slot at `index` in the list given to [`Self::show`] (repaints only the two that change).
    pub fn highlight(&mut self, slot: &DropSlot) {
        let index = self.windows.iter().position(|(_, s)| s == slot);
        if index == self.lit {
            return;
        }
        if let Some(old) = self.lit.and_then(|i| self.windows.get(i)) {
            paint(old.0, &old.1, false);
        }
        if let Some(new) = index.and_then(|i| self.windows.get(i)) {
            paint(new.0, &new.1, true);
        }
        self.lit = index;
    }

    /// Handles this thread's pending window messages (the drag loop calls it every tick).
    pub fn pump() {
        let mut msg = MSG::default();
        // SAFETY: standard non-blocking message pump for windows owned by this thread.
        unsafe {
            while PeekMessageW(&raw mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&raw const msg);
                DispatchMessageW(&raw const msg);
            }
        }
    }
}

impl Drop for DragGhosts {
    fn drop(&mut self) {
        for (hwnd, _) in self.windows.drain(..) {
            // SAFETY: each window was created by this thread in `show` and is destroyed once.
            let _ = unsafe { DestroyWindow(hwnd) };
        }
    }
}

/// Draws a pill outline (`lit` = accent) into the layered window, premultiplied BGRA.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel loop over a small DIB"
)]
fn paint(hwnd: HWND, slot: &DropSlot, lit: bool) {
    let (w, h) = (slot.w.max(4), slot.h.max(4));
    // SAFETY: a memory DC for a DIB section; both are released before returning.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return;
        }
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).unwrap_or(40),
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = core::ptr::null_mut();
        let Ok(bmp) = CreateDIBSection(Some(dc), &raw const bmi, DIB_RGB_COLORS, &raw mut bits, None, 0) else {
            let _ = DeleteDC(dc);
            return;
        };
        if bits.is_null() {
            let _ = DeleteObject(bmp.into());
            let _ = DeleteDC(dc);
            return;
        }
        let old: HGDIOBJ = SelectObject(dc, bmp.into());
        let pixels = std::slice::from_raw_parts_mut(bits.cast::<u32>(), (w * h) as usize);
        let (rgb, fill_a, line_a) = if lit { (ACCENT, 0.32, 0.95) } else { (NEUTRAL, 0.10, 0.55) };
        let r = h as f32 / 2.0;
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        let half = (w as f32 / 2.0 - r).max(0.0);
        for py in 0..h {
            for px in 0..w {
                let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
                // Signed distance to the pill (stadium) shape.
                let dx = ((x - cx).abs() - half).max(0.0);
                let d = (dx * dx + (y - cy) * (y - cy)).sqrt() - (r - 0.5);
                let inside = (0.5 - d).clamp(0.0, 1.0);
                let line = (1.0 - (d + 1.0).abs()).clamp(0.0, 1.0);
                let a = (fill_a * inside).max(line_a * line);
                let pa = (a * 255.0).round() as u32;
                let ch = |c: f32| (c * a).round() as u32;
                pixels[(py * w + px) as usize] = (pa << 24) | (ch(rgb.0) << 16) | (ch(rgb.1) << 8) | ch(rgb.2);
            }
        }
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let (pt_dst, size, pt_src) = (POINT { x: slot.x, y: slot.y }, SIZE { cx: w, cy: h }, POINT { x: 0, y: 0 });
        let _ = UpdateLayeredWindow(
            hwnd,
            None,
            Some(&raw const pt_dst),
            Some(&raw const size),
            Some(dc),
            Some(&raw const pt_src),
            COLORREF(0),
            Some(&raw const blend),
            ULW_ALPHA,
        );
        SelectObject(dc, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(dc);
    }
}
