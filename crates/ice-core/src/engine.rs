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
}

pub mod fake {
    //! An in-memory engine with the owner's devices, for UI/shell work and tests. No Windows calls.

    use std::sync::Mutex;
    use std::sync::mpsc::{Receiver, Sender, channel};

    use super::{
        Command, DeviceSummary, Engine, EngineEvent, EngineSnapshot, Envelope, Notice, RoleSet, SpatialMode,
        VolumeState,
    };
    use crate::device::{DeviceState, Flow};

    struct State {
        snap: EngineSnapshot,
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
            };
            Self { state: Mutex::new(State { snap, subscribers: Vec::new() }) }
        }

        /// Simulates a device dropping out (e.g. headset powered off).
        pub fn simulate_unplug(&self, device_id: &str) {
            let mut st = self.lock();
            let Some(pos) = st.snap.playback.iter().position(|d| d.id == device_id) else { return };
            let mut dev = st.snap.playback.remove(pos);
            dev.state = DeviceState::NotPresent;
            emit(&mut st, EngineEvent::DeviceLeft { device: dev });
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, State> {
            self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
        }
    }

    fn emit(st: &mut State, event: EngineEvent) {
        st.snap.revision += 1;
        let env = Envelope { revision: st.snap.revision, event };
        st.subscribers.retain(|tx| tx.send(env.clone()).is_ok());
    }

    fn switch(st: &mut State, id: &str, roles: RoleSet) {
        let Some(found) = st.snap.playback.iter().position(|d| d.id == id) else {
            emit(st, EngineEvent::Notice { notice: Notice::NotConnected { name: id.into() } });
            return;
        };
        for (i, d) in st.snap.playback.iter_mut().enumerate() {
            if roles != RoleSet::Communications {
                d.is_default = i == found;
            }
            if roles != RoleSet::Default {
                d.is_default_communications = i == found;
            }
        }
        let device = st.snap.playback[found].clone();
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
}
