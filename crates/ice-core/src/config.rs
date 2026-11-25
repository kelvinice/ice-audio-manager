//! User configuration, `config.toml` schema v4 (D-04, D-18).
//!
//! Location: `%APPDATA%\IceAudioManager\config.toml` (chosen by the app). Every field has a default, so a
//! missing or partial file is valid. Unknown future `schema_version`s are rejected rather than
//! misread, and a bad file is backed up and replaced by defaults (code-practices §7).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::appearance::Appearance;
use crate::device::{DeviceState, EndpointInfo, Flow};
use crate::engine::{
    BindingView, Command, DevicePrefsView, PerformanceProfile, RoleSet, SavedDevice, SettingsView, SpatialMode,
    TogglePairView,
};
use crate::hotkey::Chord;
use crate::lyrics::{LYRICS_CACHE_MAX_MB, LyricsConfig};
use crate::media::MusicServices;
use crate::placement::{NotchAnchor, NotchPlacement};
use crate::resolve::{DeviceRef, MatchMode, resolve};

/// v2: performance profile and pinned monitor. v3: `show_disconnected`. v4: saved notch appearance.
/// v5: `appearance.nowPlaying` (how the now-playing pill behaves). v6: `idle_linger_secs` (D-21.1 L1).
/// v7: `[music_services]` (D-22 switches) and `show_notch_hotkey` (D-19).
/// v8: `appearance.movingBackdrop` (D-25 ambient motion on the expanded card).
/// v9: `[lyrics]` (D-22 opt-in LRCLIB, local `.lrc`, Romaji subtitles, and `cache_max_mb`).
/// v10: `visualizer_spectrum` (D-26 local WASAPI loopback spectrum bars, default `true`; off = peak-meter-only).
/// v11: `appearance.notchStyle` (`"nook"` default flush screen-edge docked style vs `"floating"` pill).
/// v12: `appearance.hibernate` + `appearance.hibernateHideFromCapture` (D-27 plain notch for meetings; off / on).
/// v13: `appearance.hibernateLook` (D-27 `"plain"` notch or `"hidden"` while hibernating).
/// v14: `[[device]] lyrics_delay_ms` (per-output lyric delay; absent = automatic, see [`auto_lyrics_delay_ms`]).
/// v15: `notch_anchor` (D-20 `"top_left"` | `"top_center"` default | `"top_right"`).
/// v16: bottom `notch_anchor` values (Floating) and `notch_placement` (`"drag_snap"` default | `"fixed"`), D-20.
/// v17: `notch_x_ratio` continuous horizontal position (`0.0..=1.0`, absent = preset anchor) and bottom Edge Nook.
pub const SCHEMA_VERSION: u32 = 17;

/// Allowed per-device lyric delay, in ms (negative = lyrics earlier).
pub const LYRICS_DELAY_MS: std::ops::RangeInclusive<i32> = -1000..=2000;
/// Automatic lyric delay for a Bluetooth output. An estimate, not a measurement: Bluetooth audio (A2DP/LE) typically
/// reaches the ear 150–300 ms after Windows plays it, and Windows exposes no API for that link's delay.
pub const BLUETOOTH_LYRICS_DELAY_MS: i32 = 200;

/// The lyric delay an output gets when the user hasn't set one: [`BLUETOOTH_LYRICS_DELAY_MS`] for a Bluetooth device
/// (its device path starts with `BTHENUM` or `BTHLE`), 0 for everything else. A USB Bluetooth dongle enumerates as USB,
/// so it stays at 0; the user can set its delay by hand.
#[must_use]
pub fn auto_lyrics_delay_ms(endpoint: &EndpointInfo) -> i32 {
    // Hardware keys look like `BTHENUM\{…}_VID&…`; devnodes like `{1}.BTHENUM\…`.
    let key = endpoint.hardware_key.as_deref().unwrap_or_default().to_ascii_uppercase();
    let node = endpoint.devnode.as_deref().unwrap_or_default().to_ascii_uppercase();
    let on_bluetooth = key.starts_with("BTHENUM")
        || key.starts_with("BTHLE")
        || node.contains(".BTHENUM\\")
        || node.contains(".BTHLE");
    if on_bluetooth { BLUETOOTH_LYRICS_DELAY_MS } else { 0 }
}

/// Default seconds the notch WebView stays awake at idle before `Balanced` hands over to the native pill
/// (`Instant` never does; `Lite` has no WebView).
pub const IDLE_LINGER_SECS_DEFAULT: u16 = 15;
/// Allowed idle linger, in seconds.
pub const IDLE_LINGER_SECS: std::ops::RangeInclusive<u16> = 5..=600;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub appearance: Appearance,
    pub performance_profile: PerformanceProfile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_monitor: Option<String>,
    /// D-20: where on the pinned monitor's top or bottom edge the notch sits.
    pub notch_anchor: NotchAnchor,
    /// D-20: optional continuous horizontal offset (`0.0..=1.0`) between Left and Right; `None` = preset anchor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notch_x_ratio: Option<f32>,
    /// D-20: drag the notch to move it along the edge, or only move it from the tray and Settings.
    pub notch_placement: NotchPlacement,
    /// Roles set by switches that don't specify their own.
    pub default_roles: RoleSet,
    /// Device hotkeys: press → switch to that device.
    #[serde(rename = "binding", skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<Binding>,
    /// Toggle pairs, e.g. Speakers ↔ Arctis (the owner's main workflow, D-16).
    #[serde(rename = "toggle_pair", skip_serializing_if = "Vec::is_empty")]
    pub toggle_pairs: Vec<TogglePair>,
    /// Per-device behaviour applied on switch (spatial sound, D-17).
    #[serde(rename = "device", skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<DevicePrefs>,
    pub favourites: Favourites,
    pub knob: KnobGestures,
    /// List unplugged endpoints (e.g. an empty headphone jack) greyed out, like Windows' "Show disconnected
    /// devices". Never as switch targets, and never `NotPresent` ghosts (D-16).
    pub show_disconnected: bool,
    /// D-21.1 L1: how long the notch stays awake at idle before `Balanced` suspends it ([`IDLE_LINGER_SECS`]).
    pub idle_linger_secs: u16,
    /// D-22: launch buttons in the notch, one switch per service.
    pub music_services: MusicServices,
    /// D-22: synced lyrics, Privacy opt-in, Romaji subtitles, and storage cache cap.
    pub lyrics: LyricsConfig,
    /// D-26: local WASAPI loopback 14-band spectrum analyser (off = peak-meter-only fallback).
    pub visualizer_spectrum: bool,
    /// D-19: opens (or closes) the expanded notch; in Lite, the tray quick-switch menu. Unbound by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_notch_hotkey: Option<Chord>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            appearance: Appearance::default(),
            performance_profile: PerformanceProfile::default(),
            pinned_monitor: None,
            notch_anchor: NotchAnchor::default(),
            notch_x_ratio: None,
            notch_placement: NotchPlacement::default(),
            default_roles: RoleSet::Default,
            bindings: Vec::new(),
            toggle_pairs: Vec::new(),
            devices: Vec::new(),
            favourites: Favourites::default(),
            knob: KnobGestures::default(),
            show_disconnected: false,
            idle_linger_secs: IDLE_LINGER_SECS_DEFAULT,
            music_services: MusicServices::default(),
            lyrics: LyricsConfig::default(),
            visualizer_spectrum: true,
            show_notch_hotkey: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: String,
    pub hotkey: Option<Chord>,
    pub device: DeviceRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<RoleSet>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TogglePair {
    pub id: String,
    pub hotkey: Option<Chord>,
    /// Chosen when a third device has taken over (D-16 amendment).
    pub primary: DeviceRef,
    pub secondary: DeviceRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<RoleSet>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevicePrefs {
    pub device: DeviceRef,
    /// Applied after a confirmed switch to this device (read-before-write, D-17).
    #[serde(default)]
    pub spatial: SpatialMode,
    /// Shown instead of Windows' name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    /// Lyrics shown this many ms later while this device plays (its output delay). `None` = automatic
    /// ([`auto_lyrics_delay_ms`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lyrics_delay_ms: Option<i32>,
}

impl DevicePrefs {
    /// An entry that changes nothing is left out of the file.
    fn is_default(&self) -> bool {
        self.spatial == SpatialMode::Keep && self.alias.is_none() && !self.hidden && self.lyrics_delay_ms.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Favourites {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub playback: Vec<DeviceRef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recording: Vec<DeviceRef>,
    /// Hotkey for "next favourite playback device".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_hotkey: Option<Chord>,
}

/// F31: modified knob turns (`RegisterHotKey` on Mod+VolumeUp/Down; plain turns stay Windows').
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct KnobGestures {
    pub enabled: bool,
    /// Ctrl+turn: browse favourites in the notch, commit after the knob rests (Alt+Tab style).
    pub cycle_device_modifier: Option<String>,
    /// Shift+turn: fine volume steps.
    pub fine_volume_modifier: Option<String>,
    pub fine_step_percent: u8,
    /// Alt+turn: microphone volume.
    pub mic_volume_modifier: Option<String>,
    /// Quiet time after the last detent before a device browse commits.
    pub commit_after_ms: u32,
}

impl Default for KnobGestures {
    fn default() -> Self {
        Self {
            enabled: false,
            cycle_device_modifier: Some("Ctrl".into()),
            fine_volume_modifier: Some("Shift".into()),
            fine_step_percent: 1,
            mic_volume_modifier: Some("Alt".into()),
            commit_after_ms: 400,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(String),
    /// Written by a newer app version; never guess at its meaning.
    TooNew(u32),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "config I/O error: {e}"),
            Self::Parse(e) => write!(f, "config is invalid: {e}"),
            Self::TooNew(v) => write!(f, "config schema {v} is newer than this app supports ({SCHEMA_VERSION})"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Parses TOML, applying defaults and migrations.
    ///
    /// # Errors
    /// Invalid TOML, unknown fields, bad hotkeys, or a schema newer than [`SCHEMA_VERSION`].
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        /// Read first and leniently: a newer file's new fields must report `TooNew`, not an unknown-field error.
        #[derive(Deserialize)]
        struct Version {
            #[serde(default)]
            schema_version: u32,
        }
        if let Ok(Version { schema_version }) = toml::from_str::<Version>(text)
            && schema_version > SCHEMA_VERSION
        {
            return Err(ConfigError::TooNew(schema_version));
        }
        let cfg: Self = toml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        if !IDLE_LINGER_SECS.contains(&cfg.idle_linger_secs) {
            return Err(ConfigError::Parse(format!(
                "idle_linger_secs = {} is outside {}–{} seconds",
                cfg.idle_linger_secs,
                IDLE_LINGER_SECS.start(),
                IDLE_LINGER_SECS.end()
            )));
        }
        if !LYRICS_CACHE_MAX_MB.contains(&cfg.lyrics.cache_max_mb) {
            return Err(ConfigError::Parse(format!(
                "lyrics.cache_max_mb = {} is outside {}–{} MB",
                cfg.lyrics.cache_max_mb,
                LYRICS_CACHE_MAX_MB.start(),
                LYRICS_CACHE_MAX_MB.end()
            )));
        }
        if let Some(ratio) = cfg.notch_x_ratio
            && !(ratio.is_finite() && (0.0..=1.0).contains(&ratio))
        {
            return Err(ConfigError::Parse(format!("notch_x_ratio = {ratio} must be finite and within 0.0–1.0")));
        }
        match cfg.schema_version {
            v if v > SCHEMA_VERSION => Err(ConfigError::TooNew(v)),
            // v0/v1 gain the default profile and an initially unassigned monitor pin, v0–v2 gain
            // `show_disconnected = false`, v0–v4 gain `appearance.nowPlaying = "while_playing"`, and v0–v5 gain
            // `idle_linger_secs = 15`, v0–v6 gain both music services on, v0–v7 gain
            // `appearance.movingBackdrop = true`, v0–v8 gain `[lyrics]` (off by default, 50 MB cap),
            // v0–v9 gain `visualizer_spectrum = true`, v0–v10 gain `appearance.notchStyle = "nook"`, and v0–v11 gain
            // `appearance.hibernate = false` with `hibernateHideFromCapture = true`, v0–v12 gain
            // `appearance.hibernateLook = "plain"`, v0–v13 devices an automatic lyrics delay (no
            // `lyrics_delay_ms`), v0–v14 gain `notch_anchor = "top_center"`, v0–v15 gain
            // `notch_placement = "drag_snap"`, and v0–v16 gain `notch_x_ratio = None`, all via serde defaults.
            _ => Ok(Self { schema_version: SCHEMA_VERSION, ..cfg }),
        }
    }

    /// Serialises to commented-free, stable TOML.
    ///
    /// # Panics
    /// Never in practice: every field is TOML-representable.
    #[must_use]
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("config is always serialisable")
    }

    /// Carries the live session state (D-27 hibernate) over to a config re-read from disk, so a hot reload or an
    /// external edit never hibernates or wakes the notch; only the hibernate command does.
    pub const fn keep_session_state(&mut self, live: &Self) {
        self.appearance.hibernate = live.appearance.hibernate;
    }

    /// How much later lyrics show while `endpoint` plays: the user's value, else [`auto_lyrics_delay_ms`].
    #[must_use]
    pub fn lyrics_delay_for(&self, endpoint: &EndpointInfo) -> i32 {
        self.prefs_for(endpoint).and_then(|p| p.lyrics_delay_ms).unwrap_or_else(|| auto_lyrics_delay_ms(endpoint))
    }

    /// The device preferences entry for a device, if any (matched by the resolver, so it survives re-links).
    #[must_use]
    pub fn prefs_for(&self, endpoint: &crate::device::EndpointInfo) -> Option<&DevicePrefs> {
        let one = std::slice::from_ref(endpoint);
        self.devices.iter().find(|p| crate::resolve::resolve(&p.device, one).target().is_some())
    }

    /// Every saved device reference: bindings, both sides of toggle pairs, device preferences, favourites.
    pub fn device_refs_mut(&mut self) -> impl Iterator<Item = &mut DeviceRef> {
        self.bindings
            .iter_mut()
            .map(|b| &mut b.device)
            .chain(self.toggle_pairs.iter_mut().flat_map(|p| [&mut p.primary, &mut p.secondary]))
            .chain(self.devices.iter_mut().map(|d| &mut d.device))
            .chain(self.favourites.playback.iter_mut())
            .chain(self.favourites.recording.iter_mut())
    }

    /// After a confirmed switch to a re-created endpoint, points *every* reference to the old endpoint at the
    /// new one (review ENG-10), keeping each reference's display name and match mode. Returns how many changed.
    pub fn relink_all(&mut self, old_endpoint_id: &str, target: &crate::device::EndpointInfo) -> usize {
        let mut n = 0;
        for r in self.device_refs_mut() {
            if r.endpoint_id.eq_ignore_ascii_case(old_endpoint_id) && r.flow == target.flow {
                *r = r.relinked_to(target);
                n += 1;
            }
        }
        n
    }

    /// The settings as the UI shows them, resolved against the live endpoints.
    #[must_use]
    pub fn view(&self, endpoints: &[EndpointInfo]) -> SettingsView {
        let saved = |r: &DeviceRef| SavedDevice {
            saved_id: r.endpoint_id.clone(),
            name: r.display_name.clone(),
            flow: r.flow,
            device_id: resolve(r, endpoints).target().map(|e| e.id.clone()),
        };
        SettingsView {
            appearance: self.appearance,
            default_roles: self.default_roles,
            performance_profile: self.performance_profile,
            pinned_monitor: self.pinned_monitor.clone(),
            notch_anchor: self.notch_anchor,
            notch_x_ratio: self.notch_x_ratio,
            notch_placement: self.notch_placement,
            toggle_pairs: self
                .toggle_pairs
                .iter()
                .map(|p| TogglePairView {
                    id: p.id.clone(),
                    hotkey: p.hotkey.map(|c| c.to_string()),
                    primary: saved(&p.primary),
                    secondary: saved(&p.secondary),
                    roles: p.roles,
                })
                .collect(),
            bindings: self
                .bindings
                .iter()
                .map(|b| BindingView {
                    id: b.id.clone(),
                    hotkey: b.hotkey.map(|c| c.to_string()),
                    device: saved(&b.device),
                    roles: b.roles,
                })
                .collect(),
            favourites_playback: self.favourites.playback.iter().map(saved).collect(),
            favourites_recording: self.favourites.recording.iter().map(saved).collect(),
            devices: self
                .devices
                .iter()
                .map(|d| DevicePrefsView {
                    device: saved(&d.device),
                    spatial: d.spatial,
                    alias: d.alias.clone(),
                    hidden: d.hidden,
                    lyrics_delay_ms: d.lyrics_delay_ms,
                })
                .collect(),
            knob_gestures: self.knob.enabled,
            show_disconnected: self.show_disconnected,
            idle_linger_secs: self.idle_linger_secs,
            music_services: self.music_services,
            lyrics: self.lyrics,
            visualizer_spectrum: self.visualizer_spectrum,
            show_notch_hotkey: self.show_notch_hotkey.map(|c| c.to_string()),
        }
    }

    /// Applies a settings command. `Ok(true)` = changed, `Ok(false)` = not a settings command or nothing to change.
    ///
    /// # Errors
    /// A user-facing reason (device not connected, invalid or duplicate hotkey, …); the config is left unchanged.
    pub fn apply_setting(&mut self, cmd: &Command, endpoints: &[EndpointInfo]) -> Result<bool, String> {
        let mut next = self.clone();
        match cmd {
            Command::SetTogglePair { pair_id, primary_id, secondary_id, hotkey, roles } => {
                let (a, b) = (live(endpoints, primary_id)?, live(endpoints, secondary_id)?);
                if a.id == b.id {
                    return Err("a toggle pair needs two different devices".into());
                }
                if a.flow != b.flow {
                    return Err("a toggle pair can't mix playback and recording devices".into());
                }
                let hotkey = parse_hotkey(hotkey.as_deref())?;
                next.toggle_pairs.retain(|p| p.id != *pair_id);
                next.toggle_pairs.push(TogglePair {
                    id: pair_id.clone(),
                    hotkey,
                    primary: DeviceRef::from_endpoint(a, MatchMode::SameModel),
                    secondary: DeviceRef::from_endpoint(b, MatchMode::SameModel),
                    roles: *roles,
                });
                next.check_hotkeys_unique()?;
            }
            Command::RemoveTogglePair { pair_id } => next.toggle_pairs.retain(|p| p.id != *pair_id),
            Command::SetBinding { binding_id, device_id, hotkey, roles } => {
                let device = DeviceRef::from_endpoint(live(endpoints, device_id)?, MatchMode::SameModel);
                let hotkey = parse_hotkey(hotkey.as_deref())?;
                next.bindings.retain(|b| b.id != *binding_id);
                next.bindings.push(Binding { id: binding_id.clone(), hotkey, device, roles: *roles });
                next.check_hotkeys_unique()?;
            }
            Command::RemoveBinding { binding_id } => next.bindings.retain(|b| b.id != *binding_id),
            Command::SetFavourites { flow, device_ids } => {
                let saved = match flow {
                    Flow::Render => &mut next.favourites.playback,
                    Flow::Capture => &mut next.favourites.recording,
                };
                *saved = ordered_favourites(*flow, device_ids, saved, endpoints)?;
            }
            Command::SetDevicePrefs { device_id, spatial, alias, hidden } => {
                let e = live(endpoints, device_id)?;
                if spatial.is_some_and(|m| m != SpatialMode::Keep) && e.flow != Flow::Render {
                    return Err("spatial sound only applies to playback devices".into());
                }
                let i = prefs_entry(&mut next.devices, e);
                let prefs = &mut next.devices[i];
                if let Some(m) = spatial {
                    prefs.spatial = *m;
                }
                if let Some(a) = alias {
                    prefs.alias = Some(a.trim().to_string()).filter(|a| !a.is_empty());
                }
                if let Some(h) = hidden {
                    prefs.hidden = *h;
                }
                drop_if_default(&mut next.devices, i);
            }
            Command::SetLyricsDelay { device_id, delay_ms } => {
                let e = live(endpoints, device_id)?;
                if e.flow != Flow::Render {
                    return Err("the lyrics delay only applies to playback devices".into());
                }
                if delay_ms.is_some_and(|d| !LYRICS_DELAY_MS.contains(&d)) {
                    return Err(format!(
                        "the lyrics delay must be {} to {} ms",
                        LYRICS_DELAY_MS.start(),
                        LYRICS_DELAY_MS.end()
                    ));
                }
                let i = prefs_entry(&mut next.devices, e);
                next.devices[i].lyrics_delay_ms = *delay_ms;
                drop_if_default(&mut next.devices, i);
            }
            Command::SetPerformanceProfile { profile } => next.performance_profile = *profile,
            Command::SetNotchAnchor { anchor } => {
                next.notch_anchor = *anchor;
                next.notch_x_ratio = None;
            }
            Command::SetNotchPosition { anchor, x_ratio } => {
                if x_ratio.is_some_and(|r| !(r.is_finite() && (0.0..=1.0).contains(&r))) {
                    return Err("Notch horizontal ratio must be between 0.0 and 1.0.".into());
                }
                next.notch_anchor = *anchor;
                next.notch_x_ratio = *x_ratio;
            }
            Command::SetNotchPlacement { placement } => next.notch_placement = *placement,
            Command::SetPinnedMonitor { display_path } => {
                if display_path.trim().is_empty() || display_path.chars().any(char::is_control) {
                    return Err("Choose a connected monitor.".into());
                }
                next.pinned_monitor = Some(display_path.clone());
            }
            Command::SetKnobGestures { enabled } => next.knob.enabled = *enabled,
            Command::SetTheme { theme } => next.appearance.theme = *theme,
            Command::SetPreset { preset } => next.appearance.preset = *preset,
            Command::SetIdleStyle { style } => next.appearance.idle_style = *style,
            Command::SetNotchStyle { style } => next.appearance.notch_style = *style,
            Command::SetNowPlayingPill { mode } => next.appearance.now_playing = *mode,
            Command::SetMovingBackdrop { enabled } => next.appearance.moving_backdrop = *enabled,
            Command::SetHibernate { enabled } => next.appearance.hibernate = *enabled,
            Command::SetHibernateLook { look } => next.appearance.hibernate_look = *look,
            Command::SetHibernateHideFromCapture { enabled } => next.appearance.hibernate_hide_from_capture = *enabled,
            Command::SetShowDisconnected { enabled } => next.show_disconnected = *enabled,
            Command::SetIdleLinger { secs } => {
                if !IDLE_LINGER_SECS.contains(secs) {
                    return Err(format!(
                        "Choose between {} seconds and {} minutes.",
                        IDLE_LINGER_SECS.start(),
                        IDLE_LINGER_SECS.end() / 60
                    ));
                }
                next.idle_linger_secs = *secs;
            }
            Command::SetMusicService { service, enabled } => next.music_services.set(*service, *enabled),
            Command::SetLyricsEnabled { enabled } => next.lyrics.enabled = *enabled,
            Command::SetLyricsOnlineLookup { enabled } => next.lyrics.online_lookup = *enabled,
            Command::SetLyricsDisplay { show_on_pill, show_on_card, word_highlight, romaji } => {
                if let Some(v) = show_on_pill {
                    next.lyrics.show_on_pill = *v;
                }
                if let Some(v) = show_on_card {
                    next.lyrics.show_on_card = *v;
                }
                if let Some(v) = word_highlight {
                    next.lyrics.word_highlight = *v;
                }
                if let Some(v) = romaji {
                    next.lyrics.romaji = *v;
                }
            }
            Command::SetLyricsCacheMaxMb { max_mb } => {
                if !LYRICS_CACHE_MAX_MB.contains(max_mb) {
                    return Err(format!(
                        "Choose a cache limit between {} and {} MB.",
                        LYRICS_CACHE_MAX_MB.start(),
                        LYRICS_CACHE_MAX_MB.end()
                    ));
                }
                next.lyrics.cache_max_mb = *max_mb;
            }
            Command::DisableAllNetworkFeatures => next.lyrics.online_lookup = false,
            Command::SetVisualizerSpectrum { enabled } => next.visualizer_spectrum = *enabled,
            Command::SetShowNotchHotkey { hotkey } => {
                next.show_notch_hotkey = parse_hotkey(hotkey.as_deref())?;
                next.check_hotkeys_unique()?;
            }
            Command::SetDefaultRoles { roles } => next.default_roles = *roles,
            _ => return Ok(false),
        }
        let changed = next != *self;
        *self = next;
        Ok(changed)
    }

    /// Two actions on one chord would make one of them silently dead.
    fn check_hotkeys_unique(&self) -> Result<(), String> {
        let mut seen: Vec<(Chord, String)> = Vec::new();
        let named = self
            .toggle_pairs
            .iter()
            .filter_map(|p| p.hotkey.map(|c| (c, format!("toggle pair \"{}\"", p.id))))
            .chain(self.bindings.iter().filter_map(|b| b.hotkey.map(|c| (c, format!("hotkey \"{}\"", b.id)))))
            .chain(self.favourites.cycle_hotkey.map(|c| (c, "the favourites hotkey".to_string())))
            .chain(self.show_notch_hotkey.map(|c| (c, "the show-notch hotkey".to_string())));
        for (chord, owner) in named {
            if let Some((_, first)) = seen.iter().find(|(c, _)| *c == chord) {
                return Err(format!("{chord} is already used by {first}"));
            }
            seen.push((chord, owner));
        }
        Ok(())
    }
}

fn ordered_favourites(
    flow: Flow,
    ids: &[String],
    saved: &[DeviceRef],
    endpoints: &[EndpointInfo],
) -> Result<Vec<DeviceRef>, String> {
    let mut refs = Vec::<DeviceRef>::with_capacity(ids.len());
    for id in ids {
        if refs.iter().any(|r| r.endpoint_id.eq_ignore_ascii_case(id)) {
            continue;
        }
        if let Some(existing) = saved.iter().find(|r| r.endpoint_id.eq_ignore_ascii_case(id)) {
            refs.push(existing.clone());
        } else {
            let endpoint = live(endpoints, id)?;
            if endpoint.flow != flow {
                return Err(format!("{} is not a {flow:?} device", display(endpoint)));
            }
            refs.push(DeviceRef::from_endpoint(endpoint, MatchMode::SameModel));
        }
    }
    Ok(refs)
}

/// An Active endpoint by ID (settings are created from devices the user can see).
fn live<'a>(endpoints: &'a [EndpointInfo], id: &str) -> Result<&'a EndpointInfo, String> {
    endpoints
        .iter()
        .find(|e| e.id.eq_ignore_ascii_case(id) && e.state == DeviceState::Active)
        .ok_or_else(|| format!("device {id} is not connected"))
}

/// Index of `e`'s device preferences, added with defaults if it has none yet (matched by the resolver).
fn prefs_entry(devices: &mut Vec<DevicePrefs>, e: &EndpointInfo) -> usize {
    let one = std::slice::from_ref(e);
    if let Some(i) = devices.iter().position(|p| resolve(&p.device, one).target().is_some()) {
        return i;
    }
    devices.push(DevicePrefs {
        device: DeviceRef::from_endpoint(e, MatchMode::SameModel),
        spatial: SpatialMode::Keep,
        alias: None,
        hidden: false,
        lyrics_delay_ms: None,
    });
    devices.len() - 1
}

/// An entry back at all defaults is noise in the file.
fn drop_if_default(devices: &mut Vec<DevicePrefs>, i: usize) {
    if devices[i].is_default() {
        devices.remove(i);
    }
}

fn display(e: &EndpointInfo) -> String {
    crate::resolve::normalize_name(e.friendly_name.as_deref().unwrap_or(&e.id))
}

/// `None` or blank = no hotkey.
fn parse_hotkey(s: Option<&str>) -> Result<Option<Chord>, String> {
    match s.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => s.parse::<Chord>().map(Some).map_err(|e| format!("hotkey \"{s}\": {e}")),
    }
}

/// Loads and saves the config file.
pub struct ConfigStore {
    path: PathBuf,
}

/// What [`ConfigStore::load`] did.
#[derive(Debug)]
pub enum Loaded {
    Ok(Config),
    /// No file yet: defaults.
    Missing(Config),
    /// The file was unusable; defaults are used. Tell the user. `backup` is where the broken file went,
    /// or `None` if it couldn't be moved (it's then left in place untouched).
    Replaced {
        config: Config,
        backup: Option<PathBuf>,
        error: String,
    },
}

impl Loaded {
    #[must_use]
    pub fn config(&self) -> &Config {
        match self {
            Self::Ok(c) | Self::Missing(c) | Self::Replaced { config: c, .. } => c,
        }
    }
}

impl ConfigStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Never fails: a broken file is backed up and replaced by defaults (the app keeps running).
    /// A file from a *newer* build isn't broken: it stays in place untouched (`backup: None`) and this run uses
    /// defaults. Saves then refuse to overwrite it, because it doesn't parse here. That keeps the owner's
    /// settings safe when an older build (e.g. an agent's un-rebased branch) starts after a schema bump.
    ///
    /// Only the app's start calls this, so it is where session state resets: hibernate (D-27) is always off at
    /// start, whatever the file says (owner, 2026-11-25).
    #[must_use]
    pub fn load(&self) -> Loaded {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Loaded::Missing(Config::default()),
            Err(e) => return self.replace_broken(&e.to_string()),
        };
        match Config::from_toml(&text) {
            Ok(mut c) => {
                c.appearance.hibernate = false;
                Loaded::Ok(c)
            }
            Err(e @ ConfigError::TooNew(_)) => {
                Loaded::Replaced { config: Config::default(), backup: None, error: e.to_string() }
            }
            Err(e) => self.replace_broken(&e.to_string()),
        }
    }

    fn replace_broken(&self, error: &str) -> Loaded {
        // Millisecond stamp: quick restarts mustn't collide (review ENG-04).
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
        let target = self.path.with_extension(format!("invalid-{stamp}.toml"));
        let backup = std::fs::rename(&self.path, &target).ok().map(|()| target);
        Loaded::Replaced { config: Config::default(), backup, error: error.to_string() }
    }

    /// Non-destructive read for hot reload: never renames the file (the user may be mid-edit).
    /// `Ok(None)` if the file doesn't exist.
    ///
    /// # Errors
    /// I/O or parse errors, so the caller can keep the last valid config and tell the user.
    pub fn read(&self) -> Result<Option<Config>, ConfigError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Config::from_toml(&text).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    /// The file's current text, for detecting external edits before an automatic save. `None` if missing.
    ///
    /// # Errors
    /// I/O errors other than "not found".
    pub fn read_text(&self) -> std::io::Result<Option<String>> {
        match std::fs::read_to_string(&self.path) {
            Ok(t) => Ok(Some(t)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Atomic save: write and flush a temp file next to the target, then rename over it. Windows briefly
    /// locks files that editors or antivirus have open, so the rename is retried; the temp file is
    /// removed if it never lands (review ENG-04). Returns the text written.
    ///
    /// # Errors
    /// I/O failures (folder not writable, disk full, file locked for longer than the retries).
    pub fn save(&self, cfg: &Config) -> Result<String, ConfigError> {
        use std::io::Write as _;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(ConfigError::Io)?;
        }
        let text = cfg.to_toml();
        let tmp = self.path.with_extension("toml.tmp");
        let written = std::fs::File::create(&tmp).and_then(|mut f| {
            f.write_all(text.as_bytes())?;
            f.sync_all()
        });
        if let Err(e) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(ConfigError::Io(e));
        }
        let mut last = None;
        for attempt in 0..5 {
            match std::fs::rename(&tmp, &self.path) {
                Ok(()) => return Ok(text),
                Err(e) => last = Some(e),
            }
            std::thread::sleep(std::time::Duration::from_millis(20 * (attempt + 1)));
        }
        let _ = std::fs::remove_file(&tmp);
        Err(ConfigError::Io(last.unwrap_or_else(|| std::io::Error::other("rename failed"))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::Flow;
    use crate::resolve::MatchMode;

    fn dref(id: &str, name: &str) -> DeviceRef {
        DeviceRef {
            endpoint_id: id.into(),
            stable_id: None,
            flow: Flow::Render,
            display_name: name.into(),
            endpoint_name: Some("Headphones".into()),
            interface_name: Some("Arctis Nova 7".into()),
            hardware_key: Some(r"USB\VID_1038&PID_22A1&MI_00".into()),
            container_id: Some("{8EC256AB}".into()),
            form_factor: Some(3),
            match_mode: MatchMode::SameModel,
        }
    }

    fn owner_config() -> Config {
        Config {
            toggle_pairs: vec![TogglePair {
                id: "main".into(),
                hotkey: Some("Ctrl+Alt+F12".parse().unwrap()),
                primary: dref("arctis", "Headphones (Arctis Nova 7)"),
                secondary: dref("realtek", "Speakers (Realtek(R) Audio)"),
                roles: None,
            }],
            devices: vec![DevicePrefs {
                device: dref("arctis", "Headphones (Arctis Nova 7)"),
                spatial: SpatialMode::DolbyAtmosHeadphones,
                alias: None,
                hidden: false,
                lyrics_delay_ms: None,
            }],
            ..Config::default()
        }
    }

    #[test]
    fn v1_migrates_with_default_placement_and_preserves_user_settings() {
        let mut config = Config::from_toml("schema_version = 1\ndefault_roles = 'both'").unwrap();
        assert_eq!(config.default_roles, RoleSet::Both);
        assert_eq!(config.view(&[]).performance_profile, PerformanceProfile::Instant);
        assert_eq!(config.pinned_monitor, None);
        let pin = Command::SetPinnedMonitor { display_path: "DISPLAY#test".into() };
        let profile = Command::SetPerformanceProfile { profile: PerformanceProfile::Lite };
        assert_eq!(config.apply_setting(&pin, &[]), Ok(true));
        assert_eq!(config.apply_setting(&profile, &[]), Ok(true));
        assert_eq!(config.view(&[]).pinned_monitor.as_deref(), Some("DISPLAY#test"));
        assert_eq!(config.view(&[]).performance_profile, PerformanceProfile::Lite);
        assert_eq!(Config::from_toml(&config.to_toml()).unwrap(), config);
    }

    #[test]
    fn v2_migrates_with_disconnected_devices_hidden() {
        let v2 = "schema_version = 2\nperformance_profile = 'balanced'\npinned_monitor = 'DISPLAY#test'";
        let mut config = Config::from_toml(v2).unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(config.performance_profile, PerformanceProfile::Balanced);
        assert!(!config.view(&[]).show_disconnected);
        let show = Command::SetShowDisconnected { enabled: true };
        assert_eq!(config.apply_setting(&show, &[]), Ok(true));
        assert_eq!(config.apply_setting(&show, &[]), Ok(false), "already on");
        assert!(config.view(&[]).show_disconnected);
        let text = config.to_toml();
        assert!(text.contains("show_disconnected = true"), "{text}");
        assert_eq!(Config::from_toml(&text).unwrap(), config);
    }

    #[test]
    fn v3_migrates_with_default_appearance_and_saves_choices() {
        use crate::appearance::{Appearance, IdleStyle, SpringPreset, ThemeKind};
        let mut config = Config::from_toml("schema_version = 3\nshow_disconnected = true").unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert!(config.show_disconnected, "v3 settings are kept");
        assert_eq!(config.view(&[]).appearance, Appearance::default());
        assert_eq!(Appearance::default().preset, SpringPreset::Bouncy, "owner default");
        assert_eq!(Appearance::default().idle_style, IdleStyle::MiniPill, "Q-10 default");
        for cmd in [
            Command::SetTheme { theme: ThemeKind::Midnight },
            Command::SetPreset { preset: SpringPreset::Smooth },
            Command::SetIdleStyle { style: IdleStyle::Dot },
        ] {
            assert_eq!(config.apply_setting(&cmd, &[]), Ok(true));
        }
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert_eq!(
            saved.view(&[]).appearance,
            Appearance {
                theme: ThemeKind::Midnight,
                preset: SpringPreset::Smooth,
                idle_style: IdleStyle::Dot,
                ..Appearance::default()
            }
        );
    }

    #[test]
    fn show_notch_hotkey_is_unbound_by_default_parsed_and_unique() {
        let mut config = Config::from_toml("schema_version = 6").unwrap();
        assert_eq!(config.view(&[]).show_notch_hotkey, None, "unbound by default (D-19)");

        let bind = |s: Option<&str>| Command::SetShowNotchHotkey { hotkey: s.map(str::to_string) };
        assert_eq!(config.apply_setting(&bind(Some("Ctrl+Alt+N")), &[]), Ok(true));
        let shown = config.view(&[]).show_notch_hotkey.expect("bound");
        assert_eq!(Config::from_toml(&config.to_toml()).unwrap().view(&[]).show_notch_hotkey, Some(shown.clone()));

        assert!(config.apply_setting(&bind(Some("not a key")), &[]).is_err(), "bad chord");
        config.favourites.cycle_hotkey = config.show_notch_hotkey;
        assert!(config.check_hotkeys_unique().is_err(), "one chord, two actions");
        config.favourites.cycle_hotkey = None;

        assert_eq!(config.apply_setting(&bind(None), &[]), Ok(true), "unbind");
        assert_eq!(config.view(&[]).show_notch_hotkey, None);
    }

    #[test]
    fn v6_migrates_with_both_music_services_on_and_saves_each_switch() {
        use crate::media::{MusicServiceId, MusicServices};
        let mut config = Config::from_toml("schema_version = 6\nidle_linger_secs = 30").unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(config.idle_linger_secs, 30, "v6 settings are kept");
        assert_eq!(config.view(&[]).music_services, MusicServices { spotify: true, youtube_music: true });

        let off = Command::SetMusicService { service: MusicServiceId::YoutubeMusic, enabled: false };
        assert_eq!(config.apply_setting(&off, &[]), Ok(true));
        assert_eq!(config.apply_setting(&off, &[]), Ok(false), "already off");
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert_eq!(saved.view(&[]).music_services, MusicServices { spotify: true, youtube_music: false });
    }

    #[test]
    fn v5_migrates_with_the_default_idle_linger_and_validates_its_range() {
        let mut config = Config::from_toml("schema_version = 5\nperformance_profile = 'balanced'").unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(config.view(&[]).idle_linger_secs, IDLE_LINGER_SECS_DEFAULT);

        assert_eq!(config.apply_setting(&Command::SetIdleLinger { secs: 60 }, &[]), Ok(true));
        assert!(config.apply_setting(&Command::SetIdleLinger { secs: 2 }, &[]).is_err(), "below the range");
        assert!(config.apply_setting(&Command::SetIdleLinger { secs: 3600 }, &[]).is_err(), "above the range");
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert_eq!(saved.idle_linger_secs, 60);

        // A hand-edited out-of-range value is a bad file: the last valid config stays (like any parse error).
        assert!(matches!(Config::from_toml("idle_linger_secs = 1"), Err(ConfigError::Parse(_))));
    }

    #[test]
    fn v4_migrates_with_while_playing_default_and_saves_the_choice() {
        use crate::appearance::{IdleStyle, NowPlayingPill};
        let v4 = "schema_version = 4\n[appearance]\nidleStyle = 'dot'";
        let mut config = Config::from_toml(v4).unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(config.view(&[]).appearance.idle_style, IdleStyle::Dot, "v4 appearance is kept");
        assert_eq!(config.view(&[]).appearance.now_playing, NowPlayingPill::WhilePlaying, "owner default");

        let set = Command::SetNowPlayingPill { mode: NowPlayingPill::Brief };
        assert_eq!(config.apply_setting(&set, &[]), Ok(true));
        assert_eq!(config.apply_setting(&set, &[]), Ok(false), "already set");
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert_eq!(saved.view(&[]).appearance.now_playing, NowPlayingPill::Brief);
    }

    #[test]
    fn v7_migrates_with_moving_backdrop_on_and_saves_the_choice() {
        use crate::appearance::ThemeKind;
        let v7 = "schema_version = 7\n[appearance]\ntheme = 'midnight'";
        let mut config = Config::from_toml(v7).unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(config.view(&[]).appearance.theme, ThemeKind::Midnight);
        assert!(config.view(&[]).appearance.moving_backdrop, "D-25 default is on");

        let set = Command::SetMovingBackdrop { enabled: false };
        assert_eq!(config.apply_setting(&set, &[]), Ok(true));
        assert_eq!(config.apply_setting(&set, &[]), Ok(false), "already off");
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert!(!saved.view(&[]).appearance.moving_backdrop);
    }

    #[test]
    fn v8_migrates_with_lyrics_off_and_validates_cache_max_mb() {
        let v8 = "schema_version = 8\n[appearance]\nmovingBackdrop = true";
        let mut config = Config::from_toml(v8).unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        let view = config.view(&[]).lyrics;
        assert!(!view.enabled && !view.online_lookup, "D-22 privacy: lyrics and network off by default");
        assert_eq!(view.cache_max_mb, crate::lyrics::LYRICS_CACHE_MAX_MB_DEFAULT);

        assert_eq!(config.apply_setting(&Command::SetLyricsEnabled { enabled: true }, &[]), Ok(true));
        assert_eq!(config.apply_setting(&Command::SetLyricsOnlineLookup { enabled: true }, &[]), Ok(true));
        assert_eq!(config.apply_setting(&Command::SetLyricsCacheMaxMb { max_mb: 100 }, &[]), Ok(true));
        assert!(config.apply_setting(&Command::SetLyricsCacheMaxMb { max_mb: 1 }, &[]).is_err());
        assert!(config.apply_setting(&Command::SetLyricsCacheMaxMb { max_mb: 999 }, &[]).is_err());
        assert_eq!(config.apply_setting(&Command::DisableAllNetworkFeatures, &[]), Ok(true));
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert!(saved.lyrics.enabled);
        assert!(!saved.lyrics.online_lookup, "master kill switch turns off online lookup");
        assert_eq!(saved.lyrics.cache_max_mb, 100);
        assert!(matches!(Config::from_toml("[lyrics]\ncacheMaxMb = 2"), Err(ConfigError::Parse(_))));
    }

    #[test]
    fn v9_migrates_with_visualizer_spectrum_on_and_saves_privacy_switch() {
        let v9 = "schema_version = 9\n[lyrics]\nenabled = true";
        let mut config = Config::from_toml(v9).unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert!(config.lyrics.enabled, "v9 lyrics settings are preserved");
        assert!(config.view(&[]).visualizer_spectrum, "D-26 default is local spectrum bars on");

        let off = Command::SetVisualizerSpectrum { enabled: false };
        assert_eq!(config.apply_setting(&off, &[]), Ok(true));
        assert_eq!(config.apply_setting(&off, &[]), Ok(false), "already off");
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert!(!saved.view(&[]).visualizer_spectrum);
    }

    #[test]
    fn v10_migrates_with_nook_notch_style_and_saves_the_choice() {
        use crate::appearance::NotchStyle;
        let v10 = "schema_version = 10\n[appearance]\ntheme = 'ice'";
        let mut config = Config::from_toml(v10).unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(config.view(&[]).appearance.notch_style, NotchStyle::Nook);

        let set = Command::SetNotchStyle { style: NotchStyle::Floating };
        assert_eq!(config.apply_setting(&set, &[]), Ok(true));
        assert_eq!(config.apply_setting(&set, &[]), Ok(false), "already floating");
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert_eq!(saved.view(&[]).appearance.notch_style, NotchStyle::Floating);
    }

    #[test]
    fn v11_migrates_awake_with_capture_hiding_on_and_saves_hibernate() {
        let v11 = "schema_version = 11\n[appearance]\nnotchStyle = 'nook'";
        let mut config = Config::from_toml(v11).unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        let appearance = config.view(&[]).appearance;
        assert!(!appearance.hibernate, "D-27: off by default");
        assert!(appearance.hibernate_hide_from_capture, "D-27: hide from capture is on by default");
        assert!(!appearance.exclude_from_capture(), "only while hibernating");

        let set = Command::SetHibernate { enabled: true };
        assert_eq!(config.apply_setting(&set, &[]), Ok(true));
        assert_eq!(config.apply_setting(&set, &[]), Ok(false), "already hibernating");
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert!(saved.view(&[]).appearance.hibernate, "written to the file (a restart resets it: `ConfigStore::load`)");
        assert!(saved.view(&[]).appearance.exclude_from_capture());

        let show = Command::SetHibernateHideFromCapture { enabled: false };
        assert_eq!(config.apply_setting(&show, &[]), Ok(true));
        assert!(!config.view(&[]).appearance.exclude_from_capture());
    }

    #[test]
    fn v12_migrates_with_a_plain_hibernate_look_and_hidden_hides_the_idle_notch() {
        use crate::appearance::{HibernateLook, IdleStyle};
        let v12 = "schema_version = 12\n[appearance]\nidleStyle = 'dot'\nhibernate = true";
        let mut config = Config::from_toml(v12).unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        let appearance = config.view(&[]).appearance;
        assert_eq!(appearance.hibernate_look, HibernateLook::Plain, "D-27: the plain notch by default");
        assert_eq!(appearance.effective_idle_style(), IdleStyle::Dot, "the plain notch keeps the idle shape");

        let hide = Command::SetHibernateLook { look: HibernateLook::Hidden };
        assert_eq!(config.apply_setting(&hide, &[]), Ok(true));
        assert_eq!(config.apply_setting(&hide, &[]), Ok(false), "already hidden");
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert_eq!(saved.appearance.hibernate_look, HibernateLook::Hidden);
        assert_eq!(saved.appearance.effective_idle_style(), IdleStyle::Hidden, "hidden while hibernating");

        let wake = Command::SetHibernate { enabled: false };
        assert_eq!(config.apply_setting(&wake, &[]), Ok(true));
        assert_eq!(config.appearance.effective_idle_style(), IdleStyle::Dot, "awake: the chosen idle style again");
    }

    #[test]
    fn lyrics_delay_is_automatic_for_bluetooth_and_settable_per_device() {
        let mut buds = live_ep("buds", "Headphones (soundcore Liberty 5)", Flow::Render, 3);
        buds.hardware_key = Some(r"BTHENUM\{0000110B-0000-1000-8000-00805F9B34FB}_VID&000102B0_PID&0000".into());
        let mut dongle = live_ep("dongle", "Headset Earphone (BTD 700)", Flow::Render, 5);
        dongle.hardware_key = Some(r"USB\VID_3542&PID_3001&MI_02".into());
        let mut le = live_ep("le", "Headphones (LE)", Flow::Render, 3);
        le.hardware_key = None;
        le.devnode = Some(r"{1}.BTHLEDevice\{0000184e-0000-1000-8000-00805f9b34fb}_Dev_VID".into());
        let mic = live_ep("mic", "Microphone (Realtek(R) Audio)", Flow::Capture, 4);
        assert_eq!(auto_lyrics_delay_ms(&buds), BLUETOOTH_LYRICS_DELAY_MS, "classic Bluetooth");
        assert_eq!(auto_lyrics_delay_ms(&le), BLUETOOTH_LYRICS_DELAY_MS, "Bluetooth LE");
        assert_eq!(auto_lyrics_delay_ms(&dongle), 0, "a USB dongle enumerates as USB");

        let devs = vec![buds.clone(), dongle.clone(), mic];
        let mut cfg = Config::default();
        let set = |id: &str, delay_ms| Command::SetLyricsDelay { device_id: id.into(), delay_ms };
        assert_eq!(cfg.apply_setting(&set("dongle", Some(120)), &devs), Ok(true));
        assert_eq!(cfg.lyrics_delay_for(&dongle), 120);
        assert_eq!(cfg.view(&devs).devices[0].lyrics_delay_ms, Some(120));
        assert_eq!(cfg.apply_setting(&set("buds", Some(-150)), &devs), Ok(true));
        assert_eq!(cfg.lyrics_delay_for(&buds), -150, "the user's value beats automatic");
        assert!(cfg.apply_setting(&set("buds", Some(5_000)), &devs).is_err(), "out of range");
        assert!(cfg.apply_setting(&set("mic", Some(100)), &devs).is_err(), "playback only");
        let saved = Config::from_toml(&cfg.to_toml()).unwrap();
        assert_eq!(saved.lyrics_delay_for(&dongle), 120, "saved");
        // Back to automatic: the entry has nothing left to say and leaves the file.
        assert_eq!(cfg.apply_setting(&set("dongle", None), &devs), Ok(true));
        assert_eq!(cfg.apply_setting(&set("buds", None), &devs), Ok(true));
        assert!(cfg.devices.is_empty());
        assert_eq!(cfg.lyrics_delay_for(&buds), BLUETOOTH_LYRICS_DELAY_MS);
        // v13 files migrate with every device on automatic.
        let v13 = "schema_version = 13\n[[device]]\nspatial = 'keep'\nalias = 'Desk'\n[device.device]\nendpoint_id = 'buds'\nflow = 'render'\ndisplay_name = 'Buds'";
        let migrated = Config::from_toml(v13).unwrap();
        assert_eq!(migrated.schema_version, SCHEMA_VERSION);
        assert_eq!(migrated.devices[0].lyrics_delay_ms, None);
    }

    #[test]
    fn v14_migrates_to_the_top_centre_and_saves_the_anchor() {
        use crate::placement::NotchAnchor;
        let mut config = Config::from_toml("schema_version = 14\nperformance_profile = 'instant'").unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(config.view(&[]).notch_anchor, NotchAnchor::TopCenter, "where the notch always was");
        let left = Command::SetNotchAnchor { anchor: NotchAnchor::TopLeft };
        assert_eq!(config.apply_setting(&left, &[]), Ok(true));
        assert_eq!(config.apply_setting(&left, &[]), Ok(false), "already there");
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert_eq!(saved.notch_anchor, NotchAnchor::TopLeft);
        assert!(config.to_toml().contains("notch_anchor = \"top_left\""), "{}", config.to_toml());
    }

    #[test]
    fn v15_migrates_to_drag_and_snap_and_saves_bottom_anchors() {
        use crate::placement::{NotchAnchor, NotchPlacement};
        let mut config = Config::from_toml("schema_version = 15\nnotch_anchor = 'top_right'").unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(
            config.view(&[]).notch_placement,
            NotchPlacement::DragSnap,
            "D-20 owner ruling: drag & snap by default"
        );
        assert_eq!(config.notch_anchor, NotchAnchor::TopRight, "kept");
        let bottom = Command::SetNotchAnchor { anchor: NotchAnchor::BottomLeft };
        let fixed = Command::SetNotchPlacement { placement: NotchPlacement::Fixed };
        assert_eq!(config.apply_setting(&bottom, &[]), Ok(true));
        assert_eq!(config.apply_setting(&fixed, &[]), Ok(true));
        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert_eq!((saved.notch_anchor, saved.notch_placement), (NotchAnchor::BottomLeft, NotchPlacement::Fixed));
    }

    #[test]
    fn v16_migrates_with_none_x_ratio_and_saves_custom_horizontal_position() {
        use crate::placement::NotchAnchor;
        let mut config = Config::from_toml("schema_version = 16\nnotch_anchor = 'bottom_center'").unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(config.view(&[]).notch_x_ratio, None);

        let custom = Command::SetNotchPosition { anchor: NotchAnchor::BottomLeft, x_ratio: Some(0.28) };
        assert_eq!(config.apply_setting(&custom, &[]), Ok(true));
        assert_eq!(config.apply_setting(&custom, &[]), Ok(false));

        let saved = Config::from_toml(&config.to_toml()).unwrap();
        assert_eq!(saved.notch_anchor, NotchAnchor::BottomLeft);
        assert_eq!(saved.notch_x_ratio, Some(0.28));

        assert!(
            config
                .apply_setting(&Command::SetNotchPosition { anchor: NotchAnchor::TopCenter, x_ratio: Some(1.2) }, &[])
                .is_err()
        );
        assert!(
            config
                .apply_setting(
                    &Command::SetNotchPosition { anchor: NotchAnchor::TopCenter, x_ratio: Some(f32::NAN) },
                    &[]
                )
                .is_err()
        );
        assert!(Config::from_toml("schema_version = 17\nnotch_x_ratio = -0.1").is_err());

        assert_eq!(config.apply_setting(&Command::SetNotchAnchor { anchor: NotchAnchor::TopRight }, &[]), Ok(true));
        assert_eq!(config.notch_x_ratio, None);
    }

    #[test]
    fn every_start_is_awake_and_a_reload_keeps_the_live_hibernate() {
        let dir = std::env::temp_dir().join(format!("ice-cfg-hibernate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = ConfigStore::new(dir.join("config.toml"));
        let mut hibernating = Config::default();
        hibernating.appearance.hibernate = true;
        store.save(&hibernating).unwrap();
        match store.load() {
            Loaded::Ok(c) => assert!(!c.appearance.hibernate, "owner: the app never starts hibernating"),
            other => panic!("expected Ok, got {other:?}"),
        }
        // Hot reload re-reads the file: the session value wins both ways.
        let mut reread = store.read().unwrap().unwrap();
        assert!(reread.appearance.hibernate);
        reread.keep_session_state(&Config::default());
        assert!(!reread.appearance.hibernate, "a stale file doesn't hibernate an awake notch");
        let mut awake_file = Config::default();
        awake_file.keep_session_state(&hibernating);
        assert!(awake_file.appearance.hibernate, "an external edit doesn't wake a hibernating notch");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_favourites_keeps_a_disconnected_one_in_its_place() {
        let mut cfg = Config::default();
        let set = |ids: &[&str]| Command::SetFavourites {
            flow: Flow::Render,
            device_ids: ids.iter().map(|s| (*s).to_string()).collect(),
        };
        assert_eq!(cfg.apply_setting(&set(&["arctis", "realtek"]), &owner_devices()), Ok(true));
        // The Arctis is switched off. The tray re-sends the saved list (saved IDs) plus a new device.
        let without_arctis: Vec<EndpointInfo> = owner_devices().into_iter().filter(|e| e.id != "arctis").collect();
        let mut more = without_arctis.clone();
        more.push(live_ep("lg", "LG FULL HD (NVIDIA High Definition Audio)", Flow::Render, 9));
        assert_eq!(cfg.apply_setting(&set(&["arctis", "realtek", "lg"]), &more), Ok(true));
        let ids: Vec<&str> = cfg.favourites.playback.iter().map(|r| r.endpoint_id.as_str()).collect();
        assert_eq!(ids, ["arctis", "realtek", "lg"], "the offline Arctis keeps its place");
        let view = cfg.view(&without_arctis);
        assert_eq!(view.favourites_playback[0].saved_id, "arctis");
        assert_eq!(view.favourites_playback[0].device_id, None);
        // An ID that is neither saved nor connected is still refused.
        assert!(cfg.apply_setting(&set(&["arctis", "ghost"]), &more).is_err());
    }

    #[test]
    fn empty_file_is_all_defaults() {
        assert_eq!(Config::from_toml("").unwrap(), Config::default());
    }

    #[test]
    fn round_trips_the_owner_setup() {
        let cfg = owner_config();
        let text = cfg.to_toml();
        assert!(text.contains("[[toggle_pair]]") && text.contains(r#"hotkey = "Ctrl+Alt+F12""#), "{text}");
        assert!(text.contains(r#"spatial = "dolbyAtmosHeadphones""#), "{text}");
        assert_eq!(Config::from_toml(&text).unwrap(), cfg);
    }

    #[test]
    fn rejects_newer_schema_bad_hotkeys_and_typos() {
        assert!(matches!(Config::from_toml("schema_version = 99"), Err(ConfigError::TooNew(99))));
        let bad_key = "[[binding]]\nid='x'\nhotkey='VolumeUp'\n[binding.device]\nendpoint_id='a'\nflow='render'\ndisplay_name='A'";
        assert!(matches!(Config::from_toml(bad_key), Err(ConfigError::Parse(_))));
        assert!(matches!(Config::from_toml("defualt_roles = 'both'"), Err(ConfigError::Parse(_))));
    }

    #[test]
    fn store_saves_atomically_and_backs_up_broken_files() {
        let dir = std::env::temp_dir().join(format!("ice-cfg-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = ConfigStore::new(dir.join("config.toml"));
        assert!(matches!(store.load(), Loaded::Missing(_)));
        store.save(&owner_config()).unwrap();
        assert!(matches!(store.load(), Loaded::Ok(c) if c == owner_config()));
        std::fs::write(store.path(), "this is [ not toml").unwrap();
        match store.load() {
            Loaded::Replaced { config, backup, .. } => {
                assert_eq!(config, Config::default());
                assert!(backup.is_some_and(|b| b.exists()) && !store.path().exists());
            }
            other => panic!("expected Replaced, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_from_a_newer_build_is_left_in_place() {
        let dir = std::env::temp_dir().join(format!("ice-cfg-newer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = ConfigStore::new(dir.join("config.toml"));
        std::fs::create_dir_all(&dir).unwrap();
        let newer = format!("schema_version = {}\nsome_future_field = true\n", SCHEMA_VERSION + 1);
        std::fs::write(store.path(), &newer).unwrap();
        match store.load() {
            Loaded::Replaced { config, backup: None, error } => {
                assert_eq!(config, Config::default());
                assert!(error.contains("newer"), "{error}");
            }
            other => panic!("expected Replaced without a backup, got {other:?}"),
        }
        assert_eq!(std::fs::read_to_string(store.path()).unwrap(), newer, "untouched");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "no backup file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hot_reload_read_never_renames_a_broken_file() {
        let dir = std::env::temp_dir().join(format!("ice-cfg-read-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = ConfigStore::new(dir.join("config.toml"));
        assert!(matches!(store.read(), Ok(None)));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(store.path(), "half-typed = [").unwrap();
        assert!(matches!(store.read(), Err(ConfigError::Parse(_))));
        assert!(store.path().exists(), "the user's file must stay where it is");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn relink_all_updates_every_reference_to_the_old_endpoint() {
        let mut cfg = owner_config();
        cfg.favourites.playback.push(dref("arctis", "Headphones (Arctis Nova 7)"));
        let new = crate::device::EndpointInfo {
            id: "arctis-new".into(),
            flow: Flow::Render,
            state: crate::device::DeviceState::Active,
            friendly_name: Some("Headphones (2- Arctis Nova 7)".into()),
            endpoint_name: Some("Headphones".into()),
            interface_name: Some("2- Arctis Nova 7".into()),
            devnode: None,
            hardware_key: Some(r"USB\VID_1038&PID_22A1&MI_00".into()),
            container_id: Some("{8EC256AB}".into()),
            form_factor: Some(3),
            stable_id: None,
            default_for: vec![],
        };
        // Pair primary + device prefs + favourite point at "arctis" (matched case-insensitively); the speakers don't.
        assert_eq!(cfg.relink_all("ARCTIS", &new), 3);
        assert_eq!(cfg.toggle_pairs[0].primary.endpoint_id, "arctis-new");
        assert_eq!(cfg.toggle_pairs[0].primary.display_name, "Headphones (Arctis Nova 7)");
        assert_eq!(cfg.toggle_pairs[0].primary.interface_name.as_deref(), Some("Arctis Nova 7"));
        assert_eq!(cfg.devices[0].device.endpoint_id, "arctis-new");
        assert_eq!(cfg.favourites.playback[0].endpoint_id, "arctis-new");
        assert_eq!(cfg.toggle_pairs[0].secondary.endpoint_id, "realtek");
    }

    fn live_ep(id: &str, name: &str, flow: Flow, ff: u32) -> EndpointInfo {
        EndpointInfo {
            id: id.into(),
            flow,
            state: DeviceState::Active,
            friendly_name: Some(name.into()),
            endpoint_name: name.split(" (").next().map(str::to_string),
            interface_name: None,
            devnode: None,
            hardware_key: Some(format!("HW\\{id}")),
            container_id: None,
            form_factor: Some(ff),
            stable_id: None,
            default_for: vec![],
        }
    }

    fn owner_devices() -> Vec<EndpointInfo> {
        vec![
            live_ep("arctis", "Headphones (Arctis Nova 7)", Flow::Render, 3),
            live_ep("realtek", "Speakers (Realtek(R) Audio)", Flow::Render, 1),
            live_ep("mic", "Microphone (Arctis Nova 7)", Flow::Capture, 4),
        ]
    }

    #[test]
    fn set_toggle_pair_from_live_devices_then_view_it() {
        let devs = owner_devices();
        let mut cfg = Config::default();
        let cmd = Command::SetTogglePair {
            pair_id: "main".into(),
            primary_id: "arctis".into(),
            secondary_id: "realtek".into(),
            hotkey: Some("ctrl+alt+f12".into()),
            roles: None,
        };
        assert_eq!(cfg.apply_setting(&cmd, &devs), Ok(true));
        assert_eq!(cfg.apply_setting(&cmd, &devs), Ok(false), "same edit twice changes nothing");
        let view = cfg.view(&devs);
        assert_eq!(view.toggle_pairs[0].hotkey.as_deref(), Some("Ctrl+Alt+F12"));
        assert_eq!(view.toggle_pairs[0].primary.name, "Headphones (Arctis Nova 7)");
        assert_eq!(view.toggle_pairs[0].primary.device_id.as_deref(), Some("arctis"));
        // Disconnected: still listed, just without a live device.
        let view = cfg.view(&devs[1..]);
        assert_eq!(view.toggle_pairs[0].primary.device_id, None);
    }

    #[test]
    fn rejects_bad_settings_without_changing_anything() {
        let devs = owner_devices();
        let mut cfg = Config::default();
        let pair = |id: &str, a: &str, b: &str, key: &str| Command::SetTogglePair {
            pair_id: id.into(),
            primary_id: a.into(),
            secondary_id: b.into(),
            hotkey: Some(key.into()),
            roles: None,
        };
        assert_eq!(cfg.apply_setting(&pair("main", "arctis", "realtek", "Ctrl+Alt+F12"), &devs), Ok(true));
        let before = cfg.clone();
        let err = |cfg: &mut Config, c: &Command| cfg.apply_setting(c, &devs).unwrap_err();
        assert!(err(&mut cfg, &pair("x", "arctis", "arctis", "F9")).contains("two different"));
        assert!(err(&mut cfg, &pair("x", "arctis", "mic", "F9")).contains("mix playback"));
        assert!(err(&mut cfg, &pair("x", "arctis", "gone", "F9")).contains("not connected"));
        assert!(err(&mut cfg, &pair("x", "arctis", "realtek", "VolumeUp")).contains("modifier"));
        assert!(err(&mut cfg, &pair("x", "arctis", "realtek", "Ctrl+Alt+F12")).contains("already used"));
        assert_eq!(cfg, before);
    }

    #[test]
    fn device_prefs_patch_and_clean_up() {
        let devs = owner_devices();
        let mut cfg = Config::default();
        let set = |spatial, alias: Option<&str>, hidden| Command::SetDevicePrefs {
            device_id: "realtek".into(),
            spatial,
            alias: alias.map(str::to_string),
            hidden,
        };
        assert_eq!(cfg.apply_setting(&set(None, Some(" Desk speakers "), None), &devs), Ok(true));
        assert_eq!(cfg.devices[0].alias.as_deref(), Some("Desk speakers"));
        assert_eq!(cfg.apply_setting(&set(Some(SpatialMode::WindowsSonic), None, None), &devs), Ok(true));
        assert_eq!(cfg.devices.len(), 1, "patched in place");
        assert_eq!(cfg.devices[0].alias.as_deref(), Some("Desk speakers"), "untouched fields stay");
        assert_eq!(cfg.apply_setting(&set(Some(SpatialMode::Keep), Some(""), Some(false)), &devs), Ok(true));
        assert!(cfg.devices.is_empty(), "an all-default entry is removed");
        let mic_spatial = Command::SetDevicePrefs {
            device_id: "mic".into(),
            spatial: Some(SpatialMode::DolbyAtmosHeadphones),
            alias: None,
            hidden: None,
        };
        assert!(cfg.apply_setting(&mic_spatial, &devs).is_err());
    }

    #[test]
    fn favourites_knob_and_roles() {
        let devs = owner_devices();
        let mut cfg = Config::default();
        let favs = Command::SetFavourites {
            flow: Flow::Render,
            device_ids: vec!["realtek".into(), "arctis".into(), "realtek".into()],
        };
        assert_eq!(cfg.apply_setting(&favs, &devs), Ok(true));
        let names: Vec<_> = cfg.view(&devs).favourites_playback.into_iter().map(|d| d.name).collect();
        assert_eq!(names, ["Speakers (Realtek(R) Audio)", "Headphones (Arctis Nova 7)"], "ordered, de-duplicated");
        let wrong_flow = Command::SetFavourites { flow: Flow::Render, device_ids: vec!["mic".into()] };
        assert!(cfg.apply_setting(&wrong_flow, &devs).is_err());
        assert_eq!(cfg.apply_setting(&Command::SetKnobGestures { enabled: true }, &devs), Ok(true));
        assert_eq!(cfg.apply_setting(&Command::SetDefaultRoles { roles: RoleSet::Both }, &devs), Ok(true));
        assert!(cfg.view(&devs).knob_gestures);
        assert_eq!(cfg.apply_setting(&Command::ToggleMute, &devs), Ok(false), "not a settings command");
    }
}
