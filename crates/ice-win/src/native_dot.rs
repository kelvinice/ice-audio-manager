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
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_TOPMOST, KillTimer, MSG,
    PostMessageW, PostQuitMessage, RegisterClassExW, SET_WINDOW_POS_FLAGS, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSENDCHANGING, SWP_NOSIZE, SetTimer, SetWindowPos, ShowWindow,
    TranslateMessage, ULW_ALPHA, UpdateLayeredWindow, WM_APP, WM_CLOSE, WM_DESTROY, WM_TIMER, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{Result, w};

use crate::monitors::MonitorInfo;

const WM_DOT_CMD: u32 = WM_APP + 1;
const ACTIVITY_TIMER_ID: usize = 0x4943_4531;
const LOGICAL_TOP_OFFSET: i32 = 6;

const IDLE_W: i32 = 88;
const IDLE_H: i32 = 16;
const VOLUME_W: i32 = 192;
const VOLUME_H: i32 = 30;
const MIC_W: i32 = 156;
const MIC_H: i32 = 30;
const DEVICE_W: i32 = 184;
const DEVICE_H: i32 = 30;

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

    /// Neutral Windows 11 dark-grey `Fluent` theme.
    #[must_use]
    pub const fn fluent() -> Self {
        Self {
            bg: Rgba::new(32, 32, 32, 224),
            surface: Rgba::new(44, 44, 44, 235),
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

    /// Resolves one of the four bundled themes.
    #[must_use]
    pub const fn from_builtin(theme: BuiltinTheme) -> Self {
        match theme {
            BuiltinTheme::Ice => Self::ice(),
            BuiltinTheme::Midnight => Self::midnight(),
            BuiltinTheme::Fluent => Self::fluent(),
            BuiltinTheme::Minimal => Self::minimal(),
        }
    }

    /// Resolves a bundled theme name (`ice`, `midnight`, `fluent`, `minimal`), case-insensitively.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "ice" => Some(Self::ice()),
            "midnight" => Some(Self::midnight()),
            "fluent" => Some(Self::fluent()),
            "minimal" => Some(Self::minimal()),
            _ => None,
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
    pub mic_muted: bool,
    pub active_device: Option<String>,
    pub spatial_active: bool,
    pub mode: PillMode,
    pub theme: NativePillTheme,
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
            mic_muted: false,
            active_device: None,
            spatial_active: false,
            mode: PillMode::Idle,
            theme,
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
        self.volume = snapshot.volume.map(|v| VolumeState { percent: v.percent.min(100), muted: v.muted });
        self.mic_muted = snapshot.mic_muted.unwrap_or(false);
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
            EngineEvent::Notice { .. } => {}
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
}

struct SharedState {
    hwnd_raw: AtomicIsize,
    visible: AtomicBool,
    closing: AtomicBool,
    runtime: Mutex<RuntimeState>,
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
            }),
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

    /// Shows the subtle idle dot/mini-pill at top-center (or muted-mic badge if `mic_muted`).
    pub fn show_idle(&self) {
        {
            let mut rt = self.shared.lock();
            rt.pulse_accent_override = None;
            rt.pill.theme = rt.base_theme;
            rt.pill.mode = PillMode::Idle;
            rt.desired_visible = true;
        }
        self.shared.post_signal(ThreadSignal::Idle);
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

    /// Returns whether the native pill window is currently visible.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.shared.visible.load(Ordering::Acquire)
    }

    /// Updates the active theme used by subsequent and current native pill renders.
    pub fn set_theme(&self, theme: NativePillTheme) {
        let should_repaint = {
            let mut rt = self.shared.lock();
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
            rt.pill = state.clone();
            rt.desired_visible = true;
        }
        self.shared.post_signal(ThreadSignal::Sync { restart_timer });
    }

    /// Synchronizes state from an [`EngineSnapshot`] without stealing focus or revealing expired activities.
    pub fn apply_snapshot(&self, snapshot: &EngineSnapshot) {
        let repaint_idle = {
            let mut rt = self.shared.lock();
            let prev_mic = rt.pill.mic_muted;
            rt.pill.apply_snapshot(snapshot);
            rt.desired_visible && matches!(rt.pill.mode, PillMode::Idle) && rt.pill.mic_muted != prev_mic
        };
        if repaint_idle {
            self.shared.post_signal(ThreadSignal::Sync { restart_timer: false });
        }
    }

    /// Applies an [`Envelope`] from `ice_core::engine` and reveals/updates the pill on activity events.
    pub fn apply_envelope(&self, envelope: &Envelope) -> EnvelopeOutcome {
        let (outcome, reveal) = {
            let mut rt = self.shared.lock();
            rt.pulse_accent_override = None;
            rt.pill.theme = rt.base_theme;
            let outcome = rt.pill.apply_envelope(envelope);
            let is_activity = matches!(
                envelope.event,
                EngineEvent::VolumeChanged { .. }
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
        lpszClassName: class_name,
        ..Default::default()
    };
    // SAFETY: registering an immutable static window class name; duplicate registration is harmless.
    unsafe { RegisterClassExW(&raw const wc) };

    let ex_style = WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_TRANSPARENT;
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
                (rt.pill.clone(), rt.monitor.clone(), rt.desired_visible)
            };
            if still_visible {
                let _ = paint_pill(hwnd, &active_mon, &pill);
                reveal_topmost_noactivate(hwnd);
                shared.visible.store(true, Ordering::Release);
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

const fn logical_dimensions(mode: PillMode) -> (i32, i32) {
    match mode {
        PillMode::Idle => (IDLE_W, IDLE_H),
        PillMode::Volume { .. } => (VOLUME_W, VOLUME_H),
        PillMode::MicMute { .. } => (MIC_W, MIC_H),
        PillMode::DeviceEvent { .. } => (DEVICE_W, DEVICE_H),
    }
}

fn paint_pill(hwnd: HWND, monitor: &MonitorInfo, state: &PillState) -> Result<()> {
    let (logical_w, logical_h) = logical_dimensions(state.mode);
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

    let radius = (lh - 2.0) * 0.5;
    let cx = lw * 0.5;
    let cy = lh * 0.5;
    let half_w = (lw - 2.0) * 0.5;
    let half_h = (lh - 2.0) * 0.5;

    let (bg_r, bg_g, bg_b, bg_a) = state.theme.bg.as_f32_rgba();
    let (surf_r, surf_g, surf_b, surf_a) = state.theme.surface.as_f32_rgba();
    let (fg_r, fg_g, fg_b, _) = state.theme.fg.as_f32_rgba();
    let (acc_r, acc_g, acc_b, _) = state.theme.accent.as_f32_rgba();
    let (ok_r, ok_g, ok_b, _) = state.theme.ok.as_f32_rgba();
    let (dng_r, dng_g, dng_b, _) = state.theme.danger.as_f32_rgba();

    let (rim_color, rim_strength) = match state.mode {
        PillMode::Idle if state.mic_muted => (state.theme.danger, 0.78),
        PillMode::Idle => (state.theme.rim, 0.40),
        PillMode::Volume { muted: true, .. } => (state.theme.danger, 0.78),
        PillMode::MicMute { muted: true } | PillMode::DeviceEvent { is_removal: true, .. } => {
            (state.theme.danger, 0.85)
        }
        PillMode::MicMute { muted: false } => (state.theme.ok, 0.75),
        PillMode::Volume { muted: false, .. } | PillMode::DeviceEvent { is_removal: false, .. } => {
            (state.theme.rim, 0.70)
        }
    };
    let (rim_r, rim_g, rim_b, rim_a) = rim_color.as_f32_rgba();

    let label_buf: String;
    let mut label_str = "";
    let mut label_x = 0.0f32;
    let mut label_w = 0.0f32;
    let mut label_color = (fg_r, fg_g, fg_b);
    let mut bar_bounds: Option<(f32, f32, f32, bool)> = None;

    match state.mode {
        PillMode::Idle => {}
        PillMode::Volume { percent, muted } => {
            label_buf = if muted { "MUTE".to_string() } else { format!("{}%", percent.min(100)) };
            label_str = &label_buf;
            label_x = 141.0;
            label_w = 40.0;
            label_color = if muted { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
            bar_bounds = Some((34.0, 134.0, f32::from(percent.min(100)) / 100.0, muted));
        }
        PillMode::MicMute { muted } => {
            label_str = if muted { "MIC OFF" } else { "MIC ON" };
            label_x = 42.0;
            label_w = 100.0;
            label_color = if muted { (dng_r, dng_g, dng_b) } else { (ok_r, ok_g, ok_b) };
        }
        PillMode::DeviceEvent { is_removal, .. } => {
            if is_removal {
                label_str = "OFF";
                label_color = (dng_r, dng_g, dng_b);
                bar_bounds = Some((34.0, 126.0, 0.0, true));
            } else if let Some(v) = state.volume {
                label_buf = if v.muted { "MUTE".to_string() } else { format!("{}%", v.percent.min(100)) };
                label_str = &label_buf;
                label_color = if v.muted { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
                bar_bounds = Some((34.0, 126.0, f32::from(v.percent.min(100)) / 100.0, v.muted));
            } else {
                label_str = "ON";
                label_color = (acc_r, acc_g, acc_b);
                bar_bounds = Some((34.0, 126.0, 1.0, false));
            }
            label_x = 132.0;
            label_w = 40.0;
        }
    }

    for py in 0..h {
        let ly = (py as f32 + 0.5) / scale;
        for px in 0..w {
            let lx = (px as f32 + 0.5) / scale;
            let sdf = sd_round_box(lx, ly, cx, cy, half_w, half_h, radius);
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

            match state.mode {
                PillMode::Idle => {
                    let dot_d = sd_circle(lx, ly, cx, cy, 2.3);
                    let dot_cov = smooth_cov(dot_d, aa);
                    if dot_cov > 0.0 {
                        let (dr, dg, db) = if state.mic_muted { (dng_r, dng_g, dng_b) } else { (acc_r, acc_g, acc_b) };
                        blend_over(&mut cr, &mut cg, &mut cb, dr, dg, db, dot_cov * 0.90);
                    }
                }
                PillMode::Volume { muted, .. } => {
                    let (icon_cov, is_danger_px) = speaker_icon_cov(lx, ly, 20.0, cy, muted, aa);
                    if icon_cov > 0.0 {
                        let (ir, ig, ib) =
                            if muted && is_danger_px { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
                        blend_over(&mut cr, &mut cg, &mut cb, ir, ig, ib, icon_cov);
                    }
                    if state.mic_muted {
                        let badge_cov = smooth_cov(sd_circle(lx, ly, lw - 8.0, 7.5, 2.0), aa);
                        blend_over(&mut cr, &mut cg, &mut cb, dng_r, dng_g, dng_b, badge_cov);
                    }
                }
                PillMode::MicMute { muted } => {
                    let (icon_cov, is_slash) = mic_icon_cov(lx, ly, 23.0, cy, muted, aa);
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
                        headphones_icon_cov(lx, ly, 20.0, cy, is_removal, aa)
                    } else {
                        speaker_icon_cov(lx, ly, 20.0, cy, is_removal, aa)
                    };
                    if icon_cov > 0.0 {
                        let (ir, ig, ib) = if is_danger_px { (dng_r, dng_g, dng_b) } else { (fg_r, fg_g, fg_b) };
                        blend_over(&mut cr, &mut cg, &mut cb, ir, ig, ib, icon_cov);
                    }
                    if spatial_active && !is_removal {
                        let sp_cov = smooth_cov(sd_circle(lx, ly, lw - 9.0, cy, 2.3), aa);
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

            if !label_str.is_empty() && lx >= label_x - 2.0 && lx <= label_x + label_w + 2.0 {
                let cov = text_cov_sdf(label_str, lx, ly, (label_x, label_w, cy, 10.5), aa);
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

fn text_cov_sdf(text: &str, lx: f32, ly: f32, (box_x, box_w, cy, box_h): (f32, f32, f32, f32), aa: f32) -> f32 {
    let char_count = text.chars().count();
    if char_count == 0 {
        return 0.0;
    }
    let glyph_h = box_h;
    let glyph_w = 6.2f32;
    let gap = 2.0f32;
    let total_w = glyph_w * char_count as f32 + gap * char_count.saturating_sub(1) as f32;
    let start_x = box_x + (box_w - total_w).max(0.0) * 0.5;
    let top_y = cy - glyph_h * 0.5;
    if ly < top_y - 1.5 || ly > top_y + glyph_h + 1.5 || lx < start_x - 1.5 || lx > start_x + total_w + 1.5 {
        return 0.0;
    }

    let stride = glyph_w + gap;
    let stroke = 0.75f32;
    let mut best_sd = 100.0f32;

    for (idx, ch) in text.chars().enumerate() {
        let gx0 = start_x + idx as f32 * stride;
        if lx < gx0 - 1.5 || lx > gx0 + glyph_w + 1.5 {
            continue;
        }
        if ch == ' ' {
            continue;
        }
        let nx = (lx - gx0) / glyph_w;
        let ny = (ly - top_y) / glyph_h;
        let norm_sd = glyph_unit_sd(ch, nx, ny);
        // Scale normalized distance back to logical px using glyph_w (~0.6 ratio) so strokes look even.
        let px_sd = norm_sd * glyph_w - stroke;
        best_sd = best_sd.min(px_sd);
    }
    smooth_cov(best_sd, aa)
}

fn glyph_unit_sd(ch: char, x: f32, y: f32) -> f32 {
    let segs: &[(f32, f32, f32, f32)] = match ch {
        '0' | 'O' => return sd_round_box(x, y, 0.5, 0.5, 0.42, 0.48, 0.34).abs(),
        '1' => &[(0.24, 0.22, 0.56, 0.02), (0.56, 0.02, 0.56, 0.98), (0.20, 0.98, 0.88, 0.98)],
        '2' => &[
            (0.10, 0.22, 0.30, 0.02),
            (0.30, 0.02, 0.76, 0.02),
            (0.76, 0.02, 0.92, 0.22),
            (0.92, 0.22, 0.86, 0.44),
            (0.86, 0.44, 0.08, 0.98),
            (0.08, 0.98, 0.94, 0.98),
        ],
        '3' => &[
            (0.10, 0.14, 0.28, 0.02),
            (0.28, 0.02, 0.76, 0.02),
            (0.76, 0.02, 0.92, 0.22),
            (0.92, 0.22, 0.72, 0.48),
            (0.42, 0.48, 0.78, 0.48),
            (0.78, 0.48, 0.94, 0.72),
            (0.94, 0.72, 0.76, 0.98),
            (0.76, 0.98, 0.24, 0.98),
            (0.24, 0.98, 0.08, 0.84),
        ],
        '4' => &[(0.74, 0.98, 0.74, 0.02), (0.74, 0.02, 0.06, 0.68), (0.06, 0.68, 0.96, 0.68)],
        '5' => &[
            (0.88, 0.02, 0.14, 0.02),
            (0.14, 0.02, 0.10, 0.46),
            (0.10, 0.46, 0.74, 0.46),
            (0.74, 0.46, 0.92, 0.64),
            (0.92, 0.64, 0.92, 0.80),
            (0.92, 0.80, 0.74, 0.98),
            (0.74, 0.98, 0.16, 0.98),
            (0.16, 0.98, 0.08, 0.86),
        ],
        '6' => &[
            (0.84, 0.10, 0.66, 0.02),
            (0.66, 0.02, 0.28, 0.02),
            (0.28, 0.02, 0.08, 0.24),
            (0.08, 0.24, 0.08, 0.80),
            (0.08, 0.80, 0.26, 0.98),
            (0.26, 0.98, 0.76, 0.98),
            (0.76, 0.98, 0.92, 0.80),
            (0.92, 0.80, 0.92, 0.62),
            (0.92, 0.62, 0.74, 0.46),
            (0.74, 0.46, 0.10, 0.46),
        ],
        '7' => &[(0.08, 0.02, 0.94, 0.02), (0.94, 0.02, 0.34, 0.98)],
        '8' => {
            let top = sd_round_box(x, y, 0.5, 0.25, 0.38, 0.23, 0.18).abs();
            let bot = sd_round_box(x, y, 0.5, 0.74, 0.42, 0.24, 0.20).abs();
            return top.min(bot);
        }
        '9' => &[
            (0.16, 0.90, 0.34, 0.98),
            (0.34, 0.98, 0.72, 0.98),
            (0.72, 0.98, 0.92, 0.76),
            (0.92, 0.76, 0.92, 0.20),
            (0.92, 0.20, 0.74, 0.02),
            (0.74, 0.02, 0.24, 0.02),
            (0.24, 0.02, 0.08, 0.20),
            (0.08, 0.20, 0.08, 0.38),
            (0.08, 0.38, 0.26, 0.54),
            (0.26, 0.54, 0.90, 0.54),
        ],
        '%' => {
            let slash = sd_segment(x, y, 0.86, 0.04, 0.14, 0.96);
            let top_ring = sd_circle(x, y, 0.24, 0.20, 0.14).abs();
            let bot_ring = sd_circle(x, y, 0.76, 0.80, 0.14).abs();
            return slash.min(top_ring).min(bot_ring);
        }
        'M' => {
            &[(0.08, 0.98, 0.08, 0.02), (0.08, 0.02, 0.50, 0.62), (0.50, 0.62, 0.92, 0.02), (0.92, 0.02, 0.92, 0.98)]
        }
        'U' => &[
            (0.08, 0.02, 0.08, 0.78),
            (0.08, 0.78, 0.26, 0.98),
            (0.26, 0.98, 0.74, 0.98),
            (0.74, 0.98, 0.92, 0.78),
            (0.92, 0.78, 0.92, 0.02),
        ],
        'T' => &[(0.06, 0.02, 0.94, 0.02), (0.50, 0.02, 0.50, 0.98)],
        'E' => {
            &[(0.88, 0.02, 0.14, 0.02), (0.14, 0.02, 0.14, 0.98), (0.14, 0.98, 0.88, 0.98), (0.14, 0.48, 0.74, 0.48)]
        }
        'I' => &[(0.22, 0.02, 0.78, 0.02), (0.50, 0.02, 0.50, 0.98), (0.22, 0.98, 0.78, 0.98)],
        'C' => &[
            (0.90, 0.18, 0.74, 0.02),
            (0.74, 0.02, 0.28, 0.02),
            (0.28, 0.02, 0.10, 0.22),
            (0.10, 0.22, 0.10, 0.78),
            (0.10, 0.78, 0.28, 0.98),
            (0.28, 0.98, 0.74, 0.98),
            (0.74, 0.98, 0.90, 0.82),
        ],
        'N' => &[(0.12, 0.98, 0.12, 0.02), (0.12, 0.02, 0.88, 0.98), (0.88, 0.98, 0.88, 0.02)],
        'F' => &[(0.88, 0.02, 0.14, 0.02), (0.14, 0.02, 0.14, 0.98), (0.14, 0.48, 0.74, 0.48)],
        _ => return 100.0,
    };

    let mut min_d = 100.0f32;
    for &(ax, ay, bx, by) in segs {
        min_d = min_d.min(sd_segment(x, y, ax, ay, bx, by));
    }
    min_d
}

#[cfg(test)]
mod tests {
    use ice_core::engine::Command;
    use ice_core::engine::fake::FakeEngine;

    use super::*;

    fn test_monitor(scale_factor: f64) -> MonitorInfo {
        MonitorInfo {
            id: "\\\\.\\DISPLAY1".into(),
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
        }
        let (lw, lh) = logical_dimensions(mode);
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
        assert_eq!(fluent.bg, Rgba::new(32, 32, 32, 224));

        let min = NativePillTheme::minimal();
        assert_eq!(min.bg, Rgba::new(18, 22, 29, 255));
        assert_eq!(min.rim, Rgba::new(0, 0, 0, 0));

        assert_eq!(NativePillTheme::from_name("MIDNIGHT"), Some(mid));
        assert_eq!(NativePillTheme::from_name("unknown"), None);
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
        assert!(dot.is_visible());

        dot.hide();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while dot.is_visible() {
            assert!(std::time::Instant::now() < deadline, "timed out waiting for dot hide");
            thread::sleep(Duration::from_millis(5));
        }

        drop(engine);
        sub.join();
    }
}
