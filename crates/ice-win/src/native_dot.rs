//! Lightweight native Win32 layered window for the idle dot, wake pill, and `Lite` overlay (D-15 / D-15.1).
//!
//! Runs on a dedicated message-loop thread and draws anti-aliased RGBA graphics via
//! `UpdateLayeredWindow` using a premultiplied 32-bit BGRA DIB section (~2.2 MB RAM, zero WebView2).
//! Subscribes cleanly to [`ice_core::engine::Engine`] events using the snapshot + monotonic
//! revision protocol (D-03) and matches `--ice-*` theme tokens from `docs/theming.md`.

use core::ffi::c_void;
use std::collections::VecDeque;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use ice_core::engine::{Engine, EngineEvent, EngineSnapshot, Envelope, VolumeState};
use ice_core::media::PlaybackStatus;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_TOPMOST, IDC_ARROW, KillTimer,
    LoadCursorW, MSG, PostMessageW, PostQuitMessage, RegisterClassExW, SET_WINDOW_POS_FLAGS, SW_HIDE,
    SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSENDCHANGING, SWP_NOSIZE, SetTimer,
    SetWindowPos, ShowWindow, TranslateMessage, ULW_ALPHA, UpdateLayeredWindow, WM_APP, WM_CLOSE, WM_DESTROY, WM_TIMER,
    WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{Result, w};

use crate::monitors::MonitorInfo;

const WM_DOT_CMD: u32 = WM_APP + 1;
const ACTIVITY_TIMER_ID: usize = 0x4943_4531;
const LOGICAL_TOP_OFFSET: i32 = 6;

// Twin: `app/ui/src/lib/notch/geometry.ts` (size & radius tokens for the idle pill).
const IDLE_W: i32 = 116;
const IDLE_H: i32 = 28;
const IDLE_DOT_W: i32 = 36;
const IDLE_DOT_H: i32 = 18;
const VOLUME_W: i32 = 192;
const VOLUME_H: i32 = 30;
const MIC_W: i32 = 156;
const MIC_H: i32 = 30;
const DEVICE_W: i32 = 184;
const DEVICE_H: i32 = 30;

use ice_core::appearance::ThemeKind;
pub use ice_core::appearance::{IdleStyle, NotchStyle};

mod glyphs;
mod input;

use glyphs::{Align, FontFace, GlyphMask};
pub use input::{InputHandler, PillInput, PillInteraction};

/// How much brighter the rim gets under the cursor, so the pill reads as clickable (D-19 A1).
const HOVER_RIM_BOOST: f32 = 0.30;
/// Rim strength, background tint and floor alpha of the pressed state.
const PRESSED_RIM_STRENGTH: f32 = 0.95;
const PRESSED_TINT_ALPHA: f32 = 0.14;
const PRESSED_MIN_ALPHA: f32 = 0.94;

const SWP_KEEP_SIZE: SET_WINDOW_POS_FLAGS =
    SET_WINDOW_POS_FLAGS(SWP_NOACTIVATE.0 | SWP_NOOWNERZORDER.0 | SWP_NOSENDCHANGING.0 | SWP_NOMOVE.0 | SWP_NOSIZE.0);

/// Straight RGBA colour used by native pill theming and SDF rasterization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Constructs an [`Rgba`] colour from `0x00RRGGBB` and an alpha byte.
    #[must_use]
    pub const fn from_rgb(rgb: u32, a: u8) -> Self {
        Self { r: ((rgb >> 16) & 0xff) as u8, g: ((rgb >> 8) & 0xff) as u8, b: (rgb & 0xff) as u8, a }
    }

    #[must_use]
    pub const fn to_rgb_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    const fn as_f32_rgba(self) -> (f32, f32, f32, f32) {
        (self.r as f32, self.g as f32, self.b as f32, self.a as f32 / 255.0)
    }
}

/// Bundled themes defined in `docs/theming.md` and `base.css`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuiltinTheme {
    #[default]
    Ice,
    IceLight,
    Midnight,
    Fluent,
    Minimal,
}

/// Subset of `--ice-*` theme tokens synced to the native wake / `Lite` pill (`docs/theming.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePillTheme {
    pub bg: Rgba,
    pub surface: Rgba,
    pub rim: Rgba,
    pub fg: Rgba,
    pub fg_muted: Rgba,
    pub accent: Rgba,
    pub ok: Rgba,
    pub danger: Rgba,
}

impl Default for NativePillTheme {
    fn default() -> Self {
        Self::ice()
    }
}

impl NativePillTheme {
    /// The palette of a bundled theme (the saved `appearance.theme`).
    #[must_use]
    pub const fn builtin(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Ice => Self::ice(),
            ThemeKind::IceLight => Self::ice_light(),
            ThemeKind::Midnight => Self::midnight(),
            ThemeKind::Fluent => Self::fluent(),
            ThemeKind::Minimal => Self::minimal(),
        }
    }

    /// Portfolio-seeded default `Ice` theme.
    #[must_use]
    pub const fn ice() -> Self {
        Self {
            bg: Rgba::new(14, 17, 22, 230),
            surface: Rgba::new(23, 29, 39, 235),
            rim: Rgba::new(154, 214, 255, 115),
            fg: Rgba::new(237, 241, 247, 255),
            fg_muted: Rgba::new(152, 164, 184, 255),
            accent: Rgba::new(107, 197, 255, 255),
            ok: Rgba::new(74, 222, 128, 255),
            danger: Rgba::new(240, 113, 72, 255),
        }
    }

    /// Portfolio-seeded frosted snow `Ice Light` theme.
    #[must_use]
    pub const fn ice_light() -> Self {
        Self {
            bg: Rgba::new(246, 247, 249, 240),
            surface: Rgba::new(255, 255, 255, 245),
            rim: Rgba::new(2, 106, 162, 87),
            fg: Rgba::new(17, 22, 31, 255),
            fg_muted: Rgba::new(75, 86, 104, 255),
            accent: Rgba::new(2, 106, 162, 255),
            ok: Rgba::new(21, 128, 61, 255),
            danger: Rgba::new(201, 70, 36, 255),
        }
    }

    /// Near-pure-black `Midnight` theme.
    #[must_use]
    pub const fn midnight() -> Self {
        Self {
            bg: Rgba::new(0, 0, 0, 245),
            surface: Rgba::new(12, 12, 14, 245),
            rim: Rgba::new(255, 255, 255, 61),
            ..Self::ice()
        }
    }

    /// Neutral Windows 11 dark-grey `Fluent` theme (contrast-tuned for WCAG AA over light desktops).
    #[must_use]
    pub const fn fluent() -> Self {
        Self {
            bg: Rgba::new(24, 24, 27, 237),
            surface: Rgba::new(36, 36, 40, 240),
            rim: Rgba::new(107, 197, 255, 89),
            ..Self::ice()
        }
    }

    /// Opaque `Minimal` theme with zero specular rim or glow.
    #[must_use]
    pub const fn minimal() -> Self {
        Self {
            bg: Rgba::new(18, 22, 29, 255),
            surface: Rgba::new(26, 32, 42, 255),
            rim: Rgba::new(0, 0, 0, 0),
            ..Self::ice()
        }
    }

    /// Resolves one of the bundled themes.
    #[must_use]
    pub const fn from_builtin(theme: BuiltinTheme) -> Self {
        match theme {
            BuiltinTheme::Ice => Self::ice(),
            BuiltinTheme::IceLight => Self::ice_light(),
            BuiltinTheme::Midnight => Self::midnight(),
            BuiltinTheme::Fluent => Self::fluent(),
            BuiltinTheme::Minimal => Self::minimal(),
        }
    }

    /// Applies user CSS `--ice-*` overrides onto `base`.
    ///
    /// Supports `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(...)`, `rgba(...)`, and `transparent`.
    /// Unknown variables and unparseable values are ignored without failing.
    #[must_use]
    pub fn from_css_overrides(base: Self, css: &str) -> Self {
        let mut out = base;
        let stripped = strip_css_comments(css);
        let normalized = stripped.replace(['{', '}'], ";");
        for decl in normalized.split(';') {
            let Some((raw_name, raw_val)) = decl.split_once(':') else {
                continue;
            };
            let name = raw_name.trim();
            if !name.starts_with("--ice-") {
                continue;
            }
            let val = raw_val.split("!important").next().unwrap_or(raw_val).trim();
            let Some(color) = parse_css_color(val) else {
                continue;
            };
            match name {
                "--ice-bg" => out.bg = color,
                "--ice-surface" => out.surface = color,
                "--ice-rim" => out.rim = color,
                "--ice-fg" => out.fg = color,
                "--ice-fg-muted" => out.fg_muted = color,
                "--ice-accent" => out.accent = color,
                "--ice-ok" => out.ok = color,
                "--ice-danger" => out.danger = color,
                _ => {}
            }
        }
        out
    }
}

fn strip_css_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("*/") else {
            return out;
        };
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

fn parse_css_color(input: &str) -> Option<Rgba> {
    let s = input.trim();
    if s.eq_ignore_ascii_case("transparent") {
        return Some(Rgba::new(0, 0, 0, 0));
    }
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    let lower = s.to_ascii_lowercase();
    if let Some(inner) = lower.strip_prefix("rgba(").and_then(|v| v.strip_suffix(')')) {
        return parse_rgb_args(inner);
    }
    if let Some(inner) = lower.strip_prefix("rgb(").and_then(|v| v.strip_suffix(')')) {
        return parse_rgb_args(inner);
    }
    None
}

fn parse_hex_color(hex: &str) -> Option<Rgba> {
    let bytes = hex.as_bytes();
    match bytes.len() {
        3 | 4 => {
            let r = parse_hex_nibble(bytes[0])?;
            let g = parse_hex_nibble(bytes[1])?;
            let b = parse_hex_nibble(bytes[2])?;
            let a = if bytes.len() == 4 { parse_hex_nibble(bytes[3])? * 17 } else { 255 };
            Some(Rgba::new(r * 17, g * 17, b * 17, a))
        }
        6 | 8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = if bytes.len() == 8 { u8::from_str_radix(&hex[6..8], 16).ok()? } else { 255 };
            Some(Rgba::new(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn parse_rgb_args(inner: &str) -> Option<Rgba> {
    let (rgb_part, alpha_opt) = if inner.contains(',') {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            return Some(Rgba::new(
                parse_channel_u8(parts[0])?,
                parse_channel_u8(parts[1])?,
                parse_channel_u8(parts[2])?,
                255,
            ));
        }
        if parts.len() == 4 {
            return Some(Rgba::new(
                parse_channel_u8(parts[0])?,
                parse_channel_u8(parts[1])?,
                parse_channel_u8(parts[2])?,
                parse_alpha_u8(parts[3])?,
            ));
        }
        return None;
    } else if let Some((left, right)) = inner.split_once('/') {
        (left, Some(right.trim()))
    } else {
        (inner, None)
    };

    let chans: Vec<&str> = rgb_part.split_whitespace().collect();
    if chans.len() != 3 {
        return None;
    }
    let a = match alpha_opt {
        Some(a_str) => parse_alpha_u8(a_str)?,
        None => 255,
    };
    Some(Rgba::new(parse_channel_u8(chans[0])?, parse_channel_u8(chans[1])?, parse_channel_u8(chans[2])?, a))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn parse_channel_u8(raw: &str) -> Option<u8> {
    if let Some(pct) = raw.strip_suffix('%') {
        let v: f32 = pct.trim().parse().ok()?;
        return Some((v.clamp(0.0, 100.0) * 2.55).round() as u8);
    }
    let v: f32 = raw.parse().ok()?;
    Some(v.clamp(0.0, 255.0).round() as u8)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn parse_alpha_u8(raw: &str) -> Option<u8> {
    if let Some(pct) = raw.strip_suffix('%') {
        let v: f32 = pct.trim().parse().ok()?;
        return Some((v.clamp(0.0, 100.0) * 2.55).round() as u8);
    }
    let v: f32 = raw.parse().ok()?;
    Some((v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Visual mode currently rendered by the native wake / `Lite` pill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PillMode {
    #[default]
    Idle,
    Volume {
        percent: u8,
        muted: bool,
    },
    MicVolume {
        percent: u8,
        muted: bool,
    },
    MicMute {
        muted: bool,
    },
    DeviceEvent {
        is_headphones: bool,
        spatial_active: bool,
        is_removal: bool,
    },
}

/// Result of applying an [`Envelope`] against [`PillState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeOutcome {
    Updated,
    IgnoredStale,
    NeedsResnapshot,
}

/// Authoritative native pill state synchronized via `ice_core::engine` snapshot + delta revisions.
#[derive(Debug, Clone, PartialEq)]
pub struct PillState {
    pub revision: u64,
    pub volume: Option<VolumeState>,
    pub mic_volume: Option<u8>,
    pub mic_muted: bool,
    /// Status of the current media session, `None` when nothing is open. Idle content reads it.
    pub media_status: Option<PlaybackStatus>,
    pub active_device: Option<String>,
    pub spatial_active: bool,
    pub mode: PillMode,
    pub idle_style: IdleStyle,
    pub notch_style: NotchStyle,
    pub theme: NativePillTheme,
    /// Pointer state. Owned by the window thread; `show_state` keeps the live value.
    pub interaction: PillInteraction,
}

impl Default for PillState {
    fn default() -> Self {
        Self::new(NativePillTheme::default())
    }
}

impl PillState {
    /// Creates an idle pill state with `theme`.
    #[must_use]
    pub fn new(theme: NativePillTheme) -> Self {
        Self {
            revision: 0,
            volume: None,
            mic_volume: None,
            mic_muted: false,
            media_status: None,
            active_device: None,
            spatial_active: false,
            mode: PillMode::Idle,
            idle_style: IdleStyle::default(),
            notch_style: NotchStyle::default(),
            theme,
            interaction: PillInteraction::None,
        }
    }

    /// Initializes state from an engine snapshot while staying in [`PillMode::Idle`].
    #[must_use]
    pub fn from_snapshot(snapshot: &EngineSnapshot, theme: NativePillTheme) -> Self {
        let mut st = Self::new(theme);
        st.apply_snapshot(snapshot);
        st
    }

    /// Applies an [`EngineSnapshot`] without replaying or inventing a transient activity.
    pub fn apply_snapshot(&mut self, snapshot: &EngineSnapshot) {
        self.revision = snapshot.revision;
        self.notch_style = snapshot.settings.appearance.notch_style;
        self.volume = snapshot.volume.map(|v| VolumeState { percent: v.percent.min(100), muted: v.muted });
        self.mic_volume = snapshot.mic_volume.map(|p| p.min(100));
        self.mic_muted = snapshot.mic_muted.unwrap_or(false);
        self.media_status = snapshot.media.as_ref().map(|m| m.status);
        let default_playback = snapshot.playback.iter().find(|d| d.is_default).or_else(|| snapshot.playback.first());
        self.active_device = default_playback.map(|d| d.name.clone());
        self.spatial_active =
            default_playback.and_then(|d| d.spatial).is_some_and(|s| s != ice_core::engine::SpatialMode::Off);
    }

    /// Applies a single delta [`Envelope`], enforcing monotonic revisions and returning whether
    /// callers must re-fetch the snapshot due to a revision gap.
    pub fn apply_envelope(&mut self, envelope: &Envelope) -> EnvelopeOutcome {
        if envelope.revision <= self.revision {
            return EnvelopeOutcome::IgnoredStale;
        }
        if self.revision > 0 && envelope.revision > self.revision + 1 {
            return EnvelopeOutcome::NeedsResnapshot;
        }
        self.revision = envelope.revision;
        match &envelope.event {
            EngineEvent::VolumeChanged { volume } => {
                let normalized = VolumeState { percent: volume.percent.min(100), muted: volume.muted };
                self.volume = Some(normalized);
                self.mode = PillMode::Volume { percent: normalized.percent, muted: normalized.muted };
            }
            EngineEvent::MicVolumeChanged { percent } => {
                let pct = (*percent).min(100);
                self.mic_volume = Some(pct);
                self.mode = PillMode::MicVolume { percent: pct, muted: self.mic_muted };
            }
            EngineEvent::MicMuteChanged { muted } => {
                self.mic_muted = *muted;
                self.mode = PillMode::MicMute { muted: *muted };
            }
            EngineEvent::Switched { device, spatial_applied, .. } => {
                self.active_device = Some(device.name.clone());
                self.spatial_active = spatial_applied.is_some();
                self.mode = PillMode::DeviceEvent {
                    is_headphones: device.form_factor == Some(3),
                    spatial_active: spatial_applied.is_some(),
                    is_removal: false,
                };
            }
            EngineEvent::DeviceArrived { device } => {
                self.mode = PillMode::DeviceEvent {
                    is_headphones: device.form_factor == Some(3),
                    spatial_active: false,
                    is_removal: false,
                };
            }
            EngineEvent::DeviceLeft { device } => {
                self.mode = PillMode::DeviceEvent {
                    is_headphones: device.form_factor == Some(3),
                    spatial_active: false,
                    is_removal: true,
                };
            }
            EngineEvent::DevicesChanged { playback, .. } => {
                let default_playback = playback.iter().find(|d| d.is_default).or_else(|| playback.first());
                self.active_device = default_playback.map(|d| d.name.clone());
                self.spatial_active =
                    default_playback.and_then(|d| d.spatial).is_some_and(|s| s != ice_core::engine::SpatialMode::Off);
            }
            // Media and settings sync silently without waking the pill.
            EngineEvent::MediaChanged { media } => self.media_status = media.as_ref().map(|m| m.status),
            EngineEvent::SettingsChanged { settings } => self.notch_style = settings.appearance.notch_style,
            EngineEvent::Notice { .. }
            | EngineEvent::NoticesChanged { .. }
            | EngineEvent::LyricsChanged { .. }
            | EngineEvent::ShowNotch => {}
        }
        EnvelopeOutcome::Updated
    }
}

#[derive(Debug, Clone, Copy)]
enum ThreadSignal {
    Sync { restart_timer: bool },
    Hide,
    Idle,
    MoveMonitor,
}

struct RuntimeState {
    pill: PillState,
    base_theme: NativePillTheme,
    monitor: MonitorInfo,
    desired_visible: bool,
    standby_only: bool,
    activity_timeout_ms: Option<u32>,
    pulse_accent_override: Option<Rgba>,
    signals: VecDeque<ThreadSignal>,
    input: input::InputState,
}

struct SharedState {
    hwnd_raw: AtomicIsize,
    visible: AtomicBool,
    closing: AtomicBool,
    runtime: Mutex<RuntimeState>,
    input_handler: Mutex<Option<InputHandler>>,
}

impl SharedState {
    fn lock(&self) -> std::sync::MutexGuard<'_, RuntimeState> {
        self.runtime.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn post_signal(&self, signal: ThreadSignal) {
        {
            let mut rt = self.lock();
            rt.signals.push_back(signal);
        }
        self.wake_thread();
    }

    fn wake_thread(&self) {
        if self.closing.load(Ordering::Acquire) {
            return;
        }
        let raw = self.hwnd_raw.load(Ordering::Acquire);
        if raw != 0 {
            let hwnd = HWND(raw as *mut c_void);
            // SAFETY: `hwnd` belongs to the dedicated dot thread and `closing` is checked before posting.
            let _ = unsafe { PostMessageW(Some(hwnd), WM_DOT_CMD, WPARAM(0), LPARAM(0)) };
        }
    }
}

/// Active engine event subscription feeding [`NativeDotHandle`].
pub struct NativePillSubscription {
    cancelled: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl NativePillSubscription {
    /// Signals the subscriber loop to exit once unblocked.
    pub fn stop(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Waits for the subscriber thread to finish (call after releasing or dropping the engine).
    pub fn join(mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for NativePillSubscription {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

/// Safe handle to the native Win32 layered idle dot / `Lite` activity pill window.
pub struct NativeDotHandle {
    shared: Arc<SharedState>,
    thread: Option<JoinHandle<()>>,
}

impl NativeDotHandle {
    /// Spawns the native idle dot window on a dedicated Win32 message thread.
    ///
    /// # Errors
    /// Fails if window class registration or window creation fails on the UI thread.
    pub fn spawn(monitor: MonitorInfo) -> Result<Self> {
        let theme = NativePillTheme::default();
        let shared = Arc::new(SharedState {
            hwnd_raw: AtomicIsize::new(0),
            visible: AtomicBool::new(false),
            closing: AtomicBool::new(false),
            runtime: Mutex::new(RuntimeState {
                pill: PillState::new(theme),
                base_theme: theme,
                monitor: monitor.clone(),
                desired_visible: false,
                standby_only: false,
                activity_timeout_ms: None,
                pulse_accent_override: None,
                signals: VecDeque::new(),
                input: input::InputState::default(),
            }),
            input_handler: Mutex::new(None),
        });
        let shared_thread = Arc::clone(&shared);
        let (tx, rx) = mpsc::sync_channel::<Result<isize>>(1);

        let handle = thread::Builder::new()
            .name("ice-native-dot".into())
            .spawn(move || dot_thread_main(monitor, shared_thread, tx))
            .map_err(|_| windows::core::Error::from_thread())?;

        let hwnd_raw = rx.recv().unwrap_or_else(|_| Err(windows::core::Error::from_thread()))?;
        shared.hwnd_raw.store(hwnd_raw, Ordering::Release);
        Ok(Self { shared, thread: Some(handle) })
    }

    /// Shows the subtle idle dot/mini-pill at top-center (or hides when [`IdleStyle::Hidden`]).
    pub fn show_idle(&self) {
        let should_hide = {
            let mut rt = self.shared.lock();
            rt.pulse_accent_override = None;
            rt.pill.theme = rt.base_theme;
            rt.pill.mode = PillMode::Idle;
            let hide = rt.pill.idle_style == IdleStyle::Hidden;
            rt.desired_visible = !hide;
            hide
        };
        if should_hide {
            self.shared.post_signal(ThreadSignal::Hide);
        } else {
            self.shared.post_signal(ThreadSignal::Idle);
        }
    }

    /// Configures the Q-10 idle state appearance (`MiniPill` default, `Dot`, or `Hidden`).
    pub fn set_idle_style(&self, style: IdleStyle) {
        let signal = {
            let mut rt = self.shared.lock();
            let previous = rt.pill.idle_style;
            if previous == style {
                return;
            }
            rt.pill.idle_style = style;
            if matches!(rt.pill.mode, PillMode::Idle) {
                if style == IdleStyle::Hidden {
                    rt.desired_visible = false;
                    Some(ThreadSignal::Hide)
                } else if previous == IdleStyle::Hidden && !rt.standby_only {
                    rt.desired_visible = true;
                    Some(ThreadSignal::Idle)
                } else if rt.desired_visible {
                    Some(ThreadSignal::Sync { restart_timer: false })
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(sig) = signal {
            self.shared.post_signal(sig);
        }
    }

    /// Sets whether the native pill floats below the top bezel or docks flush at the screen edge.
    pub fn set_notch_style(&self, style: NotchStyle) {
        let signal = {
            let mut rt = self.shared.lock();
            if rt.pill.notch_style == style {
                return;
            }
            rt.pill.notch_style = style;
            rt.desired_visible.then_some(ThreadSignal::Sync { restart_timer: false })
        };
        if let Some(sig) = signal {
            self.shared.post_signal(sig);
        }
    }

    /// Updates the native dot into an immediate activity pill (with optional volume progress `0.0..=1.0`).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn pulse_activity(&self, progress: Option<f32>, accent_rgb: u32) {
        {
            let mut rt = self.shared.lock();
            let accent = Rgba::from_rgb(accent_rgb, 255);
            rt.pulse_accent_override = Some(accent);
            rt.pill.theme.accent = accent;
            rt.pill.mode = match progress {
                Some(p) => {
                    let pct = (p.clamp(0.0, 1.0) * 100.0).round() as u8;
                    let muted = rt.pill.volume.is_some_and(|v| v.muted);
                    rt.pill.volume = Some(VolumeState { percent: pct, muted });
                    PillMode::Volume { percent: pct, muted }
                }
                None => PillMode::Idle,
            };
            rt.desired_visible = true;
        }
        self.shared.post_signal(ThreadSignal::Sync { restart_timer: true });
    }

    /// Hides the native dot (e.g. once the WebView notch has painted its first frame).
    pub fn hide(&self) {
        {
            let mut rt = self.shared.lock();
            rt.desired_visible = false;
        }
        self.shared.post_signal(ThreadSignal::Hide);
    }

    /// Repositions the native pill onto another monitor.
    pub fn move_to_monitor(&self, monitor: MonitorInfo) {
        {
            let mut rt = self.shared.lock();
            rt.monitor = monitor;
        }
        self.shared.post_signal(ThreadSignal::MoveMonitor);
    }

    /// Sets who hears clicks on the pill (D-19 A1). The handler runs on the pill's window thread, so it must not block.
    pub fn set_input_handler(&self, handler: InputHandler) {
        *self.shared.input_handler.lock().unwrap_or_else(PoisonError::into_inner) = Some(handler);
    }

    /// Turns hover-to-open on with a dwell from config, or off (`None`, the default: click-first).
    pub fn set_hover_open(&self, delay: Option<Duration>) {
        self.shared.lock().input.set_hover_open(delay);
    }

    /// Returns whether the native pill window is currently visible.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.shared.visible.load(Ordering::Acquire)
    }

    /// Updates the active theme used by subsequent and current native pill renders.
    pub fn set_theme(&self, theme: NativePillTheme) {
        let should_repaint = {
            let mut rt = self.shared.lock();
            if rt.base_theme == theme {
                return;
            }
            rt.base_theme = theme;
            if rt.pulse_accent_override.is_none() {
                rt.pill.theme = theme;
            }
            rt.desired_visible
        };
        if should_repaint {
            self.shared.post_signal(ThreadSignal::Sync { restart_timer: false });
        }
    }

    /// Configures an optional auto-dismiss delay back to [`PillMode::Idle`] for `Lite` mode.
    pub fn set_activity_timeout(&self, timeout: Option<Duration>) {
        let ms = timeout.and_then(|d| u32::try_from(d.as_millis()).ok()).map(|m| m.max(1));
        self.shared.lock().activity_timeout_ms = ms;
    }

    /// When `true`, incoming engine events keep [`PillState`] synchronized silently without
    /// popping the native pill on top of an already visible WebView notch.
    pub fn set_standby_only(&self, standby_only: bool) {
        self.shared.lock().standby_only = standby_only;
    }

    /// Displays live playback volume `%` (or `MUTE`) using the active theme colours.
    pub fn show_volume(&self, volume: VolumeState) {
        let normalized = VolumeState { percent: volume.percent.min(100), muted: volume.muted };
        {
            let mut rt = self.shared.lock();
            rt.pulse_accent_override = None;
            rt.pill.theme = rt.base_theme;
            rt.pill.volume = Some(normalized);
            rt.pill.mode = PillMode::Volume { percent: normalized.percent, muted: normalized.muted };
            rt.desired_visible = true;
        }
        self.shared.post_signal(ThreadSignal::Sync { restart_timer: true });
    }

    /// Displays the microphone mute/live activity pill using the active theme colours.
    pub fn show_mic_mute(&self, muted: bool) {
        {
            let mut rt = self.shared.lock();
            rt.pulse_accent_override = None;
            rt.pill.theme = rt.base_theme;
            rt.pill.mic_muted = muted;
            rt.pill.mode = PillMode::MicMute { muted };
            rt.desired_visible = true;
        }
        self.shared.post_signal(ThreadSignal::Sync { restart_timer: true });
    }

    /// Renders an arbitrary [`PillState`] snapshot onto the native pill window.
    pub fn show_state(&self, state: &PillState) {
        let restart_timer = !matches!(state.mode, PillMode::Idle);
        {
            let mut rt = self.shared.lock();
            rt.pulse_accent_override = None;
            rt.base_theme = state.theme;
            let interaction = rt.pill.interaction;
            rt.pill = state.clone();
            rt.pill.interaction = interaction;
            rt.desired_visible = true;
        }
        self.shared.post_signal(ThreadSignal::Sync { restart_timer });
    }

    /// Synchronizes state from an [`EngineSnapshot`] without stealing focus or revealing expired activities.
    /// Applies the saved idle style. The theme stays the caller's choice ([`Self::set_theme`]), so a theme
    /// built from CSS overrides isn't replaced by the built-in palette on every snapshot.
    pub fn apply_snapshot(&self, snapshot: &EngineSnapshot) {
        self.set_idle_style(snapshot.settings.appearance.idle_style);
        let repaint_idle = {
            let mut rt = self.shared.lock();
            let prev_mic = rt.pill.mic_muted;
            let prev_vol = rt.pill.volume;
            rt.pill.apply_snapshot(snapshot);
            rt.desired_visible
                && matches!(rt.pill.mode, PillMode::Idle)
                && (rt.pill.mic_muted != prev_mic || rt.pill.volume != prev_vol)
        };
        if repaint_idle {
            self.shared.post_signal(ThreadSignal::Sync { restart_timer: false });
        }
    }

    /// Applies an [`Envelope`] from `ice_core::engine` and reveals/updates the pill on activity events.
    pub fn apply_envelope(&self, envelope: &Envelope) -> EnvelopeOutcome {
        if let EngineEvent::SettingsChanged { settings } = &envelope.event {
            self.set_idle_style(settings.appearance.idle_style);
        }
        let (outcome, reveal) = {
            let mut rt = self.shared.lock();
            rt.pulse_accent_override = None;
            rt.pill.theme = rt.base_theme;
            let outcome = rt.pill.apply_envelope(envelope);
            let is_activity = matches!(
                envelope.event,
                EngineEvent::VolumeChanged { .. }
                    | EngineEvent::MicVolumeChanged { .. }
                    | EngineEvent::MicMuteChanged { .. }
                    | EngineEvent::Switched { .. }
                    | EngineEvent::DeviceArrived { .. }
                    | EngineEvent::DeviceLeft { .. }
            );
            let reveal = outcome == EnvelopeOutcome::Updated && is_activity && !rt.standby_only;
            if reveal {
                rt.desired_visible = true;
            }
            (outcome, reveal)
        };
        if reveal {
            self.shared.post_signal(ThreadSignal::Sync { restart_timer: true });
        }
        outcome
    }

    /// Returns a snapshot of the tracked [`PillState`] for diagnostics and tests.
    #[must_use]
    pub fn pill_state(&self) -> PillState {
        self.shared.lock().pill.clone()
    }

    /// Subscribes to `engine` following the D-03 protocol (subscribe before snapshot read),
    /// then forwards events onto this pill until stopped or `engine` disconnects.
    ///
    /// Holds only a weak reference internally for revision-gap resnapshots so dropping the caller's
    /// `Engine` cleanly closes the channel and ends the subscriber thread.
    #[must_use]
    pub fn subscribe_engine(&self, engine: Arc<dyn Engine>) -> NativePillSubscription {
        let rx = engine.subscribe();
        let initial = engine.snapshot();
        self.apply_snapshot(&initial);
        let weak_engine: Weak<dyn Engine> = Arc::downgrade(&engine);
        drop(engine);

        let cancelled = Arc::new(AtomicBool::new(false));
        let cancel_flag = Arc::clone(&cancelled);
        let shared = Arc::clone(&self.shared);
        let thread = thread::Builder::new()
            .name("ice-native-dot-sub".into())
            .spawn(move || {
                while !cancel_flag.load(Ordering::Acquire) {
                    let Ok(env) = rx.recv() else {
                        break;
                    };
                    if cancel_flag.load(Ordering::Acquire) {
                        break;
                    }
                    let (outcome, reveal) = {
                        let mut rt = shared.lock();
                        rt.pulse_accent_override = None;
                        rt.pill.theme = rt.base_theme;
                        let outcome = rt.pill.apply_envelope(&env);
                        if outcome == EnvelopeOutcome::NeedsResnapshot {
                            (outcome, false)
                        } else {
                            let is_activity = matches!(
                                env.event,
                                EngineEvent::VolumeChanged { .. }
                                    | EngineEvent::MicVolumeChanged { .. }
                                    | EngineEvent::MicMuteChanged { .. }
                                    | EngineEvent::Switched { .. }
                                    | EngineEvent::DeviceArrived { .. }
                                    | EngineEvent::DeviceLeft { .. }
                            );
                            let reveal = outcome == EnvelopeOutcome::Updated && is_activity && !rt.standby_only;
                            if reveal {
                                rt.desired_visible = true;
                            }
                            (outcome, reveal)
                        }
                    };
                    match outcome {
                        EnvelopeOutcome::Updated if reveal => {
                            shared.post_signal(ThreadSignal::Sync { restart_timer: true });
                        }
                        EnvelopeOutcome::NeedsResnapshot => {
                            let Some(strong) = weak_engine.upgrade() else {
                                break;
                            };
                            let fresh = strong.snapshot();
                            let repaint = {
                                let mut rt = shared.lock();
                                rt.pill.apply_snapshot(&fresh);
                                rt.desired_visible
                            };
                            if repaint {
                                shared.post_signal(ThreadSignal::Sync { restart_timer: false });
                            }
                        }
                        EnvelopeOutcome::Updated | EnvelopeOutcome::IgnoredStale => {}
                    }
                }
            })
            .ok();

        NativePillSubscription { cancelled, thread }
    }
}

impl Drop for NativeDotHandle {
    fn drop(&mut self) {
        self.shared.closing.store(true, Ordering::Release);
        let raw = self.shared.hwnd_raw.swap(0, Ordering::AcqRel);
        if raw != 0 {
            let hwnd = HWND(raw as *mut c_void);
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
        // Without a class cursor the pill (hit-testable since D-19) leaves whatever cursor was last shown, which can be
        // the busy one. Plain arrow.
        // SAFETY: loading the shared system arrow; it needs no ownership or release.
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        lpszClassName: class_name,
        ..Default::default()
    };
    // SAFETY: registering an immutable static window class name; duplicate registration is harmless.
    unsafe { RegisterClassExW(&raw const wc) };

    // Not WS_EX_TRANSPARENT: the pill takes clicks (D-19 A1). Per-pixel alpha still lets the transparent corners through.
    let ex_style = WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
    let phys_w = monitor.logical_to_physical(IDLE_W);
    let phys_h = monitor.logical_to_physical(IDLE_H);
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

    input::attach(hwnd, &shared);
    let initial_state = PillState::new(NativePillTheme::default());
    let _ = paint_pill(hwnd, &monitor, &initial_state);
    let _ = ready_tx.send(Ok(hwnd.0 as isize));

    let mut msg = MSG::default();
    // SAFETY: standard Win32 message loop for the window owned by this thread.
    while unsafe { GetMessageW(&raw mut msg, None, 0, 0) }.as_bool() {
        if msg.message == WM_TIMER && msg.hwnd == hwnd && msg.wParam.0 == ACTIVITY_TIMER_ID {
            // SAFETY: one-shot activity expiry timer; stop immediately before repainting idle.
            let _ = unsafe { KillTimer(Some(hwnd), ACTIVITY_TIMER_ID) };
            let (pill, active_mon, still_visible) = {
                let mut rt = shared.lock();
                rt.pulse_accent_override = None;
                rt.pill.theme = rt.base_theme;
                rt.pill.mode = PillMode::Idle;
                if rt.pill.idle_style == IdleStyle::Hidden {
                    rt.desired_visible = false;
                }
                (rt.pill.clone(), rt.monitor.clone(), rt.desired_visible)
            };
            if still_visible {
                let _ = paint_pill(hwnd, &active_mon, &pill);
                reveal_topmost_noactivate(hwnd);
                shared.visible.store(true, Ordering::Release);
            } else {
                // SAFETY: hides layered window when idle_style is Hidden.
                let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
                shared.visible.store(false, Ordering::Release);
            }
            continue;
        }

        if msg.message == WM_DOT_CMD {
            let (signals, pill, active_mon, desired_visible, timeout_ms) = {
                let mut rt = shared.lock();
                let signals: Vec<ThreadSignal> = rt.signals.drain(..).collect();
                (signals, rt.pill.clone(), rt.monitor.clone(), rt.desired_visible, rt.activity_timeout_ms)
            };
            if signals.is_empty() {
                continue;
            }
            let mut need_paint = false;
            let mut need_hide = false;
            let mut want_timer = false;
            let mut kill_timer = false;
            for sig in signals {
                match sig {
                    ThreadSignal::Sync { restart_timer } => {
                        need_paint = true;
                        need_hide = false;
                        if restart_timer {
                            want_timer = true;
                            kill_timer = false;
                        }
                    }
                    ThreadSignal::Idle => {
                        need_paint = true;
                        need_hide = false;
                        want_timer = false;
                        kill_timer = true;
                    }
                    ThreadSignal::Hide => {
                        need_hide = true;
                        need_paint = false;
                        want_timer = false;
                        kill_timer = true;
                    }
                    ThreadSignal::MoveMonitor => {
                        need_paint = true;
                    }
                }
            }

            if kill_timer {
                // SAFETY: cancels any pending activity dismiss timer on the owning window thread.
                let _ = unsafe { KillTimer(Some(hwnd), ACTIVITY_TIMER_ID) };
            }
            if need_hide || !desired_visible {
                input::reset(hwnd, &shared);
                // SAFETY: hides layered window without changing activation.
                let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
                shared.visible.store(false, Ordering::Release);
                continue;
            }
            if need_paint {
                let _ = paint_pill(hwnd, &active_mon, &pill);
                reveal_topmost_noactivate(hwnd);
                shared.visible.store(true, Ordering::Release);
            }
            if want_timer
                && !matches!(pill.mode, PillMode::Idle)
                && let Some(ms) = timeout_ms
            {
                // SAFETY: installs or replaces a Win32 timer on the thread's own window.
                let _ = unsafe { SetTimer(Some(hwnd), ACTIVITY_TIMER_ID, ms, None) };
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

fn reveal_topmost_noactivate(hwnd: HWND) {
    // SAFETY: reveals and promotes `hwnd` without stealing keyboard or window focus.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_KEEP_SIZE);
    }
}

unsafe extern "system" fn dot_wnd_proc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if let Some(result) = input::dispatch(hwnd, umsg, wparam, lparam) {
        return result;
    }
    match umsg {
        WM_CLOSE => {
            // SAFETY: destroys timer and window on its owning thread.
            unsafe {
                let _ = KillTimer(Some(hwnd), ACTIVITY_TIMER_ID);
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            input::detach(hwnd);
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
            // SAFETY: releases the compatible DC allocated in `paint_pill`.
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

const fn logical_dimensions(mode: PillMode, idle_style: IdleStyle) -> (i32, i32) {
    match mode {
        PillMode::Idle => match idle_style {
            IdleStyle::Dot => (IDLE_DOT_W, IDLE_DOT_H),
            IdleStyle::MiniPill | IdleStyle::Hidden => (IDLE_W, IDLE_H),
        },
        PillMode::Volume { .. } | PillMode::MicVolume { .. } => (VOLUME_W, VOLUME_H),
        PillMode::MicMute { .. } => (MIC_W, MIC_H),
        PillMode::DeviceEvent { .. } => (DEVICE_W, DEVICE_H),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameMetrics {
    body_w: i32,
    body_h: i32,
    ear: i32,
    top_offset: i32,
}

impl FrameMetrics {
    const fn total_w(self) -> i32 {
        self.body_w + self.ear * 2
    }
}

// Twin: `app/ui/src/lib/notch/notchStyle.ts` (Floating vs Edge Nook top offset, ear radius and +2 px docked height).
const fn frame_metrics(mode: PillMode, idle_style: IdleStyle, notch_style: NotchStyle) -> FrameMetrics {
    let (body_w, base_h) = logical_dimensions(mode, idle_style);
    match notch_style {
        NotchStyle::Floating => FrameMetrics { body_w, body_h: base_h, ear: 0, top_offset: LOGICAL_TOP_OFFSET },
        NotchStyle::Nook => {
            let ear = if matches!(mode, PillMode::Idle) && matches!(idle_style, IdleStyle::Dot) { 6 } else { 8 };
            FrameMetrics { body_w, body_h: base_h + 2, ear, top_offset: 0 }
        }
    }
}

fn paint_pill(hwnd: HWND, monitor: &MonitorInfo, state: &PillState) -> Result<()> {
    let metrics = frame_metrics(state.mode, state.idle_style, state.notch_style);
    let w = monitor.logical_to_physical(metrics.total_w()).max(16);
    let h = monitor.logical_to_physical(metrics.body_h).max(8);
    let x = monitor.left + (monitor.width - w) / 2;
    let y = monitor.top + monitor.logical_to_physical(metrics.top_offset);

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
    rasterize_pill_frame(pixels, w, h, monitor.scale_factor as f32, state);

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

/// Renders one anti-aliased premultiplied BGRA pill frame into `pixels` (`w * h`).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::too_many_lines)]
pub(crate) fn rasterize_pill_frame(pixels: &mut [u32], w: i32, h: i32, scale: f32, state: &PillState) {
    let scale = scale.max(1.0);
    let lw = w as f32 / scale;
    let lh = h as f32 / scale;
    let aa = 0.75 / scale;

    let metrics = frame_metrics(state.mode, state.idle_style, state.notch_style);
    let is_nook = state.notch_style == NotchStyle::Nook;
    let ear = if is_nook { metrics.ear as f32 } else { 0.0 };
    let body_w = (lw - 2.0 * ear).max(12.0);
    let ox = ear;

    let cx = lw * 0.5;
    let shape_cy = lh * 0.5;
    // Shift interior elements down by 1 px in Edge Nook so content breathes below y = 0.
    let cy = shape_cy + if is_nook { 1.0 } else { 0.0 };
    let radius = if is_nook { (lh * 0.44).min(13.0) } else { (lh - 2.0) * 0.5 };
    let half_w = (body_w - 2.0) * 0.5;
    let half_h = (lh - 2.0) * 0.5;

    let (bg_r, bg_g, bg_b, bg_a) = state.theme.bg.as_f32_rgba();
    let (surf_r, surf_g, surf_b, surf_a) = state.theme.surface.as_f32_rgba();
    let (fg_r, fg_g, fg_b, _) = state.theme.fg.as_f32_rgba();
    let (fgm_r, fgm_g, fgm_b, _) = state.theme.fg_muted.as_f32_rgba();
    let (acc_r, acc_g, acc_b, _) = state.theme.accent.as_f32_rgba();
    let (ok_r, ok_g, ok_b, _) = state.theme.ok.as_f32_rgba();
    let (dng_r, dng_g, dng_b, _) = state.theme.danger.as_f32_rgba();

    let (rim_color, rim_strength) = match state.mode {
        PillMode::Idle if state.mic_muted => (state.theme.danger, 0.78),
        PillMode::Idle => (state.theme.rim, 0.40),
        PillMode::Volume { muted: true, .. } | PillMode::MicVolume { muted: true, .. } => (state.theme.danger, 0.78),
        PillMode::MicMute { muted: true } | PillMode::DeviceEvent { is_removal: true, .. } => {
            (state.theme.danger, 0.85)
        }
        PillMode::MicMute { muted: false } => (state.theme.ok, 0.75),
        PillMode::Volume { muted: false, .. }
        | PillMode::MicVolume { muted: false, .. }
        | PillMode::DeviceEvent { is_removal: false, .. } => (state.theme.rim, 0.70),
    };
    let (rim_r, rim_g, rim_b, rim_a) = rim_color.as_f32_rgba();
    let rim_strength = match state.interaction {
        PillInteraction::None => rim_strength,
        PillInteraction::Hover => (rim_strength + HOVER_RIM_BOOST).min(1.0),
        PillInteraction::Pressed => PRESSED_RIM_STRENGTH,
    };

    let label_buf: String;
    let mut label_str = "";
    let mut label_x = 0.0f32;
    let mut label_w = 0.0f32;
    let mut label_h = 11.5f32;
    let mut label_align = Align::Center;
    let mut label_color = (fg_r, fg_g, fg_b);
    let mut bar_bounds: Option<(f32, f32, f32, bool)> = None;

    match state.mode {
        PillMode::Idle => {
            if state.idle_style == IdleStyle::MiniPill {
                // A muted microphone is a privacy state, so it outranks the volume readout.
                if state.mic_muted {
                    label_str = "MIC OFF";
                    label_x = ox + 28.0;
                    label_w = 76.0;
                    label_h = 11.5;
                    label_color = (dng_r, dng_g, dng_b);
                } else if let Some(v) = state.volume {
                    label_buf = if v.muted { "MUTE".to_string() } else { format!("{}%", v.percent.min(100)) };
                    label_str = &label_buf;
                    label_x = ox + 28.0;
                    label_w = 76.0;
                    label_h = 11.5;
                    label_align = Align::Right;
                    label_color = if v.muted { (dng_r, dng_g, dng_b) } else { (fgm_r, fgm_g, fgm_b) };
                }
            }
        }
        PillMode::Volume { percent, muted } | PillMode::MicVolume { percent, muted } => {
            label_buf = if muted { "MUTE".to_string() } else { format!("{}%", percent.min(100)) };
            label_str = &label_buf;
            label_x = ox + 141.0;
            label_w = 40.0;
            label_color = if muted { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
            bar_bounds = Some((ox + 34.0, ox + 134.0, f32::from(percent.min(100)) / 100.0, muted));
        }
        PillMode::MicMute { muted } => {
            label_str = if muted { "MIC OFF" } else { "MIC ON" };
            label_x = ox + 42.0;
            label_w = 100.0;
            label_color = if muted { (dng_r, dng_g, dng_b) } else { (ok_r, ok_g, ok_b) };
        }
        PillMode::DeviceEvent { is_removal, .. } => {
            if is_removal {
                label_str = "OFF";
                label_color = (dng_r, dng_g, dng_b);
                bar_bounds = Some((ox + 34.0, ox + 126.0, 0.0, true));
            } else if let Some(v) = state.volume {
                label_buf = if v.muted { "MUTE".to_string() } else { format!("{}%", v.percent.min(100)) };
                label_str = &label_buf;
                label_color = if v.muted { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
                bar_bounds = Some((ox + 34.0, ox + 126.0, f32::from(v.percent.min(100)) / 100.0, v.muted));
            } else {
                label_str = "ON";
                label_color = (acc_r, acc_g, acc_b);
                bar_bounds = Some((ox + 34.0, ox + 126.0, 1.0, false));
            }
            label_x = ox + 132.0;
            label_w = 40.0;
        }
    }

    let glyph_mask = if !label_str.is_empty() {
        let mut mask = GlyphMask::new(w, h, scale);
        mask.draw_run(label_str, FontFace::Ui, (label_x, label_w, cy, label_h), label_align);
        Some(mask)
    } else {
        None
    };

    for py in 0..h {
        let ly = (py as f32 + 0.5) / scale;
        for px in 0..w {
            let lx = (px as f32 + 0.5) / scale;
            let sdf = if is_nook {
                sd_nook_notch(lx, ly, cx, half_w, lh, radius, ear)
            } else {
                sd_round_box(lx, ly, cx, shape_cy, half_w, half_h, radius)
            };
            let coverage = ((aa * 0.5 - sdf) / aa).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                pixels[(py * w + px) as usize] = 0;
                continue;
            }

            let mut cr = bg_r;
            let mut cg = bg_g;
            let mut cb = bg_b;
            let base_alpha = if matches!(state.mode, PillMode::Idle) { bg_a * 0.95 } else { bg_a.max(0.88) };
            let mut alpha = base_alpha * coverage;

            // 1 px specular rim around outer edge (skipped cleanly when rim alpha == 0).
            if sdf > -1.25 && rim_a > 0.0 {
                let rim_t = ((sdf + 1.25) / 1.25).clamp(0.0, 1.0) * rim_a * rim_strength;
                cr = cr * (1.0 - rim_t) + rim_r * rim_t;
                cg = cg * (1.0 - rim_t) + rim_g * rim_t;
                cb = cb * (1.0 - rim_t) + rim_b * rim_t;
                alpha = alpha.max(rim_t * coverage);
            }

            if state.interaction == PillInteraction::Pressed {
                blend_over(&mut cr, &mut cg, &mut cb, fg_r, fg_g, fg_b, PRESSED_TINT_ALPHA);
                alpha = alpha.max(PRESSED_MIN_ALPHA * coverage);
            }

            match state.mode {
                PillMode::Idle => {
                    // `IdlePill.svelte` uses `padding: 0 12px` and a `7 px` status dot (`r = 3.5`).
                    let icon_cx =
                        if state.idle_style == IdleStyle::MiniPill && !label_str.is_empty() { ox + 17.0 } else { cx };
                    if state.mic_muted {
                        // D-12 accessibility: render a distinct slashed-mic glyph shape in addition to danger colour.
                        let inv_scale = 1.25f32;
                        let sx = icon_cx + (lx - icon_cx) * inv_scale;
                        let sy = cy + (ly - cy) * inv_scale;
                        let (icon_cov, is_slash) = mic_icon_cov(sx, sy, icon_cx, cy, true, aa * inv_scale);
                        if icon_cov > 0.0 {
                            let (ir, ig, ib) = if is_slash { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
                            blend_over(&mut cr, &mut cg, &mut cb, ir, ig, ib, icon_cov);
                        }
                    } else {
                        let dot_d = sd_circle(lx, ly, icon_cx, cy, 3.5);
                        let dot_cov = smooth_cov(dot_d, aa);
                        if dot_cov > 0.0 {
                            blend_over(&mut cr, &mut cg, &mut cb, acc_r, acc_g, acc_b, dot_cov * 0.92);
                        }
                    }
                }
                PillMode::Volume { muted, .. } => {
                    let (icon_cov, is_danger_px) = speaker_icon_cov(lx, ly, ox + 20.0, cy, muted, aa);
                    if icon_cov > 0.0 {
                        let (ir, ig, ib) =
                            if muted && is_danger_px { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
                        blend_over(&mut cr, &mut cg, &mut cb, ir, ig, ib, icon_cov);
                    }
                    if state.mic_muted {
                        let badge_cov = smooth_cov(
                            sd_circle(lx, ly, ox + body_w - 8.0, 7.5 + if is_nook { 1.0 } else { 0.0 }, 2.0),
                            aa,
                        );
                        blend_over(&mut cr, &mut cg, &mut cb, dng_r, dng_g, dng_b, badge_cov);
                    }
                }
                PillMode::MicVolume { muted, .. } => {
                    let (icon_cov, is_slash) = mic_icon_cov(lx, ly, ox + 20.0, cy, muted, aa);
                    if icon_cov > 0.0 {
                        let (ir, ig, ib) = if muted && is_slash { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
                        blend_over(&mut cr, &mut cg, &mut cb, ir, ig, ib, icon_cov);
                    }
                }
                PillMode::MicMute { muted } => {
                    let (icon_cov, is_slash) = mic_icon_cov(lx, ly, ox + 23.0, cy, muted, aa);
                    if icon_cov > 0.0 {
                        let (ir, ig, ib) = if muted && is_slash {
                            (dng_r, dng_g, dng_b)
                        } else if muted {
                            (fg_r, fg_g, fg_b)
                        } else {
                            (ok_r, ok_g, ok_b)
                        };
                        blend_over(&mut cr, &mut cg, &mut cb, ir, ig, ib, icon_cov);
                    }
                }
                PillMode::DeviceEvent { is_headphones, spatial_active, is_removal } => {
                    let (icon_cov, is_danger_px) = if is_headphones {
                        headphones_icon_cov(lx, ly, ox + 20.0, cy, is_removal, aa)
                    } else {
                        speaker_icon_cov(lx, ly, ox + 20.0, cy, is_removal, aa)
                    };
                    if icon_cov > 0.0 {
                        let (ir, ig, ib) = if is_danger_px { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
                        blend_over(&mut cr, &mut cg, &mut cb, ir, ig, ib, icon_cov);
                    }
                    if spatial_active && !is_removal {
                        let sp_cov = smooth_cov(sd_circle(lx, ly, ox + body_w - 9.0, cy, 2.3), aa);
                        blend_over(&mut cr, &mut cg, &mut cb, acc_r, acc_g, acc_b, sp_cov);
                    }
                }
            }

            if let Some((bx0, bx1, frac, muted_bar)) = bar_bounds {
                let bw = bx1 - bx0;
                let bh_half = 2.5f32;
                let track_sd = sd_round_box(lx, ly, bx0 + bw * 0.5, cy, bw * 0.5, bh_half, bh_half);
                let track_cov = smooth_cov(track_sd, aa);
                if track_cov > 0.0 {
                    blend_over(&mut cr, &mut cg, &mut cb, surf_r, surf_g, surf_b, track_cov * surf_a * 0.72);
                    let fill_w = (bw * frac).max(if frac > 0.0 { bh_half * 2.0 } else { 0.0 });
                    if fill_w > 0.0 {
                        let fill_sd = sd_round_box(lx, ly, bx0 + fill_w * 0.5, cy, fill_w * 0.5, bh_half, bh_half);
                        let fill_cov = smooth_cov(fill_sd, aa) * track_cov;
                        let (fr, fg_c, fb, strength) =
                            if muted_bar { (dng_r, dng_g, dng_b, 0.45) } else { (acc_r, acc_g, acc_b, 0.96) };
                        blend_over(&mut cr, &mut cg, &mut cb, fr, fg_c, fb, fill_cov * strength);
                    }
                }
            }

            if let Some(mask) = &glyph_mask {
                let cov = mask.coverage_at(px, py);
                if cov > 0.0 {
                    blend_over(&mut cr, &mut cg, &mut cb, label_color.0, label_color.1, label_color.2, cov);
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

fn blend_over(cr: &mut f32, cg: &mut f32, cb: &mut f32, sr: f32, sg: f32, sb: f32, a: f32) {
    let t = a.clamp(0.0, 1.0);
    if t <= 0.0 {
        return;
    }
    *cr = *cr * (1.0 - t) + sr * t;
    *cg = *cg * (1.0 - t) + sg * t;
    *cb = *cb * (1.0 - t) + sb * t;
}

fn smooth_cov(sdf: f32, aa: f32) -> f32 {
    ((aa * 0.5 - sdf) / aa).clamp(0.0, 1.0)
}

fn sd_round_box(px: f32, py: f32, cx: f32, cy: f32, half_w: f32, half_h: f32, r: f32) -> f32 {
    let r = r.min(half_w).min(half_h).max(0.0);
    let dx = (px - cx).abs() - (half_w - r).max(0.0);
    let dy = (py - cy).abs() - (half_h - r).max(0.0);
    dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - r
}

/// Analytical SDF for the top-docked `NotchStyle::Nook` silhouette (flat flush top at `ly = 0`,
/// concave top-left/top-right ears of radius `ear`, and convex bottom corners of radius `bottom_r`).
fn sd_nook_notch(lx: f32, ly: f32, cx: f32, half_body_w: f32, h: f32, bottom_r: f32, ear: f32) -> f32 {
    let ax = (lx - cx).abs();
    let bottom_y = (h - 1.0).max(1.0);
    let r = bottom_r.min(half_body_w).min((bottom_y - ear).max(1.0)).max(0.0);
    if ly > bottom_y - r {
        let bx = ax - (half_body_w - r).max(0.0);
        let by = ly - (bottom_y - r);
        return bx.max(0.0).hypot(by.max(0.0)) + bx.max(by).min(0.0) - r;
    }
    let ear_outer = half_body_w + ear;
    if ear > 0.0 && ax > half_body_w && ly <= ear {
        if ax > ear_outer {
            return (ax - ear_outer).hypot(ly.max(0.0));
        }
        return ear - (ax - ear_outer).hypot(ly - ear);
    }
    ax - half_body_w
}

fn sd_circle(px: f32, py: f32, cx: f32, cy: f32, r: f32) -> f32 {
    (px - cx).hypot(py - cy) - r
}

fn sd_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let pax = px - ax;
    let pay = py - ay;
    let bax = bx - ax;
    let bay = by - ay;
    let denom = bax * bax + bay * bay;
    let h = if denom <= 1e-6 { 0.0 } else { ((pax * bax + pay * bay) / denom).clamp(0.0, 1.0) };
    (pax - bax * h).hypot(pay - bay * h)
}

fn speaker_icon_cov(lx: f32, ly: f32, cx: f32, cy: f32, muted: bool, aa: f32) -> (f32, bool) {
    if (lx - cx).abs() > 8.5 || (ly - cy).abs() > 8.5 {
        return (0.0, false);
    }
    let rx = lx - cx;
    let ry = ly - cy;
    let body_sd = sd_round_box(rx, ry, -3.5, 0.0, 1.8, 2.3, 0.5);
    let cone_sd = if (-2.2..=1.2).contains(&rx) {
        let t = (rx + 2.2) / 3.4;
        let half_h = 2.3 + t * 2.5;
        ry.abs() - half_h
    } else {
        10.0
    };
    let mut base_sd = body_sd.min(cone_sd);

    if muted {
        let slash_sd = sd_segment(rx, ry, -5.0, 5.0, 5.2, -5.0) - 0.85;
        let slash_cov = smooth_cov(slash_sd, aa);
        if slash_cov > 0.0 {
            return (slash_cov, true);
        }
        return (smooth_cov(base_sd, aa), true);
    }

    if rx >= 1.8 && ry.abs() <= rx * 0.95 {
        let dist = rx.hypot(ry);
        let wave1 = (dist - 3.6).abs() - 0.65;
        let wave2 = (dist - 5.9).abs() - 0.65;
        base_sd = base_sd.min(wave1).min(wave2);
    }
    (smooth_cov(base_sd, aa), false)
}

fn mic_icon_cov(lx: f32, ly: f32, cx: f32, cy: f32, muted: bool, aa: f32) -> (f32, bool) {
    if (lx - cx).abs() > 8.5 || (ly - cy).abs() > 8.5 {
        return (0.0, false);
    }
    let rx = lx - cx;
    let ry = ly - cy;
    let cap_sd = sd_round_box(rx, ry, 0.0, -2.0, 1.9, 3.6, 1.8);
    let arc_sd = if (-0.8..=4.2).contains(&ry) { ((rx.hypot(ry + 0.5)) - 3.9).abs() - 0.65 } else { 10.0 };
    let stem_sd = sd_segment(rx, ry, 0.0, 3.4, 0.0, 5.6) - 0.65;
    let foot_sd = sd_segment(rx, ry, -2.6, 5.6, 2.6, 5.6) - 0.65;
    let mic_sd = cap_sd.min(arc_sd).min(stem_sd).min(foot_sd);

    if muted {
        let slash_sd = sd_segment(rx, ry, -5.2, 5.2, 5.2, -5.2) - 0.85;
        let slash_cov = smooth_cov(slash_sd, aa);
        if slash_cov > 0.0 {
            return (slash_cov, true);
        }
        return (smooth_cov(mic_sd, aa), false);
    }
    (smooth_cov(mic_sd, aa), false)
}

fn headphones_icon_cov(lx: f32, ly: f32, cx: f32, cy: f32, is_removal: bool, aa: f32) -> (f32, bool) {
    if (lx - cx).abs() > 8.5 || (ly - cy).abs() > 8.5 {
        return (0.0, false);
    }
    let rx = lx - cx;
    let ry = ly - cy;
    let band_sd = if ry <= 1.2 { (rx.hypot(ry - 1.0) - 5.0).abs() - 0.72 } else { 10.0 };
    let left_cup = sd_round_box(rx, ry, -4.6, 2.2, 1.5, 2.6, 1.1);
    let right_cup = sd_round_box(rx, ry, 4.6, 2.2, 1.5, 2.6, 1.1);
    let hp_sd = band_sd.min(left_cup).min(right_cup);

    if is_removal {
        let slash_sd = sd_segment(rx, ry, -5.2, 5.2, 5.2, -5.2) - 0.85;
        let slash_cov = smooth_cov(slash_sd, aa);
        if slash_cov > 0.0 {
            return (slash_cov, true);
        }
        return (smooth_cov(hp_sd, aa), true);
    }
    (smooth_cov(hp_sd, aa), false)
}

#[cfg(test)]
mod tests {
    use ice_core::engine::Command;
    use ice_core::engine::fake::FakeEngine;
    use ice_core::media::NowPlaying;

    use super::*;

    fn test_monitor(scale_factor: f64) -> MonitorInfo {
        MonitorInfo {
            id: "\\\\.\\DISPLAY1".into(),
            display_path: "\\\\?\\DISPLAY#TEST0001#1".into(),
            friendly_name: "Primary Test Display".into(),
            is_primary: true,
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
            work_left: 0,
            work_top: 0,
            work_width: 1920,
            work_height: 1032,
            dpi_x: (96.0 * scale_factor) as u32,
            scale_factor,
            refresh_hz: 144,
        }
    }

    fn render_test(mode: PillMode, theme: NativePillTheme, mic_muted: bool, scale: f32) -> (i32, i32, Vec<u32>) {
        let mut st = PillState::new(theme);
        st.mode = mode;
        st.mic_muted = mic_muted;
        if let PillMode::Volume { percent, muted } = mode {
            st.volume = Some(VolumeState { percent, muted });
        } else if let PillMode::MicVolume { percent, .. } = mode {
            st.mic_volume = Some(percent);
        }
        let (lw, lh) = logical_dimensions(mode, st.idle_style);
        let w = (lw as f32 * scale).round() as i32;
        let h = (lh as f32 * scale).round() as i32;
        let mut px = vec![0u32; (w * h) as usize];
        rasterize_pill_frame(&mut px, w, h, scale, &st);
        (w, h, px)
    }

    #[test]
    fn builtin_themes_match_base_css() {
        let ice = NativePillTheme::ice();
        assert_eq!(ice.bg, Rgba::new(14, 17, 22, 230));
        assert_eq!(ice.accent, Rgba::new(107, 197, 255, 255));
        assert_eq!(ice.danger, Rgba::new(240, 113, 72, 255));

        let mid = NativePillTheme::midnight();
        assert_eq!(mid.bg, Rgba::new(0, 0, 0, 245));
        assert_eq!(mid.rim, Rgba::new(255, 255, 255, 61));

        let fluent = NativePillTheme::fluent();
        assert_eq!(fluent.bg, Rgba::new(24, 24, 27, 237));
        assert_eq!(fluent.surface, Rgba::new(36, 36, 40, 240));

        let min = NativePillTheme::minimal();
        assert_eq!(min.bg, Rgba::new(18, 22, 29, 255));
        assert_eq!(min.rim, Rgba::new(0, 0, 0, 0));

        assert_eq!(NativePillTheme::builtin(ThemeKind::Midnight), mid);
    }

    #[test]
    fn idle_styles_compare_minipill_dot_and_hidden_with_d12_mic_glyph() {
        assert_eq!(IdleStyle::default(), IdleStyle::MiniPill);

        for scale in [1.0f32, 1.5f32] {
            let mut st_pill = PillState::new(NativePillTheme::ice());
            st_pill.idle_style = IdleStyle::MiniPill;
            st_pill.volume = Some(VolumeState { percent: 40, muted: false });
            let (pw, ph) = logical_dimensions(PillMode::Idle, IdleStyle::MiniPill);
            let w_pill = (pw as f32 * scale).round() as i32;
            let h_pill = (ph as f32 * scale).round() as i32;
            let mut px_pill = vec![0u32; (w_pill * h_pill) as usize];
            rasterize_pill_frame(&mut px_pill, w_pill, h_pill, scale, &st_pill);

            let mut st_pill_mic = st_pill.clone();
            st_pill_mic.mic_muted = true;
            let mut px_pill_mic = vec![0u32; (w_pill * h_pill) as usize];
            rasterize_pill_frame(&mut px_pill_mic, w_pill, h_pill, scale, &st_pill_mic);
            assert_ne!(px_pill, px_pill_mic, "MiniPill mic_muted must render distinct slashed-mic glyph");

            let mut st_dot = PillState::new(NativePillTheme::ice());
            st_dot.idle_style = IdleStyle::Dot;
            let (dw, dh) = logical_dimensions(PillMode::Idle, IdleStyle::Dot);
            let w_dot = (dw as f32 * scale).round() as i32;
            let h_dot = (dh as f32 * scale).round() as i32;
            assert!(w_dot < w_pill);
            let mut px_dot = vec![0u32; (w_dot * h_dot) as usize];
            rasterize_pill_frame(&mut px_dot, w_dot, h_dot, scale, &st_dot);

            let mut st_dot_mic = st_dot.clone();
            st_dot_mic.mic_muted = true;
            let mut px_dot_mic = vec![0u32; (w_dot * h_dot) as usize];
            rasterize_pill_frame(&mut px_dot_mic, w_dot, h_dot, scale, &st_dot_mic);
            assert_ne!(px_dot, px_dot_mic, "Dot mic_muted must render distinct slashed-mic glyph");
        }
    }

    #[test]
    fn css_override_parser_handles_formats_and_ignores_invalid() {
        let css = r"
            /* custom user theme */
            :root {
                --ice-bg: rgba(11, 20, 22, 0.92);
                --ice-surface: rgb(18 33 36 / 94%);
                --ice-rim: transparent;
                --ice-fg: #ecfeff;
                --ice-fg-muted: #9bc;
                --ice-accent: #5eead4ff;
                --ice-ok: #4ade80;
                --ice-danger: rgb(251, 113, 133);
                --ice-bar-height: 6px;
                --ice-radius: not-a-color;
            }
        ";
        let theme = NativePillTheme::from_css_overrides(NativePillTheme::ice(), css);
        assert_eq!(theme.bg, Rgba::new(11, 20, 22, 235));
        assert_eq!(theme.surface, Rgba::new(18, 33, 36, 240));
        assert_eq!(theme.rim, Rgba::new(0, 0, 0, 0));
        assert_eq!(theme.fg, Rgba::new(0xec, 0xfe, 0xff, 255));
        assert_eq!(theme.fg_muted, Rgba::new(0x99, 0xbb, 0xcc, 255));
        assert_eq!(theme.accent, Rgba::new(0x5e, 0xea, 0xd4, 255));
        assert_eq!(theme.ok, Rgba::new(0x4a, 0xde, 0x80, 255));
        assert_eq!(theme.danger, Rgba::new(251, 113, 133, 255));
    }

    #[test]
    fn pill_state_tracks_fake_engine_events_and_revision_rules() {
        let eng = FakeEngine::new();
        let rx = eng.subscribe();
        let snap = eng.snapshot();
        let mut pill = PillState::from_snapshot(&snap, NativePillTheme::ice());
        assert_eq!(pill.mode, PillMode::Idle);
        assert_eq!(pill.volume, Some(VolumeState { percent: 40, muted: false }));
        assert!(!pill.mic_muted);

        eng.send(Command::SetVolume { percent: 72 });
        let e1 = rx.recv().unwrap();
        assert_eq!(pill.apply_envelope(&e1), EnvelopeOutcome::Updated);
        assert_eq!(pill.mode, PillMode::Volume { percent: 72, muted: false });

        // Duplicate/stale revision is ignored.
        assert_eq!(pill.apply_envelope(&e1), EnvelopeOutcome::IgnoredStale);

        eng.send(Command::ToggleMute);
        let e2 = rx.recv().unwrap();
        assert_eq!(pill.apply_envelope(&e2), EnvelopeOutcome::Updated);
        assert_eq!(pill.mode, PillMode::Volume { percent: 72, muted: true });

        eng.send(Command::ToggleMicMute);
        let e3 = rx.recv().unwrap();
        assert_eq!(pill.apply_envelope(&e3), EnvelopeOutcome::Updated);
        assert_eq!(pill.mode, PillMode::MicMute { muted: true });
        assert!(pill.mic_muted);

        // Revision gap requests a fresh snapshot rather than applying out of order.
        let gap = Envelope { revision: e3.revision + 5, event: e3.event.clone() };
        assert_eq!(pill.apply_envelope(&gap), EnvelopeOutcome::NeedsResnapshot);

        eng.send(Command::TogglePair { pair_id: "main".into() });
        let e4 = rx.recv().unwrap();
        assert_eq!(pill.apply_envelope(&e4), EnvelopeOutcome::Updated);
        assert_eq!(pill.mode, PillMode::DeviceEvent { is_headphones: false, spatial_active: false, is_removal: false });
    }

    #[test]
    fn rasterizer_renders_distinct_volume_mute_and_theme_pixels() {
        let (w, h, px_40) =
            render_test(PillMode::Volume { percent: 40, muted: false }, NativePillTheme::ice(), false, 1.0);
        let (_, _, px_85) =
            render_test(PillMode::Volume { percent: 85, muted: false }, NativePillTheme::ice(), false, 1.0);
        let (_, _, px_mute) =
            render_test(PillMode::Volume { percent: 85, muted: true }, NativePillTheme::ice(), false, 1.0);
        let (_, _, px_mid) =
            render_test(PillMode::Volume { percent: 85, muted: false }, NativePillTheme::midnight(), false, 1.0);

        // Outer corner outside capsule must remain completely transparent.
        assert_eq!(px_40[0], 0);
        // Center pixel inside pill must have substantial alpha.
        let center_a = (px_40[((h / 2) * w + (w / 2)) as usize] >> 24) & 0xff;
        assert!(center_a >= 180, "expected opaque pill center, got alpha={center_a}");
        // Changing live percentage or muting changes the rendered readout/fill buffer.
        assert_ne!(px_40, px_85);
        assert_ne!(px_85, px_mute);
        // Switching theme changes the background/rim buffer.
        assert_ne!(px_85, px_mid);

        // Scaled DPI rendering (150%) produces crisp non-empty output with no panics.
        let (w150, h150, px_150) = render_test(PillMode::MicMute { muted: true }, NativePillTheme::ice(), true, 1.5);
        assert!(w150 > VOLUME_W / 2 && h150 > 30);
        assert!(px_150.iter().any(|&p| (p >> 24) > 200));
    }

    #[test]
    fn native_dot_handle_subscribes_to_fake_engine_live() {
        let engine = Arc::new(FakeEngine::new());
        let eng_trait: Arc<dyn Engine> = Arc::clone(&engine) as Arc<dyn Engine>;
        let dot = NativeDotHandle::spawn(test_monitor(1.5)).expect("spawn native dot window");
        let custom = NativePillTheme::from_css_overrides(NativePillTheme::midnight(), "--ice-accent: #5eead4;");
        dot.set_theme(custom);
        dot.show_idle();

        let sub = dot.subscribe_engine(eng_trait);
        assert_eq!(dot.pill_state().theme, custom);
        assert_eq!(dot.pill_state().volume, Some(VolumeState { percent: 40, muted: false }));

        engine.send(Command::SetVolume { percent: 65 });
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while dot.pill_state().mode != (PillMode::Volume { percent: 65, muted: false }) {
            assert!(std::time::Instant::now() < deadline, "timed out waiting for volume update");
            thread::sleep(Duration::from_millis(5));
        }

        engine.send(Command::ToggleMute);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while dot.pill_state().mode != (PillMode::Volume { percent: 65, muted: true }) {
            assert!(std::time::Instant::now() < deadline, "timed out waiting for mute update");
            thread::sleep(Duration::from_millis(5));
        }

        engine.send(Command::ToggleMicMute);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while dot.pill_state().mode != (PillMode::MicMute { muted: true }) {
            assert!(std::time::Instant::now() < deadline, "timed out waiting for mic mute update");
            thread::sleep(Duration::from_millis(5));
        }
        assert!(dot.pill_state().mic_muted);
        // `pill_state` updates on the subscriber thread; `visible` only once the window thread has painted.
        // Wait for it like the other steps (asserting at once raced under parallel test load).
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !dot.is_visible() {
            assert!(std::time::Instant::now() < deadline, "timed out waiting for the dot to show");
            thread::sleep(Duration::from_millis(5));
        }

        dot.hide();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while dot.is_visible() {
            assert!(std::time::Instant::now() < deadline, "timed out waiting for dot hide");
            thread::sleep(Duration::from_millis(5));
        }

        drop(engine);
        sub.join();
    }

    #[test]
    fn idle_mic_off_label_wins_over_a_known_volume() {
        let (lw, lh) = logical_dimensions(PillMode::Idle, IdleStyle::MiniPill);
        let render = |volume: Option<VolumeState>| {
            let mut st = PillState::new(NativePillTheme::ice());
            st.mic_muted = true;
            st.volume = volume;
            let mut px = vec![0u32; (lw * lh) as usize];
            rasterize_pill_frame(&mut px, lw, lh, 1.0, &st);
            px
        };
        // Before the fix the known volume drew its `%` label and `MIC OFF` was only reachable without one.
        let with_volume = render(Some(VolumeState { percent: 40, muted: false }));
        assert_eq!(with_volume, render(None), "MIC OFF must not depend on whether the volume is known");
        let mut unmuted = PillState::new(NativePillTheme::ice());
        unmuted.volume = Some(VolumeState { percent: 40, muted: false });
        let mut px_unmuted = vec![0u32; (lw * lh) as usize];
        rasterize_pill_frame(&mut px_unmuted, lw, lh, 1.0, &unmuted);
        assert_ne!(with_volume, px_unmuted);
    }

    #[test]
    fn media_changed_updates_the_pill_without_waking_it() {
        let media = |status| NowPlaying {
            app_id: "Spotify.exe".into(),
            title: "Canopy".into(),
            artist: "Auralis Sessions".into(),
            album: String::new(),
            status,
            position_ms: None,
            duration_ms: None,
            position_updated_ms: None,
            art: None,
        };
        let snap = FakeEngine::new().snapshot();
        let mut pill = PillState::from_snapshot(&snap, NativePillTheme::ice());
        assert_eq!(pill.media_status, snap.media.as_ref().map(|m| m.status));

        let mut rev = snap.revision + 1;
        let playing = EngineEvent::MediaChanged { media: Some(media(PlaybackStatus::Playing)) };
        assert_eq!(pill.apply_envelope(&Envelope { revision: rev, event: playing }), EnvelopeOutcome::Updated);
        assert_eq!(pill.media_status, Some(PlaybackStatus::Playing));
        assert_eq!(pill.mode, PillMode::Idle, "a track change must not pop the native pill");

        rev += 1;
        let paused = EngineEvent::MediaChanged { media: Some(media(PlaybackStatus::Paused)) };
        pill.apply_envelope(&Envelope { revision: rev, event: paused });
        assert_eq!(pill.media_status, Some(PlaybackStatus::Paused));

        rev += 1;
        pill.apply_envelope(&Envelope { revision: rev, event: EngineEvent::MediaChanged { media: None } });
        assert_eq!(pill.media_status, None);
    }

    #[test]
    fn idle_dimensions_match_webview_tokens() {
        // Twin: `app/ui/src/lib/notch/geometry.ts` (`MiniPill` resting = 116x28, `Dot` resting = 36x18).
        assert_eq!(logical_dimensions(PillMode::Idle, IdleStyle::MiniPill), (116, 28));
        assert_eq!(logical_dimensions(PillMode::Idle, IdleStyle::Dot), (36, 18));
    }

    #[test]
    fn hover_and_pressed_states_are_drawn_distinctly() {
        let (lw, lh) = logical_dimensions(PillMode::Idle, IdleStyle::MiniPill);
        let render = |interaction| {
            let mut st = PillState::new(NativePillTheme::ice());
            st.interaction = interaction;
            let mut px = vec![0u32; (lw * lh) as usize];
            rasterize_pill_frame(&mut px, lw, lh, 1.0, &st);
            px
        };
        let (none, hover, pressed) =
            (render(PillInteraction::None), render(PillInteraction::Hover), render(PillInteraction::Pressed));
        assert_ne!(none, hover);
        assert_ne!(hover, pressed);
        assert_ne!(none, pressed);
        // The pressed centre is lighter than the resting one: that is what a press looks like on a dark pill.
        let centre = ((lh / 2) * lw + lw / 2) as usize;
        let luma = |p: u32| ((p >> 16) & 0xff) + ((p >> 8) & 0xff) + (p & 0xff);
        assert!(luma(pressed[centre]) > luma(none[centre]));
    }

    #[test]
    fn mic_volume_updates_pill_and_silent_events_do_not_wake() {
        let eng = FakeEngine::new();
        let snap = eng.snapshot();
        let mut pill = PillState::from_snapshot(&snap, NativePillTheme::ice());
        assert_eq!(pill.mic_volume, snap.mic_volume.map(|p| p.min(100)));

        // MicVolumeChanged enters MicVolume mode while preserving current mic_muted flag.
        let mut rev = snap.revision + 1;
        let out1 =
            pill.apply_envelope(&Envelope { revision: rev, event: EngineEvent::MicVolumeChanged { percent: 64 } });
        assert_eq!(out1, EnvelopeOutcome::Updated);
        assert_eq!(pill.mic_volume, Some(64));
        assert_eq!(pill.mode, PillMode::MicVolume { percent: 64, muted: false });

        rev += 1;
        let _ = pill.apply_envelope(&Envelope { revision: rev, event: EngineEvent::MicMuteChanged { muted: true } });
        rev += 1;
        let _ = pill.apply_envelope(&Envelope { revision: rev, event: EngineEvent::MicVolumeChanged { percent: 38 } });
        assert_eq!(pill.mic_volume, Some(38));
        assert_eq!(pill.mode, PillMode::MicVolume { percent: 38, muted: true });

        // MicVolume rasterizer renders distinct mic glyph vs speaker glyph, and muted vs unmuted mic volume.
        let (_, _, px_vol) =
            render_test(PillMode::Volume { percent: 64, muted: false }, NativePillTheme::ice(), false, 1.0);
        let (_, _, px_mic) =
            render_test(PillMode::MicVolume { percent: 64, muted: false }, NativePillTheme::ice(), false, 1.0);
        let (_, _, px_mic_muted) =
            render_test(PillMode::MicVolume { percent: 64, muted: true }, NativePillTheme::ice(), true, 1.0);
        assert_ne!(px_vol, px_mic);
        assert_ne!(px_mic, px_mic_muted);

        // NoticesChanged and SettingsChanged advance revision without revealing a hidden pill or overriding mode.
        let dot = NativeDotHandle::spawn(test_monitor(1.0)).expect("spawn native dot window");
        dot.apply_snapshot(&snap);
        assert!(!dot.is_visible());

        let mut dot_rev = snap.revision + 1;
        let out_notices = dot.apply_envelope(&Envelope {
            revision: dot_rev,
            event: EngineEvent::NoticesChanged {
                notices: vec![ice_core::engine::Notice::NotConnected { name: "Headset".into() }],
            },
        });
        assert_eq!(out_notices, EnvelopeOutcome::Updated);
        assert_eq!(dot.pill_state().mode, PillMode::Idle);
        assert!(!dot.is_visible());

        dot_rev += 1;
        let out_settings = dot.apply_envelope(&Envelope {
            revision: dot_rev,
            event: EngineEvent::SettingsChanged { settings: snap.settings },
        });
        assert_eq!(out_settings, EnvelopeOutcome::Updated);
        assert_eq!(dot.pill_state().mode, PillMode::Idle);
        assert!(!dot.is_visible());

        dot_rev += 1;
        let out_mic_vol =
            dot.apply_envelope(&Envelope { revision: dot_rev, event: EngineEvent::MicVolumeChanged { percent: 75 } });
        assert_eq!(out_mic_vol, EnvelopeOutcome::Updated);
        assert_eq!(dot.pill_state().mode, PillMode::MicVolume { percent: 75, muted: false });

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !dot.is_visible() {
            assert!(std::time::Instant::now() < deadline, "timed out waiting for MicVolumeChanged reveal");
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn nook_style_renders_flush_top_edge_with_concave_ears_and_preserves_floating() {
        let m_float = frame_metrics(PillMode::Idle, IdleStyle::MiniPill, NotchStyle::Floating);
        assert_eq!(m_float.top_offset, LOGICAL_TOP_OFFSET);
        assert_eq!(m_float.ear, 0);

        let m_nook = frame_metrics(PillMode::Idle, IdleStyle::MiniPill, NotchStyle::Nook);
        assert_eq!(m_nook.top_offset, 0);
        assert!(m_nook.ear > 0);
        assert!(m_nook.total_w() > m_float.total_w());
        assert_eq!(m_nook.body_h, m_float.body_h + 2);

        let mut st = PillState::new(NativePillTheme::ice());
        st.notch_style = NotchStyle::Nook;
        st.volume = Some(VolumeState { percent: 55, muted: false });
        let w = m_nook.total_w();
        let h = m_nook.body_h;
        let mut px = vec![0u32; (w * h) as usize];
        rasterize_pill_frame(&mut px, w, h, 1.0, &st);

        // Top-center sits flush at y = 0 with full fill alpha.
        let top_center_a = (px[(w / 2) as usize] >> 24) & 0xff;
        assert!(top_center_a >= 180, "expected flush top center, got alpha={top_center_a}");
        // Bottom-left corner outside body remains completely transparent.
        assert_eq!(px[((h - 1) * w) as usize], 0);
        // Inside upper concave ear near top-left edge is filled, while below the curve is transparent.
        let ear_inside_x = 6;
        let ear_inside_a = (px[ear_inside_x as usize] >> 24) & 0xff;
        assert!(ear_inside_a >= 180, "expected concave ear fill near bezel, got alpha={ear_inside_a}");
        assert_eq!(px[(m_nook.ear * w + 1) as usize], 0, "cutout below ear must stay transparent");

        // Changing notchStyle via SettingsChanged updates PillState without revealing a hidden window.
        let dot = NativeDotHandle::spawn(test_monitor(1.0)).expect("spawn native dot window");
        let snap = FakeEngine::new().snapshot();
        dot.apply_snapshot(&snap);
        assert_eq!(dot.pill_state().notch_style, NotchStyle::Floating);
        assert!(!dot.is_visible());

        let mut next_settings = snap.settings;
        next_settings.appearance.notch_style = NotchStyle::Nook;
        let out = dot.apply_envelope(&Envelope {
            revision: snap.revision + 1,
            event: EngineEvent::SettingsChanged { settings: next_settings },
        });
        assert_eq!(out, EnvelopeOutcome::Updated);
        assert_eq!(dot.pill_state().notch_style, NotchStyle::Nook);
        assert!(!dot.is_visible(), "settings sync must not reveal a hidden native pill");
    }
}
