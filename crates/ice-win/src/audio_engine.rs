//! The real engine (plan.md track A): implements [`ice_core::engine::Engine`] on Core Audio.
//!
//! One `ice-audio` thread owns every COM object (MTA). COM callbacks, hotkeys and the config watcher only
//! post messages to it; nothing they do can block it. Device-change bursts (one unplug = state + 3 default
//! changes + dozens of property changes) are coalesced into one refresh after a sliding quiet period.
//!
//! "The current playback device" means the **Multimedia** default throughout (what media plays on and what
//! the volume events follow); switches set Console + Multimedia together (review ENG-13).

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ice_core::config::{Config, ConfigStore, KnobGestures, Loaded};
use ice_core::device::{DeviceState, EndpointInfo, Flow, Role};
use ice_core::engine::{
    Command, DeviceSummary, Engine, EngineEvent, EngineSnapshot, Envelope, Notice, RoleSet, SpatialMode, VolumeState,
};
use ice_core::event::AudioEvent;
use ice_core::hotkey::Chord;
use ice_core::resolve::{self, DeviceRef, Resolution, ToggleAction, normalize_name};
use windows::Win32::Media::Audio::IMMDevice;

use crate::com::ComGuard;
use crate::endpoints::Endpoints;
use crate::file_watch::DirWatch;
use crate::hotkeys::HotkeyService;
use crate::media_service::{MediaService, MediaUpdate};
use crate::meter::MeterHub;
use crate::spatial::{self, SpatialFormat};
use crate::{notify, policy, volume};

/// Quiet period before a device-change burst is turned into one refresh (sliding)…
const REFRESH_DEBOUNCE: Duration = Duration::from_millis(120);
/// …but never later than this after the first event of the burst.
const REFRESH_MAX_WAIT: Duration = Duration::from_millis(600);
/// Editors save in several writes; wait this long after the last change before re-reading.
const RELOAD_DEBOUNCE: Duration = Duration::from_millis(300);
/// Volume step for Alt+knob (mic volume).
const MIC_STEP: f32 = 0.02;
/// A hung spatial provider must not stall the audio thread (review ENG-02).
const SPATIAL_WRITE_LIMIT: Duration = Duration::from_secs(3);
/// Per-subscriber buffer. A subscriber that falls this far behind is dropped and must re-subscribe and
/// re-read the snapshot (code-practices §6, review ENG-12).
const SUBSCRIBER_BUFFER: usize = 1024;
/// Unanswered device pickers kept at most.
const MAX_PENDING_CHOICES: usize = 8;
/// Album-art PNGs kept for `Engine::art` (the current track and a few before it; ≤ ~40 KB each).
const ART_CACHE: usize = 8;

enum Msg {
    Command(Command),
    /// Endpoint notifications and the playback device's volume.
    Audio(AudioEvent),
    /// The communications microphone's volume/mute.
    Mic {
        scalar: f32,
        muted: bool,
    },
    Hotkey(i32),
    /// Something in the config folder changed (debounced, then re-read).
    ConfigChanged,
    /// A finished now-playing read from the `ice-media` thread.
    Media(Box<MediaUpdate>),
    Quit,
}

struct Shared {
    snapshot: EngineSnapshot,
    subscribers: Vec<SyncSender<Envelope>>,
    /// Recent album art by id, newest last.
    art: VecDeque<(String, Vec<u8>)>,
    /// Visualizer levels on their own thread, which runs only while subscribed.
    meter: Arc<MeterHub>,
}

fn lock(m: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Handle to the engine thread. Dropping it stops the thread and unregisters everything.
pub struct AudioEngine {
    tx: Sender<Msg>,
    shared: Arc<Mutex<Shared>>,
    join: Option<JoinHandle<()>>,
}

impl AudioEngine {
    /// Starts the engine and returns once the first snapshot is ready.
    ///
    /// # Errors
    /// A readable reason if COM or the audio service is unavailable.
    pub fn start(store: ConfigStore) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Shared {
            snapshot: EngineSnapshot::default(),
            subscribers: Vec::new(),
            art: VecDeque::new(),
            meter: MeterHub::new(),
        }));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let (worker_shared, worker_tx) = (Arc::clone(&shared), tx.clone());
        let join = std::thread::Builder::new()
            .name("ice-audio".into())
            .spawn(move || match Worker::new(store, worker_shared, worker_tx) {
                Ok(mut w) => {
                    let _ = ready_tx.send(Ok(()));
                    w.run(&rx);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| e.to_string())?;
        ready_rx.recv().map_err(|_| "engine thread exited during start-up".to_string())??;
        Ok(Self { tx, shared, join: Some(join) })
    }
}

impl Engine for AudioEngine {
    fn snapshot(&self) -> EngineSnapshot {
        lock(&self.shared).snapshot.clone()
    }

    fn send(&self, command: Command) {
        let _ = self.tx.send(Msg::Command(command));
    }

    /// Persistent problems are in [`EngineSnapshot::notices`]; nothing is replayed (review ENG-08).
    fn subscribe(&self) -> Receiver<Envelope> {
        let (tx, rx) = mpsc::sync_channel(SUBSCRIBER_BUFFER);
        lock(&self.shared).subscribers.push(tx);
        rx
    }

    fn art(&self, id: &str) -> Option<Vec<u8>> {
        lock(&self.shared).art.iter().find(|(k, _)| k == id).map(|(_, png)| png.clone())
    }

    fn meter(&self) -> Option<ice_core::meter::MeterSubscription> {
        let hub = Arc::clone(&lock(&self.shared).meter);
        hub.subscribe()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        lock(&self.shared).meter.close();
        let _ = self.tx.send(Msg::Quit);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// A [`Resolution`] that no longer borrows the endpoint list (so the worker can mutate itself while acting on it).
enum Owned {
    Exact(EndpointInfo),
    Relinked(EndpointInfo),
    Choose,
    NotConnected,
}

impl From<Resolution<'_>> for Owned {
    fn from(r: Resolution<'_>) -> Self {
        match r {
            Resolution::Exact(e) => Self::Exact(e.clone()),
            Resolution::StableId(e) | Resolution::Relinked(e) => Self::Relinked(e.clone()),
            Resolution::NeedsConfirmation(_) | Resolution::Ambiguous => Self::Choose,
            Resolution::NotConnected => Self::NotConnected,
        }
    }
}

#[derive(Debug, Clone)]
enum HotkeyAction {
    Binding(String),
    Pair(String),
    CycleFavourites,
    KnobCycle(i32),
    KnobFine(i8),
    KnobMic(f32),
}

/// An open device picker. Holds the saved reference itself, not an index into the config, so a hot
/// reload can't make it point at a different binding (review ENG-03).
struct PendingChoice {
    dref: DeviceRef,
    roles: RoleSet,
}

struct Worker {
    eps: Endpoints,
    endpoints: Vec<EndpointInfo>,
    store: ConfigStore,
    config: Config,
    /// The config file's text as we last read or wrote it, to detect external edits before saving (ENG-04).
    disk_text: Option<String>,
    shared: Arc<Mutex<Shared>>,
    revision: u64,
    engine_tx: Sender<Msg>,
    render_watch: Option<volume::VolumeWatch>,
    mic_watch: Option<volume::VolumeWatch>,
    _notify: notify::Registration,
    hotkeys: Option<HotkeyService>,
    actions: BTreeMap<i32, HotkeyAction>,
    pending: BTreeMap<u64, PendingChoice>,
    next_request: u64,
    refresh_at: Option<Instant>,
    refresh_first: Option<Instant>,
    _config_watch: Option<DirWatch>,
    reload_at: Option<Instant>,
    // Persistent problems, published as `EngineSnapshot::notices`.
    config_replaced: Option<Notice>,
    config_invalid: Option<Notice>,
    save_failed: Option<Notice>,
    hotkey_conflicts: Vec<Notice>,
    knob_steps: i32,
    /// Direction of the first detent of the current Ctrl+knob gesture.
    knob_first: i32,
    knob_at: Option<Instant>,
    /// Now playing (GSMTC) on its own thread; `None` if it couldn't start.
    media: Option<MediaService>,
    // Must drop last: every COM object above is released before COM is uninitialised.
    _com: ComGuard,
}

fn audio_sink(tx: &Sender<Msg>) -> impl Fn(AudioEvent) + Send + Sync + 'static {
    let tx = tx.clone();
    move |e| {
        let _ = tx.send(Msg::Audio(e));
    }
}

fn mic_sink(tx: &Sender<Msg>) -> impl Fn(AudioEvent) + Send + Sync + 'static {
    let tx = tx.clone();
    move |e| {
        if let AudioEvent::Volume { scalar, muted } = e {
            let _ = tx.send(Msg::Mic { scalar, muted });
        }
    }
}

impl Worker {
    fn new(store: ConfigStore, shared: Arc<Mutex<Shared>>, engine_tx: Sender<Msg>) -> Result<Self, String> {
        let com = ComGuard::init_mta().map_err(|e| format!("COM: {e}"))?;
        let eps = Endpoints::new().map_err(|e| format!("audio service: {e}"))?;
        let registration =
            notify::register(eps.raw(), audio_sink(&engine_tx)).map_err(|e| format!("notifications: {e}"))?;
        let loaded = store.load();
        let config = loaded.config().clone();
        let disk_text = match &loaded {
            Loaded::Ok(_) => store.read_text().ok().flatten(),
            _ => None,
        };
        let config_replaced = match loaded {
            Loaded::Replaced { backup, error, .. } => Some(Notice::ConfigReplaced {
                backup: backup
                    .map_or_else(|| "(could not be moved; left in place)".into(), |b| b.display().to_string()),
                error,
            }),
            _ => None,
        };
        // Hot reload (code-practices §7): watch the config folder, never poll.
        let config_watch = store.path().parent().and_then(|dir| {
            if let Err(e) = std::fs::create_dir_all(dir) {
                tracing::warn!(error = %e, "config folder unavailable; hot reload disabled");
                return None;
            }
            let tx = engine_tx.clone();
            DirWatch::start(dir, move || {
                let _ = tx.send(Msg::ConfigChanged);
            })
            .inspect_err(|e| tracing::warn!(error = %e, "config watcher failed; hot reload disabled"))
            .ok()
        });
        let media_tx = engine_tx.clone();
        let media = MediaService::start(move |u| {
            let _ = media_tx.send(Msg::Media(Box::new(u)));
        })
        .inspect_err(|e| tracing::warn!(error = %e, "media thread failed to start; now playing disabled"))
        .ok();
        let mut w = Self {
            eps,
            endpoints: Vec::new(),
            store,
            config,
            disk_text,
            shared,
            revision: 0,
            engine_tx,
            render_watch: None,
            mic_watch: None,
            _notify: registration,
            hotkeys: None,
            actions: BTreeMap::new(),
            pending: BTreeMap::new(),
            next_request: 1,
            refresh_at: None,
            refresh_first: None,
            _config_watch: config_watch,
            reload_at: None,
            config_replaced,
            config_invalid: None,
            save_failed: None,
            hotkey_conflicts: Vec::new(),
            knob_steps: 0,
            knob_first: 0,
            knob_at: None,
            media,
            _com: com,
        };
        w.refresh(false);
        w.register_hotkeys();
        w.publish_notices();
        // The first snapshot is revision 1, like the fake engine (review ENG-12).
        w.revision = 1;
        lock(&w.shared).snapshot.revision = 1;
        Ok(w)
    }

    fn run(&mut self, rx: &Receiver<Msg>) {
        loop {
            let deadline = [self.refresh_at, self.knob_at, self.reload_at].into_iter().flatten().min();
            let msg = match deadline {
                Some(d) => match rx.recv_timeout(d.saturating_duration_since(Instant::now())) {
                    Ok(m) => m,
                    Err(RecvTimeoutError::Timeout) => {
                        self.on_deadlines();
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                },
                None => match rx.recv() {
                    Ok(m) => m,
                    Err(_) => break,
                },
            };
            match msg {
                Msg::Quit => break,
                Msg::Command(c) => self.command(c),
                Msg::Audio(e) => self.audio(e),
                Msg::Mic { scalar, muted } => self.on_mic(scalar, muted),
                Msg::Hotkey(id) => self.hotkey(id),
                Msg::ConfigChanged => self.reload_at = Some(Instant::now() + RELOAD_DEBOUNCE),
                Msg::Media(u) => self.on_media(*u),
            }
        }
    }

    fn on_deadlines(&mut self) {
        let now = Instant::now();
        if self.reload_at.is_some_and(|t| t <= now) {
            self.reload_at = None;
            self.reload_config();
        }
        if self.refresh_at.is_some_and(|t| t <= now) {
            self.refresh_at = None;
            self.refresh_first = None;
            self.refresh(true);
        }
        if self.knob_at.is_some_and(|t| t <= now) {
            self.knob_at = None;
            // One device per gesture, in the spin direction. Counting detents is unsteerable without a visible
            // picker (fast spins are 13-40 ms/detent); the Phase 2 notch picker can browse by detent instead.
            let net = std::mem::take(&mut self.knob_steps);
            let first = std::mem::take(&mut self.knob_first);
            let step = if net == 0 { first } else { net.signum() };
            if step != 0 {
                self.cycle(Flow::Render, step);
            }
        }
    }

    // ---- events out -------------------------------------------------------------------------------

    fn emit(&mut self, event: EngineEvent) {
        self.revision += 1;
        let env = Envelope { revision: self.revision, event };
        let mut sh = lock(&self.shared);
        sh.snapshot.revision = self.revision;
        // A full buffer means the subscriber stopped reading: drop it rather than grow without bound.
        sh.subscribers.retain(|tx| match tx.try_send(env.clone()) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                tracing::warn!("dropping a subscriber that fell {SUBSCRIBER_BUFFER} events behind");
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        });
    }

    /// Caches new art before announcing it, so the UI's `iceart://` request finds it.
    fn on_media(&mut self, u: MediaUpdate) {
        let changed = {
            let mut sh = lock(&self.shared);
            if let Some(art) = u.art
                && !sh.art.iter().any(|(id, _)| *id == art.info.id)
            {
                if sh.art.len() >= ART_CACHE {
                    sh.art.pop_front();
                }
                sh.art.push_back((art.info.id, art.png));
            }
            let changed = sh.snapshot.media != u.media;
            sh.snapshot.media.clone_from(&u.media);
            changed
        };
        if changed {
            self.emit(EngineEvent::MediaChanged { media: u.media });
        }
    }

    fn notice(&mut self, notice: Notice) {
        self.emit(EngineEvent::Notice { notice });
    }

    /// Copies the current persistent problems into the snapshot, and tells subscribers when they changed.
    fn publish_notices(&mut self) {
        let notices: Vec<Notice> = [&self.config_replaced, &self.config_invalid, &self.save_failed]
            .into_iter()
            .flatten()
            .cloned()
            .chain(self.hotkey_conflicts.iter().cloned())
            .collect();
        let changed = {
            let mut sh = lock(&self.shared);
            let changed = sh.snapshot.notices != notices;
            sh.snapshot.notices.clone_from(&notices);
            changed
        };
        if changed {
            self.emit(EngineEvent::NoticesChanged { notices });
        }
    }

    // ---- device state -----------------------------------------------------------------------------

    /// A summary without any WinRT calls (spatial unknown).
    fn summary_basic(&self, e: &EndpointInfo) -> DeviceSummary {
        DeviceSummary {
            id: e.id.clone(),
            name: self
                .config
                .prefs_for(e)
                .and_then(|p| p.alias.clone())
                .unwrap_or_else(|| normalize_name(e.friendly_name.as_deref().unwrap_or(&e.id))),
            flow: e.flow,
            state: e.state,
            is_default: e.default_for.contains(&Role::Multimedia),
            is_default_communications: e.default_for.contains(&Role::Communications),
            form_factor: e.form_factor,
            is_virtual: e.hardware_key.as_deref().is_some_and(|k| k.starts_with("ROOT\\") || k.starts_with("SWD\\")),
            spatial: None,
        }
    }

    /// Full summary: one fast spatial read for playback devices (review ENG-02).
    fn summary(&self, e: &EndpointInfo) -> DeviceSummary {
        let spatial = if e.flow == Flow::Render { spatial::active(&e.id).ok().and_then(mode_from) } else { None };
        DeviceSummary { spatial, ..self.summary_basic(e) }
    }

    /// Re-enumerates, updates the snapshot and (when `notify`) emits arrivals, departures and `DevicesChanged`.
    fn refresh(&mut self, notify: bool) {
        let fresh = match self.eps.list(false) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(error = %e, "endpoint enumeration failed; keeping the previous device list");
                return;
            }
        };
        let old = std::mem::replace(&mut self.endpoints, fresh);
        let current = |list: &[EndpointInfo]| {
            list.iter()
                .find(|e| e.flow == Flow::Render && e.default_for.contains(&Role::Multimedia))
                .map(|e| e.id.clone())
        };
        let (old_default, new_default) = (current(&old), current(&self.endpoints));

        // One summary per endpoint per refresh (review ENG-02).
        let summaries: Vec<DeviceSummary> = self.endpoints.iter().map(|e| self.summary(e)).collect();
        let visible = |flow: Flow| -> Vec<DeviceSummary> {
            self.endpoints
                .iter()
                .zip(&summaries)
                .filter(|(e, _)| e.flow == flow && !self.config.prefs_for(e).is_some_and(|p| p.hidden))
                .map(|(_, s)| s.clone())
                .collect()
        };
        let (playback, recording) = (visible(Flow::Render), visible(Flow::Capture));

        // Re-create both watches on every refresh: a same-ID reconnect can leave the old interfaces stale (ENG-07).
        self.render_watch = None;
        self.mic_watch = None;
        lock(&self.shared).meter.rebind();
        let render_dev = self.eps.default_device(Flow::Render, Role::Multimedia).ok();
        let mic_dev = self.mic_device();
        self.render_watch = render_dev.as_ref().and_then(|d| {
            volume::VolumeWatch::new(d, audio_sink(&self.engine_tx))
                .inspect_err(|e| tracing::warn!(error = %e, "playback volume watch failed"))
                .ok()
        });
        self.mic_watch = mic_dev.as_ref().and_then(|d| {
            volume::VolumeWatch::new(d, mic_sink(&self.engine_tx))
                .inspect_err(|e| tracing::warn!(error = %e, "microphone volume watch failed"))
                .ok()
        });
        let volume = render_dev
            .as_ref()
            .and_then(|d| volume::get(d).ok())
            .map(|(scalar, muted)| VolumeState { percent: percent(scalar), muted });
        let mic = mic_dev.as_ref().and_then(|d| volume::get(d).ok());

        let lists_changed = {
            let mut sh = lock(&self.shared);
            let changed = sh.snapshot.playback != playback || sh.snapshot.recording != recording;
            sh.snapshot.playback.clone_from(&playback);
            sh.snapshot.recording.clone_from(&recording);
            sh.snapshot.volume = volume;
            sh.snapshot.mic_muted = mic.map(|(_, m)| m);
            sh.snapshot.mic_volume = mic.map(|(s, _)| percent(s));
            changed
        };
        self.publish_settings(notify);
        if !notify {
            return;
        }
        let arrived: Vec<DeviceSummary> = self
            .endpoints
            .iter()
            .zip(&summaries)
            .filter(|(e, _)| !old.iter().any(|o| o.id == e.id))
            .map(|(_, s)| s.clone())
            .collect();
        // Departed devices: no WinRT calls on endpoints that are gone.
        let left: Vec<DeviceSummary> = old
            .iter()
            .filter(|o| !self.endpoints.iter().any(|e| e.id == o.id))
            .map(|o| DeviceSummary {
                state: DeviceState::NotPresent,
                is_default: false,
                is_default_communications: false,
                ..self.summary_basic(o)
            })
            .collect();
        for device in arrived {
            self.emit(EngineEvent::DeviceArrived { device });
        }
        for device in left {
            self.emit(EngineEvent::DeviceLeft { device });
        }
        if lists_changed {
            self.emit(EngineEvent::DevicesChanged { playback, recording });
        }
        if old_default != new_default
            && let Some(volume) = volume
        {
            self.emit(EngineEvent::VolumeChanged { volume });
        }
    }

    fn default_id(&self, flow: Flow, role: Role) -> Option<String> {
        self.endpoints.iter().find(|e| e.flow == flow && e.default_for.contains(&role)).map(|e| e.id.clone())
    }

    fn mic_device(&self) -> Option<IMMDevice> {
        self.eps.default_device(Flow::Capture, Role::Communications).ok()
    }

    fn audio(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::Volume { scalar, muted } => {
                let volume = VolumeState { percent: percent(scalar), muted };
                lock(&self.shared).snapshot.volume = Some(volume);
                self.emit(EngineEvent::VolumeChanged { volume });
            }
            // Very chatty and rarely user-visible; a real change also arrives as state/default events.
            AudioEvent::PropertyChanged { .. } => {}
            _ => {
                // Sliding quiet window, capped so a long burst still refreshes (review ENG-09).
                let now = Instant::now();
                let first = *self.refresh_first.get_or_insert(now);
                self.refresh_at = Some((now + REFRESH_DEBOUNCE).min(first + REFRESH_MAX_WAIT));
            }
        }
    }

    /// Microphone volume/mute from the mic watch or our own changes; emits only real changes.
    fn on_mic(&mut self, scalar: f32, muted: bool) {
        let p = percent(scalar);
        let (muted_changed, volume_changed) = {
            let mut sh = lock(&self.shared);
            let changes = (sh.snapshot.mic_muted != Some(muted), sh.snapshot.mic_volume != Some(p));
            sh.snapshot.mic_muted = Some(muted);
            sh.snapshot.mic_volume = Some(p);
            changes
        };
        if muted_changed {
            self.emit(EngineEvent::MicMuteChanged { muted });
        }
        if volume_changed {
            self.emit(EngineEvent::MicVolumeChanged { percent: p });
        }
    }

    // ---- config -----------------------------------------------------------------------------------

    /// Applies an edited config.toml. Invalid files are reported once and the last valid config stays in effect.
    fn reload_config(&mut self) {
        let text = match self.store.read_text() {
            Ok(Some(t)) => t,
            Ok(None) => return, // deleted: keep running; the next save recreates it
            Err(e) => return self.config_problem(format!("config I/O error: {e}")),
        };
        if Some(&text) == self.disk_text.as_ref() {
            // Our own save, or the user put back the last good file: nothing to apply, but a reported problem is over.
            if self.config_invalid.take().is_some() {
                self.publish_notices();
            }
            return;
        }
        match Config::from_toml(&text) {
            Ok(cfg) => {
                self.disk_text = Some(text);
                self.config_invalid = None;
                self.config_replaced = None;
                self.set_config(cfg);
            }
            Err(e) => self.config_problem(e.to_string()),
        }
    }

    fn config_problem(&mut self, error: String) {
        let notice = Notice::ConfigInvalid { error };
        if self.config_invalid.as_ref() != Some(&notice) {
            self.config_invalid = Some(notice.clone());
            self.publish_notices();
            self.notice(notice);
        }
    }

    fn set_config(&mut self, cfg: Config) {
        if cfg != self.config {
            self.config = cfg;
            self.pending.clear(); // pickers refer to the old bindings (review ENG-03)
            self.hotkeys = None; // dropping the service unregisters every chord
            self.actions.clear();
            self.register_hotkeys();
            self.refresh(true);
        }
        self.publish_notices();
    }

    /// After a confirmed switch to a re-created endpoint, points every reference to the old endpoint at the new
    /// one and saves. If the file changed on disk since we last read it, the user's version is adopted first;
    /// a half-edited (invalid) file is never overwritten (code-practices §7, review ENG-04/ENG-10).
    fn persist_relink(&mut self, old: &DeviceRef, target: &EndpointInfo) {
        if !self.sync_with_disk("re-link") {
            return;
        }
        if self.config.relink_all(&old.endpoint_id, target) == 0 {
            return;
        }
        self.write_config();
        self.publish_settings(true);
    }

    /// Before any automatic save: if config.toml changed on disk since we last read or wrote it, adopt the
    /// user's version first. Returns `false` (and reports it) when the file is mid-edit and invalid, so it is
    /// never overwritten (code-practices §7).
    fn sync_with_disk(&mut self, what: &str) -> bool {
        match self.store.read_text() {
            Ok(text) if text != self.disk_text => {
                if let Some(t) = text {
                    match Config::from_toml(&t) {
                        Ok(cfg) => {
                            self.disk_text = Some(t);
                            self.set_config(cfg);
                        }
                        Err(_) => {
                            self.reload_at = Some(Instant::now() + RELOAD_DEBOUNCE);
                            self.save_problem(format!("config.toml is being edited and is invalid; {what} not saved"));
                            return false;
                        }
                    }
                }
                true
            }
            Ok(_) => true,
            Err(e) => {
                self.save_problem(e.to_string());
                false
            }
        }
    }

    /// Saves the in-memory config; a failure is reported and the change lasts until restart.
    fn write_config(&mut self) {
        match self.store.save(&self.config) {
            Ok(text) => {
                self.disk_text = Some(text);
                if self.save_failed.take().is_some() {
                    self.publish_notices();
                }
            }
            Err(e) => self.save_problem(e.to_string()),
        }
    }

    /// A settings command from the tray/settings UI: validate against the live devices, save, apply.
    fn edit_settings(&mut self, command: &Command) {
        if !self.sync_with_disk("setting") {
            return;
        }
        let mut cfg = self.config.clone();
        match cfg.apply_setting(command, &self.endpoints) {
            Ok(false) => {}
            Ok(true) => {
                let previous = std::mem::replace(&mut self.config, cfg.clone());
                self.write_config();
                // set_config compares against the current config, so put the old one back for it to diff.
                self.config = previous;
                self.set_config(cfg);
                self.publish_settings(true);
            }
            Err(error) => self.notice(Notice::SettingRejected { error }),
        }
    }

    /// Recomputes the settings view (it depends on which saved devices are connected); emits
    /// `SettingsChanged` only when it changed.
    fn publish_settings(&mut self, emit: bool) {
        let view = self.config.view(&self.endpoints);
        let changed = {
            let mut sh = lock(&self.shared);
            let changed = sh.snapshot.settings != view;
            if changed {
                sh.snapshot.settings = view.clone();
            }
            changed
        };
        if changed && emit {
            self.emit(EngineEvent::SettingsChanged { settings: view });
        }
    }

    fn save_problem(&mut self, error: String) {
        tracing::warn!(%error, "config save failed");
        let notice = Notice::ConfigSaveFailed { error };
        self.save_failed = Some(notice.clone());
        self.publish_notices();
        self.notice(notice);
    }

    // ---- hotkeys ----------------------------------------------------------------------------------

    fn register_hotkeys(&mut self) {
        self.hotkey_conflicts.clear();
        let mut wanted: Vec<(Chord, HotkeyAction, String, bool)> = Vec::new();
        for b in &self.config.bindings {
            if let Some(c) = b.hotkey {
                wanted.push((c, HotkeyAction::Binding(b.id.clone()), b.id.clone(), false));
            }
        }
        for p in &self.config.toggle_pairs {
            if let Some(c) = p.hotkey {
                wanted.push((c, HotkeyAction::Pair(p.id.clone()), p.id.clone(), false));
            }
        }
        if let Some(c) = self.config.favourites.cycle_hotkey {
            wanted.push((c, HotkeyAction::CycleFavourites, "favourites".into(), false));
        }
        wanted.extend(knob_chords(&self.config.knob));
        if wanted.is_empty() {
            return;
        }
        let tx = self.engine_tx.clone();
        let svc = match HotkeyService::start(move |id| {
            let _ = tx.send(Msg::Hotkey(id));
        }) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "hotkey service failed to start");
                return;
            }
        };
        for (id, (chord, action, name, repeat)) in (1..).zip(wanted) {
            match svc.register(id, chord, repeat) {
                Ok(()) => {
                    self.actions.insert(id, action);
                }
                Err(e) => {
                    tracing::warn!(%chord, error = %e, "hotkey unavailable");
                    let notice = Notice::HotkeyConflict { binding_id: name, keys: chord.to_string() };
                    self.hotkey_conflicts.push(notice.clone());
                    self.notice(notice);
                }
            }
        }
        self.hotkeys = Some(svc);
    }

    fn hotkey(&mut self, id: i32) {
        let Some(action) = self.actions.get(&id).cloned() else { return };
        match action {
            HotkeyAction::Binding(binding_id) => self.command(Command::SwitchBinding { binding_id }),
            HotkeyAction::Pair(pair_id) => self.command(Command::TogglePair { pair_id }),
            HotkeyAction::CycleFavourites => self.cycle(Flow::Render, 1),
            HotkeyAction::KnobCycle(step) => {
                // Alt+Tab style: one switch per gesture, after the knob rests (F31; fast spins = 13–40 ms/detent).
                if self.knob_at.is_none() {
                    self.knob_first = step.signum();
                }
                self.knob_steps += step;
                self.knob_at =
                    Some(Instant::now() + Duration::from_millis(u64::from(self.config.knob.commit_after_ms)));
            }
            HotkeyAction::KnobFine(delta) => self.command(Command::StepVolume { delta }),
            HotkeyAction::KnobMic(delta) => {
                if let Some(dev) = self.mic_device()
                    && let Ok((scalar, muted)) = volume::get(&dev)
                {
                    let target = (scalar + delta).clamp(0.0, 1.0);
                    if volume::set_scalar(&dev, target).is_ok() {
                        self.on_mic(target, muted); // immediate feedback; the watch callback dedupes
                    }
                }
            }
        }
    }

    // ---- commands ---------------------------------------------------------------------------------

    fn command(&mut self, command: Command) {
        match command {
            Command::SwitchTo { device_id, roles } => {
                match self.endpoints.iter().find(|e| e.id.eq_ignore_ascii_case(&device_id)).cloned() {
                    Some(target) => self.switch(&target, roles, None),
                    None => self.notice(Notice::NotConnected { name: device_id }),
                }
            }
            Command::SwitchBinding { binding_id } => {
                let Some(b) = self.config.bindings.iter().find(|b| b.id == binding_id) else { return };
                let (dref, roles) = (b.device.clone(), b.roles.unwrap_or(self.config.default_roles));
                let res = Owned::from(resolve::resolve(&dref, &self.endpoints));
                self.follow(&dref, res, roles);
            }
            Command::TogglePair { pair_id } => {
                let Some(p) = self.config.toggle_pairs.iter().find(|p| p.id == pair_id) else { return };
                let roles = p.roles.unwrap_or(self.config.default_roles);
                let pair = [p.primary.clone(), p.secondary.clone()];
                let current = self.default_id(pair[0].flow, Role::Multimedia);
                let (side, res) = match resolve::toggle([&pair[0], &pair[1]], current.as_deref(), &self.endpoints) {
                    ToggleAction::SwitchTo { side, resolution } | ToggleAction::Unavailable { side, resolution } => {
                        (side, Owned::from(resolution))
                    }
                };
                self.follow(&pair[side], res, roles);
            }
            Command::CycleFavourites { flow, step } => self.cycle(flow, i32::from(step)),
            Command::SetVolume { percent } => self.with_default_volume(|dev, _, _| {
                let _ = volume::set_scalar(dev, f32::from(percent) / 100.0);
            }),
            Command::StepVolume { delta } => self.with_default_volume(|dev, scalar, _| {
                let _ = volume::set_scalar(dev, scalar + f32::from(delta) / 100.0);
            }),
            Command::ToggleMute => self.with_default_volume(|dev, _, muted| {
                let _ = volume::set_mute(dev, !muted);
            }),
            Command::ToggleMicMute => {
                if let Some(dev) = self.mic_device()
                    && let Ok((scalar, muted)) = volume::get(&dev)
                    && volume::set_mute(&dev, !muted).is_ok()
                {
                    self.on_mic(scalar, !muted);
                }
            }
            Command::SetSpatial { device_id, mode } => {
                if let Some(target) = self.endpoints.iter().find(|e| e.id.eq_ignore_ascii_case(&device_id)).cloned() {
                    self.apply_spatial(&target, mode);
                    self.refresh(true);
                }
            }
            Command::ResolveChoice { request_id, device_id } => {
                let Some(p) = self.pending.remove(&request_id) else { return };
                let target =
                    device_id.and_then(|id| self.endpoints.iter().find(|e| e.id.eq_ignore_ascii_case(&id)).cloned());
                if let Some(target) = target {
                    // The user confirmed this device, so the re-link is persisted after the switch.
                    self.switch(&target, p.roles, Some(p.dref));
                }
            }
            Command::Refresh => self.refresh(true),
            Command::Media { action } => match &self.media {
                Some(m) => m.send(action),
                None => tracing::debug!(?action, "media command ignored: now playing is unavailable"),
            },
            Command::SetTogglePair { .. }
            | Command::RemoveTogglePair { .. }
            | Command::SetBinding { .. }
            | Command::RemoveBinding { .. }
            | Command::SetFavourites { .. }
            | Command::SetDevicePrefs { .. }
            | Command::SetPerformanceProfile { .. }
            | Command::SetPinnedMonitor { .. }
            | Command::SetKnobGestures { .. }
            | Command::SetDefaultRoles { .. } => self.edit_settings(&command),
        }
    }

    fn with_default_volume(&self, f: impl FnOnce(&IMMDevice, f32, bool)) {
        if let Ok(dev) = self.eps.default_device(Flow::Render, Role::Multimedia)
            && let Ok((scalar, muted)) = volume::get(&dev)
        {
            f(&dev, scalar, muted);
        }
    }

    /// Acts on a resolution of a saved reference.
    fn follow(&mut self, dref: &DeviceRef, res: Owned, roles: RoleSet) {
        match res {
            Owned::Exact(t) => self.switch(&t, roles, None),
            Owned::Relinked(t) => self.switch(&t, roles, Some(dref.clone())),
            Owned::Choose => {
                // Same rules as the resolver (review ENG-06).
                let candidates: Vec<DeviceSummary> =
                    resolve::candidates(dref, &self.endpoints).into_iter().map(|e| self.summary_basic(e)).collect();
                let request_id = self.next_request;
                self.next_request += 1;
                self.pending.insert(request_id, PendingChoice { dref: dref.clone(), roles });
                while self.pending.len() > MAX_PENDING_CHOICES {
                    self.pending.pop_first();
                }
                self.notice(Notice::ChooseDevice { request_id, name: dref.display_name.clone(), candidates });
            }
            Owned::NotConnected => self.notice(Notice::NotConnected { name: dref.display_name.clone() }),
        }
    }

    fn cycle(&mut self, flow: Flow, steps: i32) {
        let refs = match flow {
            Flow::Render => &self.config.favourites.playback,
            Flow::Capture => &self.config.favourites.recording,
        };
        let mut targets: Vec<EndpointInfo> =
            refs.iter().filter_map(|r| resolve::resolve(r, &self.endpoints).target().cloned()).collect();
        if targets.is_empty() {
            targets = self.endpoints.iter().filter(|e| e.flow == flow).cloned().collect();
        }
        targets.dedup_by(|a, b| a.id == b.id);
        if targets.is_empty() {
            return;
        }
        let n = i32::try_from(targets.len()).unwrap_or(1);
        let current = self.default_id(flow, Role::Multimedia);
        let cur =
            targets.iter().position(|t| Some(&t.id) == current.as_ref()).map_or(-1, |i| i32::try_from(i).unwrap_or(0));
        let next = usize::try_from((cur + steps).rem_euclid(n)).unwrap_or(0);
        let target = targets[next].clone();
        if Some(&target.id) == current.as_ref() {
            return; // already there (e.g. only one favourite connected)
        }
        self.switch(&target, self.config.default_roles, None);
    }

    /// Switches, confirms by re-reading the defaults, persists a re-link only after confirmation (D-16),
    /// then applies the device's spatial preference (D-17).
    /// `relink_from`: the saved reference that followed the device to `target`, if this is a re-link.
    fn switch(&mut self, target: &EndpointInfo, roles: RoleSet, relink_from: Option<DeviceRef>) {
        let name = self.summary_basic(target).name;
        let wanted = roles.roles();
        if let Err(e) = policy::set_default(&target.id, wanted) {
            tracing::warn!(error = %e, device = %name, "IPolicyConfig unavailable");
            self.notice(Notice::SwitchFailed { name, failed_roles: wanted.to_vec() });
            return;
        }
        let defaults = self.eps.defaults();
        let failed: Vec<Role> = wanted
            .iter()
            .copied()
            .filter(|r| !defaults.iter().any(|(f, role, id)| *f == target.flow && role == r && *id == target.id))
            .collect();
        if !failed.is_empty() {
            let all_failed = failed.len() == wanted.len();
            self.notice(Notice::SwitchFailed { name: name.clone(), failed_roles: failed });
            if all_failed {
                // Binding preserved: nothing is re-linked to a device we couldn't switch to.
                self.refresh(true);
                return;
            }
        }
        if let Some(old) = &relink_from {
            self.persist_relink(old, target);
        }
        let spatial_applied = self
            .config
            .prefs_for(target)
            .map(|p| p.spatial)
            .filter(|m| *m != SpatialMode::Keep)
            .and_then(|mode| self.apply_spatial(target, mode));
        self.refresh(true);
        let device = {
            let sh = lock(&self.shared);
            sh.snapshot.playback.iter().chain(&sh.snapshot.recording).find(|d| d.id == target.id).cloned()
        }
        .unwrap_or_else(|| self.summary_basic(target));
        self.emit(EngineEvent::Switched { device, roles, relinked: relink_from.is_some(), spatial_applied });
    }

    /// Read-before-write, bounded write, read back. Returns the mode only when Windows confirms it (D-17).
    fn apply_spatial(&mut self, target: &EndpointInfo, mode: SpatialMode) -> Option<SpatialMode> {
        let wanted = format_of(mode)?;
        if target.flow != Flow::Render {
            return None;
        }
        let selected = |id: &str| spatial::selected(id).ok().map(|s| s.and_then(|(f, _)| f));
        let is_wanted = |sel: Option<Option<SpatialFormat>>| sel == Some(wanted);
        if is_wanted(selected(&target.id)) {
            return Some(mode);
        }
        let status = spatial::set_default_within(&target.id, wanted, SPATIAL_WRITE_LIMIT);
        if is_wanted(selected(&target.id)) {
            return Some(mode);
        }
        let reason = match status {
            Ok(s) => s.to_string(),
            Err(e) => e,
        };
        let name = self.summary_basic(target).name;
        self.notice(Notice::SpatialUnavailable { name, mode, reason });
        None
    }
}

fn percent(scalar: f32) -> u8 {
    // Clamped to 0–100 first, so the cast can't truncate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let p = (scalar.clamp(0.0, 1.0) * 100.0).round() as u8;
    p
}

/// `None` = leave Windows alone; `Some(None)` = off; `Some(Some(f))` = that format.
fn format_of(mode: SpatialMode) -> Option<Option<SpatialFormat>> {
    Some(match mode {
        SpatialMode::Keep => return None,
        SpatialMode::Off => None,
        SpatialMode::WindowsSonic => Some(SpatialFormat::WindowsSonic),
        SpatialMode::DolbyAtmosHeadphones => Some(SpatialFormat::DolbyAtmosHeadphones),
        SpatialMode::DolbyAtmosHomeTheater => Some(SpatialFormat::DolbyAtmosHomeTheater),
        SpatialMode::DolbyAtmosSpeakers => Some(SpatialFormat::DolbyAtmosSpeakers),
        SpatialMode::DtsHeadphoneX => Some(SpatialFormat::DtsHeadphoneX),
        SpatialMode::DtsXUltra => Some(SpatialFormat::DtsXUltra),
        SpatialMode::DtsXHomeTheater => Some(SpatialFormat::DtsXHomeTheater),
    })
}

/// Active spatial state → UI mode. `None` = an unknown provider.
fn mode_from(active: Option<(Option<SpatialFormat>, String)>) -> Option<SpatialMode> {
    match active {
        None => Some(SpatialMode::Off),
        Some((Some(f), _)) => Some(match f {
            SpatialFormat::WindowsSonic => SpatialMode::WindowsSonic,
            SpatialFormat::DolbyAtmosHeadphones => SpatialMode::DolbyAtmosHeadphones,
            SpatialFormat::DolbyAtmosHomeTheater => SpatialMode::DolbyAtmosHomeTheater,
            SpatialFormat::DolbyAtmosSpeakers => SpatialMode::DolbyAtmosSpeakers,
            SpatialFormat::DtsHeadphoneX => SpatialMode::DtsHeadphoneX,
            SpatialFormat::DtsXUltra => SpatialMode::DtsXUltra,
            SpatialFormat::DtsXHomeTheater => SpatialMode::DtsXHomeTheater,
        }),
        Some((None, _)) => None,
    }
}

/// F31 chords: `<modifier>+VolumeUp/Down` for each enabled gesture.
fn knob_chords(k: &KnobGestures) -> Vec<(Chord, HotkeyAction, String, bool)> {
    if !k.enabled {
        return Vec::new();
    }
    let fine = i8::try_from(k.fine_step_percent).unwrap_or(1);
    let gestures: [(&Option<String>, HotkeyAction, HotkeyAction); 3] = [
        (&k.cycle_device_modifier, HotkeyAction::KnobCycle(1), HotkeyAction::KnobCycle(-1)),
        (&k.fine_volume_modifier, HotkeyAction::KnobFine(fine), HotkeyAction::KnobFine(-fine)),
        (&k.mic_volume_modifier, HotkeyAction::KnobMic(MIC_STEP), HotkeyAction::KnobMic(-MIC_STEP)),
    ];
    let mut out = Vec::new();
    for (modifier, up, down) in gestures {
        let Some(m) = modifier else { continue };
        for (key, action) in [("VolumeUp", up), ("VolumeDown", down)] {
            if let Ok(chord) = format!("{m}+{key}").parse::<Chord>() {
                out.push((chord, action, format!("knob {m}+{key}"), true));
            }
        }
    }
    out
}
