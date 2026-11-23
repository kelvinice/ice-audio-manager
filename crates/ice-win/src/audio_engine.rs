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
                backup: backup.map_or_else(|| "(left in place, unchanged)".into(), |b| b.display().to_string()),
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
}

mod devices;
mod hotkeys;
mod settings;
mod switching;
