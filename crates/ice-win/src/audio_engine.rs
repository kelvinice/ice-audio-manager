//! The real engine (plan.md track A): implements [`ice_core::engine::Engine`] on Core Audio.
//!
//! One `ice-audio` thread owns every COM object (MTA). COM callbacks and hotkeys only forward
//! messages to it. Device-change bursts (one unplug = state + 3 default changes + dozens of property
//! changes) are coalesced into one refresh after a short quiet period.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ice_core::config::{Config, ConfigStore, KnobGestures, Loaded};
use ice_core::device::{EndpointInfo, Flow, Role};
use ice_core::engine::{
    Command, DeviceSummary, Engine, EngineEvent, EngineSnapshot, Envelope, Notice, RoleSet, SpatialMode, VolumeState,
};
use ice_core::event::AudioEvent;
use ice_core::hotkey::Chord;
use ice_core::resolve::{self, DeviceRef, Resolution, ToggleAction, normalize_name};

use crate::com::ComGuard;
use crate::endpoints::Endpoints;
use crate::hotkeys::HotkeyService;
use crate::spatial::{self, SpatialFormat};
use crate::{notify, policy, volume};

/// Quiet period before a device-change burst is turned into one refresh.
const REFRESH_DEBOUNCE: Duration = Duration::from_millis(120);
/// Volume step for Alt+knob (mic volume), in percent.
const MIC_STEP: f32 = 0.02;

enum Msg {
    Command(Command),
    Audio(AudioEvent),
    Hotkey(i32),
    Quit,
}

struct Shared {
    snapshot: EngineSnapshot,
    subscribers: Vec<Sender<Envelope>>,
    /// Notices raised before anyone subscribed (hotkey conflicts, broken config); replayed to new subscribers.
    startup: Vec<Envelope>,
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
            startup: Vec::new(),
        }));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let (worker_shared, worker_tx) = (Arc::clone(&shared), tx.clone());
        let join = std::thread::Builder::new()
            .name("ice-audio".into())
            .spawn(move || match Worker::new(store, worker_shared, &worker_tx) {
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

    fn subscribe(&self) -> Receiver<Envelope> {
        let (tx, rx) = mpsc::channel();
        let mut sh = lock(&self.shared);
        for env in &sh.startup {
            let _ = tx.send(env.clone());
        }
        sh.subscribers.push(tx);
        rx
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
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

/// Where a saved [`DeviceRef`] lives, so a confirmed re-link can be persisted.
#[derive(Debug, Clone, Copy)]
enum Loc {
    Binding(usize),
    Pair(usize, usize),
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

struct PendingChoice {
    loc: Option<Loc>,
    roles: RoleSet,
}

struct Worker {
    eps: Endpoints,
    endpoints: Vec<EndpointInfo>,
    store: ConfigStore,
    config: Config,
    shared: Arc<Mutex<Shared>>,
    revision: u64,
    audio_tx: Sender<AudioEvent>,
    volume_watch: Option<volume::VolumeWatch>,
    _notify: notify::Registration,
    hotkeys: Option<HotkeyService>,
    actions: HashMap<i32, HotkeyAction>,
    pending: HashMap<u64, PendingChoice>,
    next_request: u64,
    refresh_at: Option<Instant>,
    knob_steps: i32,
    knob_at: Option<Instant>,
    // Must drop last: every COM object above is released before COM is uninitialised.
    _com: ComGuard,
}

/// Forwards one channel into the engine's message channel on a tiny thread (ends when the sender drops).
fn forward<T: Send + 'static>(engine: &Sender<Msg>, wrap: fn(T) -> Msg) -> Sender<T> {
    let (tx, rx) = mpsc::channel::<T>();
    let engine = engine.clone();
    let _ = std::thread::Builder::new().name("ice-forward".into()).spawn(move || {
        for item in rx {
            if engine.send(wrap(item)).is_err() {
                break;
            }
        }
    });
    tx
}

impl Worker {
    fn new(store: ConfigStore, shared: Arc<Mutex<Shared>>, engine_tx: &Sender<Msg>) -> Result<Self, String> {
        let com = ComGuard::init_mta().map_err(|e| format!("COM: {e}"))?;
        let eps = Endpoints::new().map_err(|e| format!("audio service: {e}"))?;
        let audio_tx = forward(engine_tx, Msg::Audio);
        let registration = notify::register(eps.raw(), audio_tx.clone()).map_err(|e| format!("notifications: {e}"))?;
        let loaded = store.load();
        let config = loaded.config().clone();
        let mut w = Self {
            eps,
            endpoints: Vec::new(),
            store,
            config,
            shared,
            revision: 0,
            audio_tx,
            volume_watch: None,
            _notify: registration,
            hotkeys: None,
            actions: HashMap::new(),
            pending: HashMap::new(),
            next_request: 1,
            refresh_at: None,
            knob_steps: 0,
            knob_at: None,
            _com: com,
        };
        if let Loaded::Replaced { backup, error, .. } = loaded {
            w.startup_notice(Notice::ConfigReplaced { backup: backup.display().to_string(), error });
        }
        w.refresh(false);
        w.register_hotkeys(engine_tx);
        Ok(w)
    }

    fn run(&mut self, rx: &Receiver<Msg>) {
        loop {
            let deadline = [self.refresh_at, self.knob_at].into_iter().flatten().min();
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
                Msg::Hotkey(id) => self.hotkey(id),
            }
        }
    }

    fn on_deadlines(&mut self) {
        let now = Instant::now();
        if self.refresh_at.is_some_and(|t| t <= now) {
            self.refresh_at = None;
            self.refresh(true);
        }
        if self.knob_at.is_some_and(|t| t <= now) {
            self.knob_at = None;
            let steps = std::mem::take(&mut self.knob_steps);
            if steps != 0 {
                self.cycle(Flow::Render, steps);
            }
        }
    }

    // ---- events out -------------------------------------------------------------------------------

    fn emit(&mut self, event: EngineEvent) {
        self.revision += 1;
        let env = Envelope { revision: self.revision, event };
        let mut sh = lock(&self.shared);
        sh.snapshot.revision = self.revision;
        sh.subscribers.retain(|tx| tx.send(env.clone()).is_ok());
    }

    fn notice(&mut self, notice: Notice) {
        self.emit(EngineEvent::Notice { notice });
    }

    fn startup_notice(&mut self, notice: Notice) {
        self.revision += 1;
        let env = Envelope { revision: self.revision, event: EngineEvent::Notice { notice } };
        let mut sh = lock(&self.shared);
        sh.snapshot.revision = self.revision;
        sh.startup.push(env);
    }

    // ---- device state -----------------------------------------------------------------------------

    fn summary(&self, e: &EndpointInfo) -> DeviceSummary {
        let prefs = self.config.prefs_for(e);
        let spatial =
            (e.flow == Flow::Render).then(|| spatial::status(&e.id).ok()).flatten().and_then(|s| match s.active {
                None => Some(SpatialMode::Off),
                Some((Some(f), _)) => Some(mode_of(f)),
                Some((None, _)) => None,
            });
        DeviceSummary {
            id: e.id.clone(),
            name: prefs
                .and_then(|p| p.alias.clone())
                .unwrap_or_else(|| normalize_name(e.friendly_name.as_deref().unwrap_or(&e.id))),
            flow: e.flow,
            state: e.state,
            is_default: e.default_for.contains(&Role::Console),
            is_default_communications: e.default_for.contains(&Role::Communications),
            form_factor: e.form_factor,
            is_virtual: e.hardware_key.as_deref().is_some_and(|k| k.starts_with("ROOT\\") || k.starts_with("SWD\\")),
            spatial,
        }
    }

    /// Re-enumerates, updates the snapshot and (when `notify`) emits arrivals, departures and `DevicesChanged`.
    fn refresh(&mut self, notify: bool) {
        let Ok(fresh) = self.eps.list(false) else { return };
        let old = std::mem::replace(&mut self.endpoints, fresh);
        let old_default = old
            .iter()
            .find(|e| e.flow == Flow::Render && e.default_for.contains(&Role::Multimedia))
            .map(|e| e.id.clone());
        let new_default = self.default_id(Flow::Render, Role::Multimedia);

        let visible = |w: &Self, flow: Flow| -> Vec<DeviceSummary> {
            w.endpoints
                .iter()
                .filter(|e| e.flow == flow && !w.config.prefs_for(e).is_some_and(|p| p.hidden))
                .map(|e| w.summary(e))
                .collect()
        };
        let (playback, recording) = (visible(self, Flow::Render), visible(self, Flow::Capture));
        let volume = self.default_volume();
        let mic_muted = self.mic_device().and_then(|d| volume::get(&d).ok()).map(|(_, m)| m);
        {
            let mut sh = lock(&self.shared);
            sh.snapshot.playback.clone_from(&playback);
            sh.snapshot.recording.clone_from(&recording);
            sh.snapshot.volume = volume;
            sh.snapshot.mic_muted = mic_muted;
        }
        if old_default != new_default || self.volume_watch.is_none() {
            self.volume_watch = self
                .eps
                .default_device(Flow::Render, Role::Multimedia)
                .ok()
                .and_then(|d| volume::VolumeWatch::new(&d, self.audio_tx.clone()).ok());
        }
        if !notify {
            return;
        }
        let arrived: Vec<DeviceSummary> =
            self.endpoints.iter().filter(|e| !old.iter().any(|o| o.id == e.id)).map(|e| self.summary(e)).collect();
        let left: Vec<DeviceSummary> = old
            .iter()
            .filter(|o| !self.endpoints.iter().any(|e| e.id == o.id))
            .map(|o| DeviceSummary { state: ice_core::device::DeviceState::NotPresent, ..self.summary(o) })
            .collect();
        for device in arrived {
            self.emit(EngineEvent::DeviceArrived { device });
        }
        for device in left {
            self.emit(EngineEvent::DeviceLeft { device });
        }
        self.emit(EngineEvent::DevicesChanged { playback, recording });
        if old_default != new_default
            && let Some(volume) = volume
        {
            self.emit(EngineEvent::VolumeChanged { volume });
        }
    }

    fn default_id(&self, flow: Flow, role: Role) -> Option<String> {
        self.endpoints.iter().find(|e| e.flow == flow && e.default_for.contains(&role)).map(|e| e.id.clone())
    }

    fn default_volume(&self) -> Option<VolumeState> {
        let dev = self.eps.default_device(Flow::Render, Role::Multimedia).ok()?;
        let (scalar, muted) = volume::get(&dev).ok()?;
        Some(VolumeState { percent: percent(scalar), muted })
    }

    fn mic_device(&self) -> Option<windows::Win32::Media::Audio::IMMDevice> {
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
                self.refresh_at.get_or_insert_with(|| Instant::now() + REFRESH_DEBOUNCE);
            }
        }
    }

    // ---- hotkeys ----------------------------------------------------------------------------------

    fn register_hotkeys(&mut self, engine_tx: &Sender<Msg>) {
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
        let Ok(svc) = HotkeyService::start(forward(engine_tx, Msg::Hotkey)) else { return };
        for (id, (chord, action, name, repeat)) in (1..).zip(wanted) {
            match svc.register(id, chord, repeat) {
                Ok(()) => {
                    self.actions.insert(id, action);
                }
                Err(_) => self.startup_notice(Notice::HotkeyConflict { binding_id: name, keys: chord.to_string() }),
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
                self.knob_steps += step;
                self.knob_at =
                    Some(Instant::now() + Duration::from_millis(u64::from(self.config.knob.commit_after_ms)));
            }
            HotkeyAction::KnobFine(delta) => self.command(Command::StepVolume { delta }),
            HotkeyAction::KnobMic(delta) => {
                if let Some(dev) = self.mic_device()
                    && let Ok((scalar, _)) = volume::get(&dev)
                {
                    let _ = volume::set_scalar(&dev, scalar + delta);
                }
            }
        }
    }

    // ---- commands ---------------------------------------------------------------------------------

    fn command(&mut self, command: Command) {
        match command {
            Command::SwitchTo { device_id, roles } => {
                match self.endpoints.iter().find(|e| e.id == device_id).cloned() {
                    Some(target) => self.switch(&target, roles, None),
                    None => self.notice(Notice::NotConnected { name: device_id }),
                }
            }
            Command::SwitchBinding { binding_id } => {
                let Some(i) = self.config.bindings.iter().position(|b| b.id == binding_id) else { return };
                let b = &self.config.bindings[i];
                let (dref, roles) = (b.device.clone(), b.roles.unwrap_or(self.config.default_roles));
                let res = Owned::from(resolve::resolve(&dref, &self.endpoints));
                self.follow(&dref, res, roles, Loc::Binding(i));
            }
            Command::TogglePair { pair_id } => {
                let Some(i) = self.config.toggle_pairs.iter().position(|p| p.id == pair_id) else { return };
                let p = &self.config.toggle_pairs[i];
                let roles = p.roles.unwrap_or(self.config.default_roles);
                let pair = [p.primary.clone(), p.secondary.clone()];
                let current = self.default_id(pair[0].flow, Role::Console);
                let (side, res) = match resolve::toggle([&pair[0], &pair[1]], current.as_deref(), &self.endpoints) {
                    ToggleAction::SwitchTo { side, resolution } | ToggleAction::Unavailable { side, resolution } => {
                        (side, Owned::from(resolution))
                    }
                };
                self.follow(&pair[side], res, roles, Loc::Pair(i, side));
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
                    && let Ok((_, muted)) = volume::get(&dev)
                    && volume::set_mute(&dev, !muted).is_ok()
                {
                    lock(&self.shared).snapshot.mic_muted = Some(!muted);
                    self.emit(EngineEvent::MicMuteChanged { muted: !muted });
                }
            }
            Command::SetSpatial { device_id, mode } => {
                if let Some(target) = self.endpoints.iter().find(|e| e.id == device_id).cloned() {
                    self.apply_spatial(&target, mode);
                    self.refresh(true);
                }
            }
            Command::ResolveChoice { request_id, device_id } => {
                let Some(p) = self.pending.remove(&request_id) else { return };
                if let Some(target) = device_id.and_then(|id| self.endpoints.iter().find(|e| e.id == id).cloned()) {
                    // The user confirmed this device, so a re-link is persisted after the switch.
                    self.switch_relinked(&target, p.roles, p.loc);
                }
            }
            Command::Refresh => self.refresh(true),
        }
    }

    fn with_default_volume(&self, f: impl FnOnce(&windows::Win32::Media::Audio::IMMDevice, f32, bool)) {
        if let Ok(dev) = self.eps.default_device(Flow::Render, Role::Multimedia)
            && let Ok((scalar, muted)) = volume::get(&dev)
        {
            f(&dev, scalar, muted);
        }
    }

    /// Acts on a resolution of a saved reference.
    fn follow(&mut self, dref: &DeviceRef, res: Owned, roles: RoleSet, loc: Loc) {
        match res {
            Owned::Exact(t) => self.switch(&t, roles, None),
            Owned::Relinked(t) => self.switch_relinked(&t, roles, Some(loc)),
            Owned::Choose => {
                let candidates: Vec<DeviceSummary> = self
                    .endpoints
                    .iter()
                    .filter(|e| e.flow == dref.flow)
                    .filter(|e| {
                        (dref.hardware_key.is_some() && e.hardware_key == dref.hardware_key)
                            || (dref.container_id.is_some() && e.container_id == dref.container_id)
                    })
                    .map(|e| self.summary(e))
                    .collect();
                let request_id = self.next_request;
                self.next_request += 1;
                self.pending.insert(request_id, PendingChoice { loc: Some(loc), roles });
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
        let current = self.default_id(flow, Role::Console);
        let cur =
            targets.iter().position(|t| Some(&t.id) == current.as_ref()).map_or(-1, |i| i32::try_from(i).unwrap_or(0));
        let next = usize::try_from((cur + steps).rem_euclid(n)).unwrap_or(0);
        let target = targets[next].clone();
        self.switch(&target, self.config.default_roles, None);
    }

    fn switch_relinked(&mut self, target: &EndpointInfo, roles: RoleSet, loc: Option<Loc>) {
        self.switch(target, roles, Some(loc));
    }

    /// Switches, confirms by re-reading the defaults, persists a re-link only after confirmation (D-16),
    /// then applies the device's spatial preference (D-17).
    /// `relink`: `None` = not a re-link; `Some(loc)` = the binding at `loc` followed the device to `target`.
    fn switch(&mut self, target: &EndpointInfo, roles: RoleSet, relink: Option<Option<Loc>>) {
        let name = self.summary(target).name;
        let wanted = roles.roles();
        if policy::set_default(&target.id, wanted).is_err() {
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
        if let Some(Some(loc)) = relink {
            self.persist_relink(loc, target);
        }
        let spatial_applied = self
            .config
            .prefs_for(target)
            .map(|p| p.spatial)
            .filter(|m| *m != SpatialMode::Keep)
            .and_then(|mode| self.apply_spatial(target, mode));
        self.refresh(true);
        let device =
            self.endpoints.iter().find(|e| e.id == target.id).map_or_else(|| self.summary(target), |e| self.summary(e));
        self.emit(EngineEvent::Switched { device, roles, relinked: relink.is_some(), spatial_applied });
    }

    fn persist_relink(&mut self, loc: Loc, target: &EndpointInfo) {
        let old = match loc {
            Loc::Binding(i) => self.config.bindings.get(i).map(|b| b.device.clone()),
            Loc::Pair(i, 0) => self.config.toggle_pairs.get(i).map(|p| p.primary.clone()),
            Loc::Pair(i, _) => self.config.toggle_pairs.get(i).map(|p| p.secondary.clone()),
        };
        let Some(old) = old else { return };
        let new = old.relinked_to(target);
        match loc {
            Loc::Binding(i) => self.config.bindings[i].device = new.clone(),
            Loc::Pair(i, 0) => self.config.toggle_pairs[i].primary = new.clone(),
            Loc::Pair(i, _) => self.config.toggle_pairs[i].secondary = new.clone(),
        }
        // Keep this device's preferences (spatial, alias) attached to the same device.
        for p in &mut self.config.devices {
            if p.device.endpoint_id == old.endpoint_id {
                p.device = p.device.relinked_to(target);
            }
        }
        let _ = self.store.save(&self.config);
    }

    /// Read-before-write, then read back. Returns the mode only when Windows confirms it (D-17).
    fn apply_spatial(&mut self, target: &EndpointInfo, mode: SpatialMode) -> Option<SpatialMode> {
        let wanted = format_of(mode)?;
        if target.flow != Flow::Render {
            return None;
        }
        let name = self.summary(target).name;
        let current = spatial::status(&target.id).ok()?.default.map(|(f, _)| f);
        if current == Some(wanted) || (current.is_none() && wanted.is_none()) {
            return Some(mode);
        }
        let status = spatial::set_default(&target.id, wanted);
        let confirmed = spatial::status(&target.id).ok().map(|s| s.default.and_then(|(f, _)| f)) == Some(wanted);
        if confirmed {
            Some(mode)
        } else {
            let reason = status.map_or_else(|e| e.message(), str::to_string);
            self.notice(Notice::SpatialUnavailable { name, mode, reason });
            None
        }
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

fn mode_of(f: SpatialFormat) -> SpatialMode {
    match f {
        SpatialFormat::WindowsSonic => SpatialMode::WindowsSonic,
        SpatialFormat::DolbyAtmosHeadphones => SpatialMode::DolbyAtmosHeadphones,
        SpatialFormat::DolbyAtmosHomeTheater => SpatialMode::DolbyAtmosHomeTheater,
        SpatialFormat::DolbyAtmosSpeakers => SpatialMode::DolbyAtmosSpeakers,
        SpatialFormat::DtsHeadphoneX => SpatialMode::DtsHeadphoneX,
        SpatialFormat::DtsXUltra => SpatialMode::DtsXUltra,
        SpatialFormat::DtsXHomeTheater => SpatialMode::DtsXHomeTheater,
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
