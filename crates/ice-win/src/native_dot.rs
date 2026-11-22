//! Lightweight native Win32 layered window for the idle dot / cold-wake indicator (D-15).
//!
//! Runs on its own dedicated message-loop thread and draws via `UpdateLayeredWindow` with a
//! premultiplied 32-bit BGRA DIB section (~1–2 MB RAM, zero WebView2).

use core::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_TOPMOST, MSG, PostMessageW,
    PostQuitMessage, RegisterClassExW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
    SWP_NOSENDCHANGING, SetWindowPos, ShowWindow, TranslateMessage, ULW_ALPHA, UpdateLayeredWindow, WM_APP, WM_CLOSE,
    WM_DESTROY, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_POPUP,
};
use windows::core::{Result, w};

use crate::monitors::MonitorInfo;

const WM_DOT_CMD: u32 = WM_APP + 1;
const LOGICAL_DOT_W: i32 = 88;
const LOGICAL_DOT_H: i32 = 16;
const LOGICAL_TOP_OFFSET: i32 = 6;

#[derive(Debug, Clone)]
enum DotCmd {
    ShowIdle,
    Pulse { progress: Option<f32>, accent_rgb: u32 },
    Hide,
    MoveMonitor(MonitorInfo),
}

struct SharedState {
    visible: AtomicBool,
    pending_cmd: std::sync::Mutex<Option<DotCmd>>,
}

/// Safe handle to the native Win32 layered idle dot window.
pub struct NativeDotHandle {
    hwnd_raw: isize,
    shared: Arc<SharedState>,
    thread: Option<JoinHandle<()>>,
}

impl NativeDotHandle {
    /// Spawns the native idle dot window on a dedicated Win32 message thread.
    ///
    /// # Errors
    /// Fails if window class registration or window creation fails on the UI thread.
    pub fn spawn(monitor: MonitorInfo) -> Result<Self> {
        let shared =
            Arc::new(SharedState { visible: AtomicBool::new(false), pending_cmd: std::sync::Mutex::new(None) });
        let shared_thread = Arc::clone(&shared);
        let (tx, rx) = mpsc::sync_channel::<Result<isize>>(1);

        let handle = thread::Builder::new()
            .name("ice-native-dot".into())
            .spawn(move || dot_thread_main(monitor, shared_thread, tx))
            .map_err(|_| windows::core::Error::from_thread())?;

        let hwnd_raw = rx.recv().unwrap_or_else(|_| Err(windows::core::Error::from_thread()))?;
        Ok(Self { hwnd_raw, shared, thread: Some(handle) })
    }

    /// Shows the subtle idle dot/mini-pill at top-center.
    pub fn show_idle(&self) {
        self.send_cmd(DotCmd::ShowIdle);
    }

    /// Updates the native dot into an immediate activity pill (with optional volume progress `0.0..=1.0`).
    pub fn pulse_activity(&self, progress: Option<f32>, accent_rgb: u32) {
        self.send_cmd(DotCmd::Pulse { progress, accent_rgb });
    }

    /// Hides the native dot (e.g. once the WebView notch has painted its first frame).
    pub fn hide(&self) {
        self.send_cmd(DotCmd::Hide);
    }

    /// Repositions the native dot onto another monitor.
    pub fn move_to_monitor(&self, monitor: MonitorInfo) {
        self.send_cmd(DotCmd::MoveMonitor(monitor));
    }

    /// Returns whether the native dot is currently visible.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.shared.visible.load(Ordering::Acquire)
    }

    fn send_cmd(&self, cmd: DotCmd) {
        if let Ok(mut slot) = self.shared.pending_cmd.lock() {
            *slot = Some(cmd);
        }
        let hwnd = HWND(self.hwnd_raw as *mut c_void);
        // SAFETY: `hwnd` lives until `Drop` sends `WM_CLOSE` and joins the thread.
        let _ = unsafe { PostMessageW(Some(hwnd), WM_DOT_CMD, WPARAM(0), LPARAM(0)) };
    }
}

impl Drop for NativeDotHandle {
    fn drop(&mut self) {
        if self.hwnd_raw != 0 {
            let hwnd = HWND(self.hwnd_raw as *mut c_void);
            // SAFETY: asks the dot window's message thread to destroy and quit.
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

fn dot_thread_main(monitor: MonitorInfo, shared: Arc<SharedState>, ready_tx: mpsc::SyncSender<Result<isize>>) {
    let class_name = w!("IceAudioNativeDotClass");
    // SAFETY: `GetModuleHandleW(None)` returns the current executable's HINSTANCE.
    let hinstance = unsafe { GetModuleHandleW(None) }.unwrap_or_default();
    let wc = WNDCLASSEXW {
        cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(0),
        lpfnWndProc: Some(dot_wnd_proc),
        hInstance: hinstance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    // SAFETY: registering an immutable static window class name; duplicate registration is harmless.
    unsafe { RegisterClassExW(&raw const wc) };

    let ex_style = WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_TRANSPARENT;
    let phys_w = monitor.logical_to_physical(LOGICAL_DOT_W);
    let phys_h = monitor.logical_to_physical(LOGICAL_DOT_H);
    let x = monitor.left + (monitor.width - phys_w) / 2;
    let y = monitor.top + monitor.logical_to_physical(LOGICAL_TOP_OFFSET);

    // SAFETY: creates the hidden layered window owned by this thread.
    let hwnd = match unsafe {
        CreateWindowExW(
            ex_style,
            class_name,
            w!("IceNativeDot"),
            WS_POPUP,
            x,
            y,
            phys_w,
            phys_h,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
    } {
        Ok(h) => h,
        Err(err) => {
            let _ = ready_tx.send(Err(err));
            return;
        }
    };

    let mut active_monitor = monitor;
    let mut last_progress: Option<f32> = None;
    let mut last_accent: u32 = 0x006b_c5ff;
    let _ = paint_dot(hwnd, &active_monitor, false, last_progress, last_accent);
    let _ = ready_tx.send(Ok(hwnd.0 as isize));

    let mut msg = MSG::default();
    // SAFETY: standard Win32 message loop for the window owned by this thread.
    while unsafe { GetMessageW(&raw mut msg, None, 0, 0) }.as_bool() {
        if msg.message == WM_DOT_CMD {
            let cmd = shared.pending_cmd.lock().ok().and_then(|mut s| s.take());
            if let Some(cmd) = cmd {
                match cmd {
                    DotCmd::ShowIdle => {
                        last_progress = None;
                        let _ = paint_dot(hwnd, &active_monitor, false, None, last_accent);
                        // SAFETY: reveals layered window without activating it.
                        unsafe {
                            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, WindowsFlags::SWP_KEEP_SIZE);
                        }
                        shared.visible.store(true, Ordering::Release);
                    }
                    DotCmd::Pulse { progress, accent_rgb } => {
                        last_progress = progress;
                        last_accent = accent_rgb;
                        let _ = paint_dot(hwnd, &active_monitor, true, progress, accent_rgb);
                        // SAFETY: reveals layered window without activating it.
                        unsafe {
                            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, WindowsFlags::SWP_KEEP_SIZE);
                        }
                        shared.visible.store(true, Ordering::Release);
                    }
                    DotCmd::Hide => {
                        // SAFETY: hides layered window without changing activation.
                        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
                        shared.visible.store(false, Ordering::Release);
                    }
                    DotCmd::MoveMonitor(m) => {
                        active_monitor = m;
                        let is_active = last_progress.is_some();
                        let _ = paint_dot(hwnd, &active_monitor, is_active, last_progress, last_accent);
                    }
                }
            }
            continue;
        }
        // SAFETY: standard Win32 message dispatch.
        unsafe {
            let _ = TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);
        }
    }
}

struct WindowsFlags;
impl WindowsFlags {
    const SWP_KEEP_SIZE: windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS =
        windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS(
            SWP_NOACTIVATE.0
                | SWP_NOOWNERZORDER.0
                | SWP_NOSENDCHANGING.0
                | windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE.0
                | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE.0,
        );
}

unsafe extern "system" fn dot_wnd_proc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match umsg {
        WM_CLOSE => {
            // SAFETY: destroys window on its owning thread.
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: exits this thread's message loop cleanly.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => {
            // SAFETY: default window handling.
            unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) }
        }
    }
}

struct MemoryDcGuard {
    dc: HDC,
}
impl Drop for MemoryDcGuard {
    fn drop(&mut self) {
        if !self.dc.is_invalid() {
            // SAFETY: releases the compatible DC allocated in `paint_dot`.
            let _ = unsafe { DeleteDC(self.dc) };
        }
    }
}

struct BitmapGuard {
    dc: HDC,
    old: HGDIOBJ,
    bmp: HBITMAP,
}
impl Drop for BitmapGuard {
    fn drop(&mut self) {
        // SAFETY: restores previous GDI object before deleting the DIB section.
        unsafe {
            if !self.old.is_invalid() {
                let _ = SelectObject(self.dc, self.old);
            }
            if !self.bmp.is_invalid() {
                let _ = DeleteObject(self.bmp.into());
            }
        }
    }
}

fn paint_dot(hwnd: HWND, monitor: &MonitorInfo, active: bool, progress: Option<f32>, accent_rgb: u32) -> Result<()> {
    let logical_w = if active { 148 } else { LOGICAL_DOT_W };
    let logical_h = if active { 26 } else { LOGICAL_DOT_H };
    let w = monitor.logical_to_physical(logical_w).max(16);
    let h = monitor.logical_to_physical(logical_h).max(8);
    let x = monitor.left + (monitor.width - w) / 2;
    let y = monitor.top + monitor.logical_to_physical(LOGICAL_TOP_OFFSET);

    // SAFETY: creates a memory DC compatible with the screen.
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.is_invalid() {
        return Err(windows::core::Error::from_thread());
    }
    let _dc_guard = MemoryDcGuard { dc };

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).unwrap_or(40),
            biWidth: w,
            biHeight: -h, // top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits_ptr: *mut c_void = core::ptr::null_mut();
    // SAFETY: `bmi` describes a top-down 32-bit BGRA DIB section; `bits_ptr` receives the pixel buffer.
    let bmp = unsafe { CreateDIBSection(Some(dc), &raw const bmi, DIB_RGB_COLORS, &raw mut bits_ptr, None, 0)? };
    if bits_ptr.is_null() {
        return Err(windows::core::Error::from_thread());
    }
    // SAFETY: `bmp` is newly created and owned until `BitmapGuard` drops.
    let old = unsafe { SelectObject(dc, bmp.into()) };
    let _bmp_guard = BitmapGuard { dc, old, bmp };

    let pixel_count = usize::try_from(w * h).unwrap_or(0);
    // SAFETY: `CreateDIBSection` allocates `w * h * 4` bytes of contiguous BGRA pixels.
    let pixels = unsafe { std::slice::from_raw_parts_mut(bits_ptr.cast::<u32>(), pixel_count) };
    rasterize_pill(pixels, w, h, active, progress, accent_rgb);

    let pt_dst = POINT { x, y };
    let size = SIZE { cx: w, cy: h };
    let pt_src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: u8::try_from(AC_SRC_OVER).unwrap_or(0),
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: u8::try_from(AC_SRC_ALPHA).unwrap_or(1),
    };
    // SAFETY: commits the premultiplied ARGB DIB directly to the DWM layered window.
    unsafe {
        UpdateLayeredWindow(
            hwnd,
            None,
            Some(&raw const pt_dst),
            Some(&raw const size),
            Some(dc),
            Some(&raw const pt_src),
            COLORREF(0),
            Some(&raw const blend),
            ULW_ALPHA,
        )
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn rasterize_pill(pixels: &mut [u32], w: i32, h: i32, active: bool, progress: Option<f32>, accent_rgb: u32) {
    let wf = w as f32;
    let hf = h as f32;
    let r = (hf - 2.0) * 0.5;
    let cx = wf * 0.5;
    let cy = hf * 0.5;
    let half_w = (wf - 2.0) * 0.5 - r;
    let half_h = 0.0f32;

    let ar = ((accent_rgb >> 16) & 0xff) as f32;
    let ag = ((accent_rgb >> 8) & 0xff) as f32;
    let ab = (accent_rgb & 0xff) as f32;
    let bar_pad = (wf * 0.14).max(10.0);
    let bar_w = (wf - bar_pad * 2.0).max(1.0);
    let fill_frac = progress.map(|p| p.clamp(0.0, 1.0));

    for py in 0..h {
        let yf = py as f32 + 0.5;
        for px in 0..w {
            let xf = px as f32 + 0.5;
            // Signed distance to capsule (rounded pill).
            let dx = (xf - cx).abs() - half_w;
            let dy = (yf - cy).abs() - half_h;
            let qx = dx.max(0.0);
            let qy = dy.max(0.0);
            let sdf = (qx * qx + qy * qy).sqrt() + dx.max(dy).min(0.0) - r;

            let coverage = (0.5 - sdf).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                pixels[(py * w + px) as usize] = 0;
                continue;
            }

            // Base Ice obsidian fill (#0e1116 @ ~90% alpha).
            let mut cr = 14.0f32;
            let mut cg = 17.0f32;
            let mut cb = 22.0f32;
            let mut alpha = (if active { 0.92 } else { 0.86 }) * coverage;

            // 1 px specular Ice rim near the outer edge.
            if sdf > -1.35 {
                let rim_t = ((sdf + 1.35) / 1.35).clamp(0.0, 1.0) * (if active { 0.55 } else { 0.34 });
                cr = cr * (1.0 - rim_t) + ar * rim_t;
                cg = cg * (1.0 - rim_t) + ag * rim_t;
                cb = cb * (1.0 - rim_t) + ab * rim_t;
                alpha = alpha.max(rim_t * coverage);
            } else if let Some(frac) = fill_frac {
                // Thin progress bar inside the native wake pill while WebView warms up.
                if (yf - cy).abs() <= 2.0 && xf >= bar_pad && xf <= bar_pad + bar_w {
                    let rel = (xf - bar_pad) / bar_w;
                    if rel <= frac {
                        cr = ar;
                        cg = ag;
                        cb = ab;
                    } else {
                        cr = 44.0;
                        cg = 52.0;
                        cb = 66.0;
                    }
                }
            } else if !active && (xf - cx).abs() < 4.5 && (yf - cy).abs() < 4.5 {
                // Subtle center Ice dot inside the idle pill.
                let dist = ((xf - cx).powi(2) + (yf - cy).powi(2)).sqrt();
                if dist < 2.2 {
                    let dot_t = ((2.2 - dist) / 1.2).clamp(0.0, 1.0) * 0.72;
                    cr = cr * (1.0 - dot_t) + ar * dot_t;
                    cg = cg * (1.0 - dot_t) + ag * dot_t;
                    cb = cb * (1.0 - dot_t) + ab * dot_t;
                }
            }

            let pa = (alpha * 255.0).round().clamp(0.0, 255.0) as u32;
            let pr = (cr * alpha).round().clamp(0.0, 255.0) as u32;
            let pg = (cg * alpha).round().clamp(0.0, 255.0) as u32;
            let pb = (cb * alpha).round().clamp(0.0, 255.0) as u32;
            pixels[(py * w + px) as usize] = (pa << 24) | (pr << 16) | (pg << 8) | pb;
        }
    }
}
