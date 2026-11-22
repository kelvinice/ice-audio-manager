//! The engine contract (plan.md "Parallel work", track A).
//!
//! Everything outside the engine talks to it through these types only:
//! - the tray, hotkeys and settings (Codex, `app/src-tauri`) send [`Command`]s;
//! - the notch UI (Gemini, `app/ui`) renders [`EngineSnapshot`] + [`Envelope`] deltas.
//!
//! Protocol (D-03, Codex's amendment): every delta carries a monotonic `revision`. A consumer subscribes
//! first, then reads the snapshot, applies only deltas with a newer revision, and re-reads the snapshot
//! on a gap. [`fake::FakeEngine`] implements the same contract for UI and shell work before the real
//! engine (`phase-1/engine`) merges.
//!
//! With the `ts` feature, every type derives `ts_rs::TS` so the app can export TypeScript bindings.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::device::{DeviceState, Flow, Role};
use crate::media::NowPlaying;

/// Which default roles a switch sets (D-05, F02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub enum RoleSet {
    /// Console + Multimedia (what Windows calls "default device").
    #[default]
    Default,
    /// Communications only ("default communication device").
    Communications,
    /// All three roles.
    Both,
}

impl RoleSet {
    #[must_use]
    pub fn roles(self) -> &'static [Role] {
        match self {
            Self::Default => &Role::DEFAULT,
            Self::Communications => &[Role::Communications],
            Self::Both => &Role::ALL,
        }
    }
}

/// D-15.1 notch WebView lifetime. Both WebView profiles suspend on idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub enum PerformanceProfile {
    #[default]
    Instant,
    Balanced,
    Lite,
}

/// Spatial sound choice for a device (D-17). `Keep` never touches Windows' setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub enum SpatialMode {
    #[default]
    Keep,
    Off,
    WindowsSonic,
    DolbyAtmosHeadphones,
    DolbyAtmosHomeTheater,
    DolbyAtmosSpeakers,
    DtsHeadphoneX,
    DtsXUltra,
    DtsXHomeTheater,
}

/// Transport control for the current media session (Spotify, browser, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub enum MediaAction {
    PlayPause,
    Play,
    Pause,
    Next,
    Previous,
}

/// A device as the UI sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct DeviceSummary {
    /// Endpoint ID. Use it for `SwitchTo`; bindings use `DeviceRef`s held by the engine.
    pub id: String,
    /// Display name without Windows' `N- ` re-install prefix, e.g. "Headphones (Arctis Nova 7)".
    pub name: String,
    pub flow: Flow,
    pub state: DeviceState,
    /// Default for Console + Multimedia.
    pub is_default: bool,
    /// Default for Communications.
    pub is_default_communications: bool,
    /// `EndpointFormFactor` (3 = headphones, 1 = speakers, 4 = microphone, 9 = digital/HDMI…), for the icon.
    pub form_factor: Option<u32>,
    /// Software device (Steam Streaming, Sonar, VB-Cable…): shown with a "virtual" badge.
    pub is_virtual: bool,
    /// Active spatial format, when known (`None` for capture devices or unknown).
    pub spatial: Option<SpatialMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct VolumeState {
    /// 0–100, rounded from Windows' scalar.
    pub percent: u8,
    pub muted: bool,
}

/// Full state. Fetch it on start, on window (re)creation, and on a revision gap.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct EngineSnapshot {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub revision: u64,
    /// Active playback devices (ghosts excluded, D-16).
    pub playback: Vec<DeviceSummary>,
    /// Active recording devices.
    pub recording: Vec<DeviceSummary>,
    /// Volume of the default playback device.
    pub volume: Option<VolumeState>,
    /// Mute state of the default communications microphone.
    pub mic_muted: Option<bool>,
    /// Volume (0–100) of the default communications microphone.
    pub mic_volume: Option<u8>,
    /// Problems that stay true until fixed (hotkey conflicts, config problems). A UI shows these after
    /// (re)subscribing, instead of relying on replayed events (review ENG-08).
    #[serde(default)]
    pub notices: Vec<Notice>,
    /// The user's settings, resolved against the live devices.
    #[serde(default)]
    pub settings: SettingsView,
    /// What's playing (Windows' current media session), `None` when nothing is.
    #[serde(default)]
    pub media: Option<NowPlaying>,
}

/// What the tray, hotkeys and UI ask the engine to do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Command {
    /// Switch to a live endpoint (tray menu, settings list).
    SwitchTo {
        device_id: String,
        roles: RoleSet,
    },
    /// Switch to a saved binding (hotkey, favourite). Resolved via its `DeviceRef` (D-16).
    SwitchBinding {
        binding_id: String,
    },
    /// Flip a toggle pair, e.g. Speakers ↔ Arctis (D-16).
    TogglePair {
        pair_id: String,
    },
    /// Next/previous favourite of a flow. `step` is +1 or −1.
    CycleFavourites {
        flow: Flow,
        step: i8,
    },
    /// Absolute volume of the default playback device.
    SetVolume {
        percent: u8,
    },
    /// Relative volume change in percent points (knob/scroll/hotkey).
    StepVolume {
        delta: i8,
    },
    ToggleMute,
    ToggleMicMute,
    /// Play/pause/next/previous on the current media session.
    Media {
        action: MediaAction,
    },
    SetSpatial {
        device_id: String,
        mode: SpatialMode,
    },
    /// Answer to [`Notice::ChooseDevice`]. `None` = cancel.
    ResolveChoice {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        request_id: u64,
        device_id: Option<String>,
    },
    /// Re-enumerate and emit a fresh snapshot (rarely needed; the engine is event-driven).
    Refresh,

    // ---- settings (saved to config.toml; result: `SettingsChanged` or `Notice::SettingRejected`) ----
    /// Creates or replaces a toggle pair from two live devices. `hotkey` like `"Ctrl+Alt+F12"`; `None` = no hotkey.
    SetTogglePair {
        pair_id: String,
        primary_id: String,
        secondary_id: String,
        hotkey: Option<String>,
        roles: Option<RoleSet>,
    },
    RemoveTogglePair {
        pair_id: String,
    },
    /// Creates or replaces a "switch to this device" hotkey.
    SetBinding {
        binding_id: String,
        device_id: String,
        hotkey: Option<String>,
        roles: Option<RoleSet>,
    },
    RemoveBinding {
        binding_id: String,
    },
    /// Replaces one flow's favourites, in order (live device IDs).
    SetFavourites {
        flow: Flow,
        device_ids: Vec<String>,
    },
    /// Patches a device's preferences. `None` fields stay as they are; an empty `alias` clears it.
    SetDevicePrefs {
        device_id: String,
        spatial: Option<SpatialMode>,
        alias: Option<String>,
        hidden: Option<bool>,
    },
    SetPerformanceProfile {
        profile: PerformanceProfile,
    },
    SetPinnedMonitor {
        display_path: String,
    },
    /// Turns the F31 knob gestures (Ctrl/Shift/Alt + knob) on or off.
    SetKnobGestures {
        enabled: bool,
    },
    /// Roles used by switches that don't specify their own.
    SetDefaultRoles {
        roles: RoleSet,
    },
}

/// A saved device reference as the UI sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct SavedDevice {
    /// Name it was saved under, e.g. "Headphones (Arctis Nova 7)".
    pub name: String,
    pub flow: Flow,
    /// The live endpoint it resolves to right now (`None` = not connected or needs a choice).
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct TogglePairView {
    pub id: String,
    pub hotkey: Option<String>,
    pub primary: SavedDevice,
    pub secondary: SavedDevice,
    pub roles: Option<RoleSet>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct BindingView {
    pub id: String,
    pub hotkey: Option<String>,
    pub device: SavedDevice,
    pub roles: Option<RoleSet>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct DevicePrefsView {
    pub device: SavedDevice,
    pub spatial: SpatialMode,
    pub alias: Option<String>,
    pub hidden: bool,
}

/// The user's settings, resolved against the live devices. Built from `config.toml`; edit it with the settings commands.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    #[serde(default)]
    pub performance_profile: PerformanceProfile,
    #[serde(default)]
    pub pinned_monitor: Option<String>,
    pub default_roles: RoleSet,
    pub toggle_pairs: Vec<TogglePairView>,
    pub bindings: Vec<BindingView>,
    pub favourites_playback: Vec<SavedDevice>,
    pub favourites_recording: Vec<SavedDevice>,
    pub devices: Vec<DevicePrefsView>,
    pub knob_gestures: bool,
}

/// Something the user should be told about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Notice {
    /// The bound device isn't connected. Nothing was changed.
    NotConnected { name: String },
    /// The engine needs the user to pick (two identical devices, or an uncertain re-link).
    ChooseDevice {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        request_id: u64,
        name: String,
        candidates: Vec<DeviceSummary>,
    },
    /// Windows refused some roles. The device list reflects what actually happened.
    SwitchFailed { name: String, failed_roles: Vec<Role> },
    /// The requested spatial format couldn't be applied (e.g. no Dolby Access licence). The switch still happened.
    SpatialUnavailable { name: String, mode: SpatialMode, reason: String },
    /// A hotkey couldn't be registered because another app owns it.
    HotkeyConflict { binding_id: String, keys: String },
    /// `config.toml` was unreadable; it was renamed to `backup` and defaults are in use.
    ConfigReplaced { backup: String, error: String },
    /// An edited `config.toml` is invalid; the last valid config stays in effect until the file is fixed.
    ConfigInvalid { error: String },
    /// An automatic save (e.g. a re-link) couldn't be written; the change only lasts until restart.
    ConfigSaveFailed { error: String },
    /// A settings command was refused (unknown/disconnected device, invalid or duplicate hotkey…). Nothing changed.
    SettingRejected { error: String },
}

/// Deltas. The notch maps these onto activities (switch, plug/unplug, volume, mic).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum EngineEvent {
    /// Device list or defaults changed (no user-facing activity on its own).
    DevicesChanged {
        playback: Vec<DeviceSummary>,
        recording: Vec<DeviceSummary>,
    },
    /// Volume of the default playback device changed, from any source (keys, knob, mixer, other apps).
    VolumeChanged {
        volume: VolumeState,
    },
    MicMuteChanged {
        muted: bool,
    },
    /// Microphone volume changed (e.g. Alt+knob, F31).
    MicVolumeChanged {
        percent: u8,
    },
    /// Track, playback state, position or art changed (or the media session went away: `None`).
    /// Album art bytes: `Engine::art(art.id)` (the app serves them as `iceart://<id>`).
    MediaChanged {
        media: Option<NowPlaying>,
    },
    /// A switch the user asked for completed and was confirmed by re-reading the defaults.
    Switched {
        device: DeviceSummary,
        roles: RoleSet,
        /// The binding followed the device to a new endpoint ID (shown as a caption, one notice).
        relinked: bool,
        /// Set when a spatial format was applied and read back.
        spatial_applied: Option<SpatialMode>,
    },
    /// A device became Active (plug-in, headset power-on).
    DeviceArrived {
        device: DeviceSummary,
    },
    /// A device stopped being Active.
    DeviceLeft {
        device: DeviceSummary,
    },
    Notice {
        notice: Notice,
    },
    /// The set of persistent problems changed (raised or resolved). Mirrors `EngineSnapshot::notices`.
    NoticesChanged {
        notices: Vec<Notice>,
    },
    /// Settings changed (a settings command, a hand edit of config.toml, a re-link) or a saved device (dis)connected.
    SettingsChanged {
        settings: SettingsView,
    },
}

/// One delta with its revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub revision: u64,
    pub event: EngineEvent,
}

/// The engine as seen by the shell. Implemented by the real engine (`phase-1/engine`) and [`fake::FakeEngine`].
pub trait Engine: Send + Sync {
    /// Current full state.
    fn snapshot(&self) -> EngineSnapshot;
    /// Fire-and-forget; results arrive as events.
    fn send(&self, command: Command);
    /// A new event stream. Subscribe *before* reading the snapshot.
    fn subscribe(&self) -> std::sync::mpsc::Receiver<Envelope>;
    /// Album-art PNG for an `ArtInfo::id` from the snapshot or a `MediaChanged` event (recent tracks only).
    fn art(&self, id: &str) -> Option<Vec<u8>> {
        let _ = id;
        None
    }
}

pub mod fake {
    //! An in-memory engine with the owner's devices, for UI/shell work and tests. No Windows calls.

    use std::sync::Mutex;
    use std::sync::mpsc::{Receiver, Sender, channel};

    use super::MediaAction;
    use super::{
        Command, DeviceSummary, Engine, EngineEvent, EngineSnapshot, Envelope, Notice, RoleSet, SpatialMode,
        VolumeState,
    };
    use crate::config::Config;
    use crate::device::EndpointInfo;
    use crate::media::{ArtInfo, NowPlaying, PlaybackStatus, Rgb};

    /// A 16×16 Ice-blue gradient, so UI work has a real image to style.
    const FAKE_ART_PNG: &[u8] = include_bytes!("fake_art.png");
    /// (artist, title, album, duration s, accent) — distinct accents so track changes are visible.
    const FAKE_TRACKS: [(&str, &str, &str, u64, Rgb); 3] = [
        ("Coldplay", "Higher Power", "Music Of The Spheres", 211, Rgb::new(0x6b, 0xc5, 0xff)),
        ("Ariana Grande", "One Last Time", "My Everything (Deluxe)", 197, Rgb::new(0xf0, 0x8a, 0xc8)),
        ("Daft Punk", "Instant Crush", "Random Access Memories", 337, Rgb::new(0xf2, 0xb8, 0x4b)),
    ];

    fn fake_track(i: usize, status: PlaybackStatus) -> NowPlaying {
        let (artist, title, album, secs, accent) = FAKE_TRACKS[i % FAKE_TRACKS.len()];
        NowPlaying {
            app_id: "Spotify.exe".into(),
            title: title.into(),
            artist: artist.into(),
            album: album.into(),
            status,
            position_ms: Some(52_000),
            duration_ms: Some(secs * 1000),
            art: Some(ArtInfo { id: format!("fake-art-{}", i % FAKE_TRACKS.len()), width: 16, height: 16, accent }),
        }
    }
    use crate::device::{DeviceState, Flow};

    struct State {
        snap: EngineSnapshot,
        /// Index into `FAKE_TRACKS`.
        track: usize,
        /// A real config, edited with the real `Config::apply_setting` (same validation as the real engine).
        config: Config,
        subscribers: Vec<Sender<Envelope>>,
    }

    pub struct FakeEngine {
        state: Mutex<State>,
    }

    fn device(id: &str, name: &str, flow: Flow, form_factor: u32, spatial: Option<SpatialMode>) -> DeviceSummary {
        DeviceSummary {
            id: id.into(),
            name: name.into(),
            flow,
            state: DeviceState::Active,
            is_default: false,
            is_default_communications: false,
            form_factor: Some(form_factor),
            is_virtual: false,
            spatial,
        }
    }

    impl Default for FakeEngine {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FakeEngine {
        /// The owner's setup: Arctis Nova 7 (default, Atmos), Realtek speakers, LG monitor; Arctis + webcam mics.
        #[must_use]
        pub fn new() -> Self {
            let mut arctis = device(
                "fake-arctis",
                "Headphones (Arctis Nova 7)",
                Flow::Render,
                3,
                Some(SpatialMode::DolbyAtmosHeadphones),
            );
            arctis.is_default = true;
            arctis.is_default_communications = true;
            let mut mic = device("fake-arctis-mic", "Microphone (Arctis Nova 7)", Flow::Capture, 4, None);
            mic.is_default_communications = true;
            let snap = EngineSnapshot {
                revision: 1,
                playback: vec![
                    arctis,
                    device("fake-realtek", "Speakers (Realtek(R) Audio)", Flow::Render, 1, Some(SpatialMode::Off)),
                    device(
                        "fake-lg",
                        "LG FULL HD (NVIDIA High Definition Audio)",
                        Flow::Render,
                        9,
                        Some(SpatialMode::Off),
                    ),
                ],
                recording: vec![mic, device("fake-ugreen", "Microphone (UGREEN Camera Audio)", Flow::Capture, 4, None)],
                volume: Some(VolumeState { percent: 40, muted: false }),
                mic_muted: Some(false),
                mic_volume: Some(80),
                notices: Vec::new(),
                settings: super::SettingsView::default(),
                media: Some(fake_track(0, PlaybackStatus::Paused)),
            };
            let mut config = Config::default();
            let eps = endpoints(&snap);
            for cmd in [
                Command::SetTogglePair {
                    pair_id: "main".into(),
                    primary_id: "fake-arctis".into(),
                    secondary_id: "fake-realtek".into(),
                    hotkey: Some("Ctrl+Alt+F12".into()),
                    roles: None,
                },
                Command::SetDevicePrefs {
                    device_id: "fake-arctis".into(),
                    spatial: Some(SpatialMode::DolbyAtmosHeadphones),
                    alias: None,
                    hidden: None,
                },
                Command::SetFavourites {
                    flow: crate::device::Flow::Render,
                    device_ids: vec!["fake-arctis".into(), "fake-realtek".into()],
                },
                Command::SetKnobGestures { enabled: true },
            ] {
                let _ = config.apply_setting(&cmd, &eps);
            }
            let mut snap = snap;
            snap.settings = config.view(&eps);
            Self { state: Mutex::new(State { snap, track: 0, config, subscribers: Vec::new() }) }
        }

        /// Simulates a device dropping out (e.g. headset powered off).
        pub fn simulate_unplug(&self, device_id: &str) {
            let mut st = self.lock();
            let Some(pos) = st.snap.playback.iter().position(|d| d.id == device_id) else { return };
            let mut dev = st.snap.playback.remove(pos);
            dev.state = DeviceState::NotPresent;
            // Same shape as the real engine's DeviceLeft (review ENG-14).
            dev.is_default = false;
            dev.is_default_communications = false;
            dev.spatial = None;
            emit(&mut st, EngineEvent::DeviceLeft { device: dev });
            refresh_settings(&mut st);
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, State> {
            self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
        }
    }

    /// Fake endpoints for the config code: one per visible device, with a unique fake hardware key.
    fn endpoints(snap: &EngineSnapshot) -> Vec<EndpointInfo> {
        snap.playback
            .iter()
            .chain(&snap.recording)
            .map(|d| EndpointInfo {
                id: d.id.clone(),
                flow: d.flow,
                state: d.state,
                friendly_name: Some(d.name.clone()),
                endpoint_name: d.name.split(" (").next().map(str::to_string),
                interface_name: None,
                devnode: None,
                hardware_key: Some(format!("FAKE\\{}", d.id)),
                container_id: None,
                form_factor: d.form_factor,
                stable_id: None,
                default_for: Vec::new(),
            })
            .collect()
    }

    /// Recomputes the settings view; emits `SettingsChanged` only when it changed.
    fn refresh_settings(st: &mut State) {
        let view = st.config.view(&endpoints(&st.snap));
        if view != st.snap.settings {
            st.snap.settings = view.clone();
            emit(st, EngineEvent::SettingsChanged { settings: view });
        }
    }

    fn emit(st: &mut State, event: EngineEvent) {
        st.snap.revision += 1;
        let env = Envelope { revision: st.snap.revision, event };
        st.subscribers.retain(|tx| tx.send(env.clone()).is_ok());
    }

    fn switch(st: &mut State, id: &str, roles: RoleSet) {
        let devices = if st.snap.playback.iter().any(|device| device.id == id) {
            &mut st.snap.playback
        } else {
            &mut st.snap.recording
        };
        let Some(found) = devices.iter().position(|device| device.id == id) else {
            emit(st, EngineEvent::Notice { notice: Notice::NotConnected { name: id.into() } });
            return;
        };
        for (i, d) in devices.iter_mut().enumerate() {
            if roles != RoleSet::Communications {
                d.is_default = i == found;
            }
            if roles != RoleSet::Default {
                d.is_default_communications = i == found;
            }
        }
        let device = devices[found].clone();
        let spatial_applied = device.spatial.filter(|s| *s != SpatialMode::Off);
        emit(st, EngineEvent::Switched { device, roles, relinked: false, spatial_applied });
    }

    fn set_volume(st: &mut State, percent: i16, toggle_mute: bool) {
        let v = st.snap.volume.get_or_insert(VolumeState { percent: 0, muted: false });
        v.percent = u8::try_from(percent.clamp(0, 100)).unwrap_or(0);
        if toggle_mute {
            v.muted = !v.muted;
        }
        let volume = *v;
        emit(st, EngineEvent::VolumeChanged { volume });
    }

    impl Engine for FakeEngine {
        fn snapshot(&self) -> EngineSnapshot {
            self.lock().snap.clone()
        }

        fn send(&self, command: Command) {
            let mut st = self.lock();
            let current = st.snap.volume.map_or(0, |v| i16::from(v.percent));
            match command {
                Command::SwitchTo { device_id, roles } => switch(&mut st, &device_id, roles),
                // The fake has one toggle pair (Arctis ↔ Speakers) and treats every binding as the Arctis.
                Command::TogglePair { .. } => {
                    let to = if st.snap.playback.iter().any(|d| d.id == "fake-arctis" && d.is_default) {
                        "fake-realtek"
                    } else {
                        "fake-arctis"
                    };
                    switch(&mut st, to, RoleSet::Default);
                }
                Command::SwitchBinding { .. } => switch(&mut st, "fake-arctis", RoleSet::Default),
                Command::CycleFavourites { step, .. } => {
                    let n = st.snap.playback.len();
                    if n > 0 {
                        let cur = st.snap.playback.iter().position(|d| d.is_default).unwrap_or(0);
                        let next = (cur + n).saturating_add_signed(isize::from(step)) % n;
                        let id = st.snap.playback[next].id.clone();
                        switch(&mut st, &id, RoleSet::Default);
                    }
                }
                Command::SetVolume { percent } => set_volume(&mut st, i16::from(percent), false),
                Command::StepVolume { delta } => set_volume(&mut st, current + i16::from(delta), false),
                Command::ToggleMute => set_volume(&mut st, current, true),
                Command::ToggleMicMute => {
                    let muted = !st.snap.mic_muted.unwrap_or(false);
                    st.snap.mic_muted = Some(muted);
                    emit(&mut st, EngineEvent::MicMuteChanged { muted });
                }
                Command::SetSpatial { device_id, mode } => {
                    if let Some(d) = st.snap.playback.iter_mut().find(|d| d.id == device_id) {
                        d.spatial = Some(mode);
                    }
                    let (playback, recording) = (st.snap.playback.clone(), st.snap.recording.clone());
                    emit(&mut st, EngineEvent::DevicesChanged { playback, recording });
                }
                Command::SetTogglePair { .. }
                | Command::RemoveTogglePair { .. }
                | Command::SetBinding { .. }
                | Command::RemoveBinding { .. }
                | Command::SetFavourites { .. }
                | Command::SetDevicePrefs { .. }
                | Command::SetPerformanceProfile { .. }
                | Command::SetPinnedMonitor { .. }
                | Command::SetKnobGestures { .. }
                | Command::SetDefaultRoles { .. } => {
                    let eps = endpoints(&st.snap);
                    match st.config.apply_setting(&command, &eps) {
                        Ok(true) => refresh_settings(&mut st),
                        Ok(false) => {}
                        Err(error) => emit(&mut st, EngineEvent::Notice { notice: Notice::SettingRejected { error } }),
                    }
                }
                Command::Media { action } => {
                    let status = st.snap.media.as_ref().map_or(PlaybackStatus::Paused, |m| m.status);
                    let (track, status) = match action {
                        MediaAction::PlayPause if status == PlaybackStatus::Playing => {
                            (st.track, PlaybackStatus::Paused)
                        }
                        MediaAction::PlayPause | MediaAction::Play => (st.track, PlaybackStatus::Playing),
                        MediaAction::Pause => (st.track, PlaybackStatus::Paused),
                        MediaAction::Next => (st.track + 1, status),
                        MediaAction::Previous => (st.track + FAKE_TRACKS.len() - 1, status),
                    };
                    st.track = track % FAKE_TRACKS.len();
                    let mut media = fake_track(st.track, status);
                    if !matches!(action, MediaAction::Next | MediaAction::Previous) {
                        media.position_ms = st.snap.media.as_ref().and_then(|m| m.position_ms);
                    } else {
                        media.position_ms = Some(0);
                    }
                    st.snap.media = Some(media.clone());
                    emit(&mut st, EngineEvent::MediaChanged { media: Some(media) });
                }
                Command::ResolveChoice { .. } | Command::Refresh => {
                    let (playback, recording) = (st.snap.playback.clone(), st.snap.recording.clone());
                    emit(&mut st, EngineEvent::DevicesChanged { playback, recording });
                }
            }
        }

        fn subscribe(&self) -> Receiver<Envelope> {
            let (tx, rx) = channel();
            self.lock().subscribers.push(tx);
            rx
        }

        fn art(&self, id: &str) -> Option<Vec<u8>> {
            (0..FAKE_TRACKS.len()).any(|i| id == format!("fake-art-{i}")).then(|| FAKE_ART_PNG.to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeEngine;
    use super::*;

    fn next(rx: &std::sync::mpsc::Receiver<Envelope>) -> Envelope {
        rx.try_recv().expect("an event")
    }

    #[test]
    fn toggle_pair_flips_and_revisions_increase() {
        let eng = FakeEngine::new();
        let rx = eng.subscribe();
        let start = eng.snapshot().revision;
        eng.send(Command::TogglePair { pair_id: "main".into() });
        let a = next(&rx);
        assert!(matches!(&a.event, EngineEvent::Switched { device, .. } if device.id == "fake-realtek"));
        eng.send(Command::TogglePair { pair_id: "main".into() });
        let b = next(&rx);
        assert!(
            matches!(&b.event, EngineEvent::Switched { device, spatial_applied: Some(SpatialMode::DolbyAtmosHeadphones), .. } if device.id == "fake-arctis")
        );
        assert!(start < a.revision && a.revision < b.revision);
        assert_eq!(eng.snapshot().revision, b.revision);
    }

    #[test]
    fn switching_recording_device_updates_only_recording_defaults() {
        let eng = FakeEngine::new();
        let rx = eng.subscribe();
        let previous_playback = eng.snapshot().playback;
        eng.send(Command::SwitchTo { device_id: "fake-ugreen".into(), roles: RoleSet::Both });
        assert!(matches!(
            next(&rx).event,
            EngineEvent::Switched { device, roles: RoleSet::Both, .. } if device.id == "fake-ugreen"
        ));
        let snapshot = eng.snapshot();
        assert_eq!(snapshot.playback, previous_playback);
        assert!(
            snapshot
                .recording
                .iter()
                .any(|device| { device.id == "fake-ugreen" && device.is_default && device.is_default_communications })
        );
    }

    #[test]
    fn volume_clamps_and_mute_toggles() {
        let eng = FakeEngine::new();
        let rx = eng.subscribe();
        eng.send(Command::StepVolume { delta: 100 });
        assert!(matches!(
            next(&rx).event,
            EngineEvent::VolumeChanged { volume: VolumeState { percent: 100, muted: false } }
        ));
        eng.send(Command::StepVolume { delta: -128 });
        eng.send(Command::ToggleMute);
        let _ = next(&rx);
        assert!(matches!(
            next(&rx).event,
            EngineEvent::VolumeChanged { volume: VolumeState { percent: 0, muted: true } }
        ));
    }

    #[test]
    fn unplug_emits_device_left_and_unknown_switch_is_a_notice() {
        let eng = FakeEngine::new();
        let rx = eng.subscribe();
        eng.simulate_unplug("fake-arctis");
        assert!(
            matches!(next(&rx).event, EngineEvent::DeviceLeft { ref device } if device.state == DeviceState::NotPresent)
        );
        // The saved toggle pair's headset is now disconnected.
        assert!(matches!(next(&rx).event, EngineEvent::SettingsChanged { .. }));
        eng.send(Command::SwitchTo { device_id: "fake-arctis".into(), roles: RoleSet::Default });
        assert!(matches!(next(&rx).event, EngineEvent::Notice { notice: Notice::NotConnected { .. } }));
    }

    #[test]
    fn commands_and_events_use_camel_case_tags() {
        let json = serde_json::to_string(&Command::SwitchTo { device_id: "x".into(), roles: RoleSet::Both }).unwrap();
        assert_eq!(json, r#"{"type":"switchTo","deviceId":"x","roles":"both"}"#);
        let env = Envelope { revision: 7, event: EngineEvent::MicMuteChanged { muted: true } };
        assert_eq!(
            serde_json::to_string(&env).unwrap(),
            r#"{"revision":7,"event":{"type":"micMuteChanged","muted":true}}"#
        );
        let back: Command = serde_json::from_str(r#"{"type":"stepVolume","delta":-2}"#).unwrap();
        assert_eq!(back, Command::StepVolume { delta: -2 });
    }

    #[cfg(feature = "ts")]
    #[test]
    fn typescript_matches_the_json_shape() {
        use ts_rs::{Config, TS};
        let cfg = Config::default();
        let cmd = Command::decl(&cfg);
        assert!(cmd.contains(r#""type": "switchTo""#) && cmd.contains("deviceId: string"), "{cmd}");
        let env = Envelope::decl(&cfg);
        assert!(env.contains("revision: number"), "{env}");
    }

    #[test]
    fn fake_settings_commands_use_the_real_validation() {
        let eng = FakeEngine::new();
        let snap = eng.snapshot();
        assert_eq!(snap.settings.toggle_pairs[0].hotkey.as_deref(), Some("Ctrl+Alt+F12"));
        assert!(snap.settings.knob_gestures);
        let rx = eng.subscribe();
        eng.send(Command::SetBinding {
            binding_id: "lg".into(),
            device_id: "fake-lg".into(),
            hotkey: Some("Ctrl+Alt+F12".into()),
            roles: None,
        });
        assert!(matches!(next(&rx).event, EngineEvent::Notice { notice: Notice::SettingRejected { .. } }));
        eng.send(Command::SetBinding {
            binding_id: "lg".into(),
            device_id: "fake-lg".into(),
            hotkey: Some("Ctrl+Alt+F11".into()),
            roles: None,
        });
        assert!(
            matches!(next(&rx).event, EngineEvent::SettingsChanged { ref settings } if settings.bindings.len() == 1)
        );
        eng.simulate_unplug("fake-arctis");
        let _left = next(&rx);
        assert!(
            matches!(next(&rx).event, EngineEvent::SettingsChanged { ref settings } if settings.toggle_pairs[0].primary.device_id.is_none())
        );
    }

    #[test]
    fn fake_media_plays_skips_and_serves_art() {
        use crate::media::PlaybackStatus;
        let eng = FakeEngine::new();
        let first = eng.snapshot().media.unwrap();
        assert_eq!(first.status, PlaybackStatus::Paused);
        let png = eng.art(&first.art.as_ref().unwrap().id).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(eng.art("unknown").is_none());
        let rx = eng.subscribe();
        eng.send(Command::Media { action: MediaAction::PlayPause });
        assert!(
            matches!(next(&rx).event, EngineEvent::MediaChanged { media: Some(ref m) } if m.status == PlaybackStatus::Playing)
        );
        eng.send(Command::Media { action: MediaAction::Next });
        match next(&rx).event {
            EngineEvent::MediaChanged { media: Some(m) } => {
                assert_ne!(m.title, first.title);
                assert_eq!(m.status, PlaybackStatus::Playing, "skipping keeps playing");
                assert_eq!(m.position_ms, Some(0));
            }
            other => panic!("{other:?}"),
        }
        eng.send(Command::Media { action: MediaAction::Previous });
        assert!(matches!(next(&rx).event, EngineEvent::MediaChanged { media: Some(ref m) } if m.title == first.title));
    }

    #[cfg(feature = "ts")]
    #[test]
    fn media_types_export_to_typescript() {
        use ts_rs::{Config, TS};
        let cfg = Config::default();
        assert!(crate::media::NowPlaying::decl(&cfg).contains("positionMs"));
        assert!(MediaAction::decl(&cfg).contains("\"playPause\""));
    }
}
