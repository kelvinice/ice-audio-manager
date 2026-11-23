//! An in-memory engine with the owner's devices, for UI/shell work and tests. No Windows calls.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::MediaAction;
use super::{
    Command, DeviceSummary, Engine, EngineEvent, EngineSnapshot, Envelope, Notice, RoleSet, SpatialMode, VolumeState,
};
use crate::config::Config;
use crate::device::EndpointInfo;
use crate::media::{ArtInfo, NowPlaying, PlaybackStatus, Rgb};
use crate::meter::{METER_INTERVAL, MeterFrame, MeterSubscription};

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
    /// Shared with the fake meter threads, which follow the play state.
    state: Arc<Mutex<State>>,
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Made-up stereo levels for tick `t`: two beating sines, so the visualizer has movement to style.
fn fake_levels(t: u32) -> MeterFrame {
    #[expect(clippy::cast_precision_loss, reason = "the tick wraps at 2^16, exact in f32")]
    let x = (t % 65_536) as f32;
    let left = 0.45 + 0.35 * (x * 0.37).sin() * (x * 0.05).sin().abs();
    let right = 0.40 + 0.35 * (x * 0.29 + 1.0).sin() * (x * 0.07).sin().abs();
    MeterFrame::new(left.max(right), vec![left, right])
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
                device("fake-lg", "LG FULL HD (NVIDIA High Definition Audio)", Flow::Render, 9, Some(SpatialMode::Off)),
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
        Self { state: Arc::new(Mutex::new(State { snap, track: 0, config, subscribers: Vec::new() })) }
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

    fn lock(&self) -> MutexGuard<'_, State> {
        lock(&self.state)
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
                    MediaAction::PlayPause if status == PlaybackStatus::Playing => (st.track, PlaybackStatus::Paused),
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

    /// Moving levels while the fake track plays and isn't muted, silence otherwise. One thread per
    /// subscription, which ends when the subscription is dropped.
    fn meter(&self) -> Option<MeterSubscription> {
        let (mut sink, subscription) = crate::meter::channel();
        let state = Arc::clone(&self.state);
        std::thread::Builder::new()
            .name("fake-meter".into())
            .spawn(move || {
                let mut tick = 0_u32;
                loop {
                    let audible = {
                        let st = lock(&state);
                        st.snap.media.as_ref().is_some_and(|m| m.status == PlaybackStatus::Playing)
                            && !st.snap.volume.is_some_and(|v| v.muted)
                    };
                    let frame = if audible { fake_levels(tick) } else { MeterFrame::default() };
                    if !sink.offer(&frame) {
                        return;
                    }
                    tick = tick.wrapping_add(1);
                    std::thread::sleep(METER_INTERVAL);
                }
            })
            .ok()?;
        Some(subscription)
    }
}
