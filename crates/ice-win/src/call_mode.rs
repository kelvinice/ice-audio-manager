//! D-30 call mode: lowers the now-playing app while people talk on a call, and brings it back after a quiet stretch.
//!
//! Runs on its own `ice-call` thread, which exists only while call mode is on (stop condition: turning it off or
//! quitting drops [`CallModeService`], which restores every lowered app and joins the thread). On that thread:
//! - **Idle** (no call app playing): blocks on its channel. Session "created" and "state changed" notifications wake it
//!   to re-scan; there is no timer.
//! - **In a call** (a call app's session is active, or the music is still lowered): reads the call apps' session peaks
//!   every [`CALL_TICK`] (~15 Hz) into the pure [`Ducker`], and writes the resulting gain to the music app's session.
//!
//! Safety nets: the music's own level is saved to `%LOCALAPPDATA%\IceAudioManager\call-mode-restore.json` before it
//! is first lowered in a call (once per call, not per sentence) and put back on the next start if the app died while
//! the music was low. If the user moves the music's volume while it's lowered, theirs wins for the rest of that dip.

mod sessions;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Instant;

use ice_core::call_mode::{CALL_TICK, CallModeConfig, Ducker, app_id_matches_exe};
use serde::{Deserialize, Serialize};

use crate::com::ComGuard;
use crate::endpoints::Endpoints;
use sessions::{Session, SessionWatch};

/// A written volume that reads back further away than this was changed by someone else (the user).
const USER_CHANGE_EPS: f32 = 0.01;

enum Msg {
    Config(CallModeConfig),
    NowPlaying(Option<String>),
    /// A session was created, started or stopped playing, or went away.
    Sessions,
    /// Playback devices changed: re-register on the new set.
    Devices,
    Quit,
}

/// Handle to the `ice-call` thread. Dropping it restores the music and stops the thread.
pub struct CallModeService {
    tx: Sender<Msg>,
    join: Option<JoinHandle<()>>,
}

impl CallModeService {
    /// Starts the thread (call mode is on).
    ///
    /// # Errors
    /// If the thread can't be spawned.
    pub fn start(config: CallModeConfig, now_playing: Option<String>) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let thread_tx = tx.clone();
        let join = std::thread::Builder::new()
            .name("ice-call".into())
            .spawn(move || run(config, now_playing, &thread_tx, &rx))?;
        Ok(Self { tx, join: Some(join) })
    }

    pub fn set_config(&self, config: CallModeConfig) {
        let _ = self.tx.send(Msg::Config(config));
    }

    /// The GSMTC now-playing app id (`Spotify.exe`), whose session is the one lowered.
    pub fn set_now_playing(&self, app_id: Option<String>) {
        let _ = self.tx.send(Msg::NowPlaying(app_id));
    }

    pub fn devices_changed(&self) {
        let _ = self.tx.send(Msg::Devices);
    }
}

impl Drop for CallModeService {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Quit);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn run(config: CallModeConfig, now_playing: Option<String>, tx: &Sender<Msg>, rx: &Receiver<Msg>) {
    // Declared first so it drops last, after every COM object in `State`.
    let com = ComGuard::init_mta();
    let Some(eps) = com.as_ref().ok().and_then(|_| Endpoints::new().ok()) else {
        tracing::warn!("call mode could not reach the audio service; it stays inactive");
        return;
    };
    let sink_tx = tx.clone();
    let sink: sessions::Sink = Arc::new(move || {
        let _ = sink_tx.send(Msg::Sessions);
    });
    let mut st = State {
        ducker: Ducker::new(&config),
        config,
        now_playing,
        watch: SessionWatch::new(&eps, Arc::clone(&sink)),
        eps,
        sink,
        call_active: false,
        calls: Vec::new(),
        dip: Vec::new(),
        dipping: false,
        restore: RestoreFile::new(),
        last_tick: Instant::now(),
    };
    st.restore_after_crash();
    st.rescan();
    tracing::debug!("call mode on");
    'run: loop {
        let first = if st.ticking() {
            match rx.recv_timeout(CALL_TICK.saturating_sub(st.last_tick.elapsed())) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => break,
            }
        };
        // Take everything already queued: a burst of notifications costs one re-scan, and messages can never hold
        // the level check back (before, a steady stream of messages starved the tick and froze a fade mid-way).
        let mut rescan = false;
        for msg in first.into_iter().chain(std::iter::from_fn(|| rx.try_recv().ok())) {
            match msg {
                Msg::Quit => break 'run,
                Msg::Config(c) => rescan |= st.configure(c),
                Msg::NowPlaying(id) => st.set_now_playing(id),
                Msg::Sessions => rescan = true,
                Msg::Devices => {
                    st.release_dip();
                    st.watch = SessionWatch::new(&st.eps, Arc::clone(&st.sink));
                    rescan = true;
                }
            }
        }
        if rescan {
            st.rescan();
        }
        if st.ticking() && st.last_tick.elapsed() >= CALL_TICK {
            st.tick();
        }
    }
    st.end_dip();
    st.restore.clear();
    tracing::debug!("call mode off");
}

/// A music session we lowered, with the level to put back.
struct Lowered {
    session: Session,
    base: f32,
    written: f32,
}

struct State {
    config: CallModeConfig,
    now_playing: Option<String>,
    ducker: Ducker,
    watch: SessionWatch,
    eps: Endpoints,
    sink: sessions::Sink,
    /// A call app's session is playing right now.
    call_active: bool,
    calls: Vec<Session>,
    /// The music sessions lowered in the current dip.
    dip: Vec<Lowered>,
    /// A dip is under way (its sessions were looked up), until the music is back at full level.
    dipping: bool,
    restore: RestoreFile,
    last_tick: Instant,
}

impl State {
    /// Polling runs only during a call or while the music is still lowered (§2 stop condition).
    fn ticking(&self) -> bool {
        self.call_active || !self.ducker.is_resting()
    }

    fn rescan(&mut self) {
        let config = &self.config;
        self.calls = self.watch.sessions().into_iter().filter(|s| config.is_call_app(&s.exe)).collect();
        for s in &self.calls {
            self.watch.watch(s);
        }
        let keys: Vec<&str> = self.calls.iter().map(|s| s.key.as_str()).collect();
        self.watch.retain_watched(&keys);
        let was_active = self.call_active;
        self.call_active = self.calls.iter().any(Session::is_active);
        if self.call_active && !was_active {
            self.last_tick = Instant::now();
            tracing::debug!("call mode: a call app is playing; listening");
        }
        if !self.call_active && self.ducker.is_resting() {
            self.restore.clear();
        }
    }

    /// Returns whether the call sessions must be re-scanned (the call-app list changed).
    fn configure(&mut self, config: CallModeConfig) -> bool {
        self.ducker.configure(&config);
        let apps_changed = config.apps != self.config.apps;
        self.config = config;
        if apps_changed {
            self.release_dip();
        }
        apps_changed
    }

    fn set_now_playing(&mut self, app_id: Option<String>) {
        if app_id != self.now_playing {
            // The lowered app is no longer the one playing: give it its level back. The ducker carries on, so if people
            // are still talking the next tick lowers the new app at the same level (no jump to full and back).
            self.release_dip();
            self.now_playing = app_id;
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        // "Is a call playing" is re-read on every tick, so a missed state notification can't keep this polling.
        let mut peak = 0.0_f32;
        let mut active = false;
        for s in self.calls.iter().filter(|s| s.is_active()) {
            active = true;
            peak = peak.max(s.peak());
        }
        self.call_active = active;
        let was_talking = self.ducker.talking();
        let gain = self.ducker.step(peak, dt);
        // A new dip, or new talk after an outside change took the music out of the current one.
        let onset = self.ducker.talking() && !was_talking;
        if gain < 1.0 && (!self.dipping || (onset && self.dip.is_empty())) {
            self.start_dip();
        }
        self.apply(gain);
        if self.ducker.is_resting() {
            self.dip.clear();
            self.dipping = false;
            if !self.call_active {
                self.restore.clear();
            }
        }
    }

    /// Finds the now-playing app's sessions (never a call app's) and remembers their level before lowering them.
    /// Runs once per dip, even if nothing matches, so a missing app isn't searched for on every tick.
    fn start_dip(&mut self) {
        self.dipping = true;
        let Some(app_id) = self.now_playing.as_deref() else { return };
        let config = &self.config;
        self.dip = self
            .watch
            .sessions()
            .into_iter()
            .filter(|s| app_id_matches_exe(app_id, &s.exe) && !config.is_call_app(&s.exe))
            .filter_map(|session| {
                let base = session.volume()?;
                Some(Lowered { session, base, written: base })
            })
            .collect();
        let entries: Vec<RestoreEntry> =
            self.dip.iter().map(|l| RestoreEntry { exe: l.session.exe.clone(), base: l.base }).collect();
        self.restore.save(&entries);
    }

    /// Writes `base × gain` to every lowered session. A session whose level was changed by someone else (the user,
    /// or another app such as Discord's Attenuation) leaves the dip and keeps that level; the next talk takes it in again.
    fn apply(&mut self, gain: f32) {
        let before = self.dip.len();
        self.dip.retain_mut(|l| {
            if let Some(now) = l.session.volume()
                && (now - l.written).abs() > USER_CHANGE_EPS
            {
                tracing::debug!(
                    now,
                    written = l.written,
                    "call mode: the music's level was changed elsewhere; leaving it"
                );
                return false;
            }
            let level = l.base * gain;
            if (level - l.written).abs() < f32::EPSILON {
                return true;
            }
            l.written = level;
            l.session.set_volume(level)
        });
        if self.dip.len() != before {
            // Nothing of ours to restore after a crash for the sessions that left.
            let entries: Vec<RestoreEntry> =
                self.dip.iter().map(|l| RestoreEntry { exe: l.session.exe.clone(), base: l.base }).collect();
            if entries.is_empty() { self.restore.clear() } else { self.restore.save(&entries) }
        }
    }

    /// Gives every lowered session its own level back at once, without touching the ducker (app or device change).
    fn release_dip(&mut self) {
        self.apply(1.0);
        self.dip.clear();
        self.dipping = false;
    }

    /// Releases the dip and resets the ducker (call mode off, quitting).
    fn end_dip(&mut self) {
        self.release_dip();
        self.ducker.reset();
    }

    /// After a crash or power loss while the music was lowered: raise those apps back to their saved level (only
    /// ever up, so a level the user set lower since then is kept).
    fn restore_after_crash(&mut self) {
        let entries = self.restore.load();
        if entries.is_empty() {
            return;
        }
        for s in self.watch.sessions() {
            if let Some(e) = entries.iter().find(|e| e.exe.eq_ignore_ascii_case(&s.exe))
                && s.volume().is_some_and(|v| v < e.base)
            {
                s.set_volume(e.base);
            }
        }
        tracing::info!(apps = entries.len(), "call mode: restored music levels left lowered by a previous run");
        self.restore.remove();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct RestoreEntry {
    exe: String,
    base: f32,
}

/// The pre-dip levels on disk. Written only when they differ from what's already saved, deleted after the call.
struct RestoreFile {
    path: Option<PathBuf>,
    saved: Option<Vec<RestoreEntry>>,
}

impl RestoreFile {
    fn new() -> Self {
        let path = std::env::var_os("LOCALAPPDATA")
            .map(|d| PathBuf::from(d).join("IceAudioManager").join("call-mode-restore.json"));
        Self { path, saved: None }
    }

    fn load(&self) -> Vec<RestoreEntry> {
        let Some(path) = &self.path else { return Vec::new() };
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    fn save(&mut self, entries: &[RestoreEntry]) {
        if entries.is_empty() || self.saved.as_deref() == Some(entries) {
            return;
        }
        let Some(path) = &self.path else { return };
        let Ok(json) = serde_json::to_vec(entries) else { return };
        let tmp = path.with_extension("json.tmp");
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&tmp, json))
            .and_then(|()| std::fs::rename(&tmp, path));
        match written {
            Ok(()) => self.saved = Some(entries.to_vec()),
            Err(e) => {
                tracing::warn!(error = %e, "call mode: couldn't save the music's level; a crash may leave it low")
            }
        }
    }

    /// Deletes the file if this run wrote it (the common case: the call ended and the music is back).
    fn clear(&mut self) {
        if self.saved.is_some() {
            self.remove();
        }
    }

    fn remove(&mut self) {
        self.saved = None;
        if let Some(path) = &self.path {
            let _ = std::fs::remove_file(path);
        }
    }
}
