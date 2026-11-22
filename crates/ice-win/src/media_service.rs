//! The engine's `ice-media` thread: owns GSMTC (its own MTA), coalesces change signals, and hands finished
//! reads to the engine. Kept off the `ice-audio` thread so a slow media app can never delay a device switch.
//!
//! Signals set bits in a dirty mask and wake the thread at most once per burst (review MED-07); a track change
//! fires properties + playback + timeline together, so the thread waits [`SETTLE`] and then reads once.
//! Track titles are only ever logged at `debug` (code-practices §4).

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ice_core::engine::MediaAction;
use ice_core::media::NowPlaying;

use crate::com::ComGuard;
use crate::media::{self, Art, MediaSignal, MediaWatcher};

/// Quiet period after the first signal of a burst before reading.
const SETTLE: Duration = Duration::from_millis(60);
/// Longer side of the downscaled art.
const ART_PX: u32 = 128;

/// A finished read for the engine.
#[derive(Debug, Clone)]
pub struct MediaUpdate {
    pub media: Option<NowPlaying>,
    /// The PNG, only when the art is new; the engine caches it for [`ice_core::engine::Engine::art`].
    pub art: Option<Art>,
}

enum In {
    Wake,
    Action(MediaAction),
    Quit,
}

/// Handle to the media thread. Dropping it stops the thread and unsubscribes everything.
pub struct MediaService {
    tx: Sender<In>,
    join: Option<JoinHandle<()>>,
}

impl MediaService {
    /// Starts the thread. `on_update` runs on it and must not block (e.g. a channel send).
    ///
    /// # Errors
    /// Fails only if the thread can't be spawned. A missing GSMTC is logged and leaves media empty.
    pub fn start(on_update: impl Fn(MediaUpdate) + Send + 'static) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let wake = tx.clone();
        let join = std::thread::Builder::new().name("ice-media".into()).spawn(move || run(&rx, wake, &on_update))?;
        Ok(Self { tx, join: Some(join) })
    }

    /// Sends a transport command to the current session (asynchronously; the result shows up as an update).
    pub fn send(&self, action: MediaAction) {
        let _ = self.tx.send(In::Action(action));
    }
}

impl Drop for MediaService {
    fn drop(&mut self) {
        let _ = self.tx.send(In::Quit);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

const SESSION_BITS: u8 = MediaSignal::CurrentSessionChanged.bit() | MediaSignal::SessionsChanged.bit();
const PLAYBACK_BITS: u8 = MediaSignal::PlaybackChanged.bit() | MediaSignal::TimelineChanged.bit();

fn run(rx: &Receiver<In>, wake: Sender<In>, out: &dyn Fn(MediaUpdate)) {
    let _com = match ComGuard::init_mta() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "media thread: COM unavailable; now playing disabled");
            return;
        }
    };
    let dirty = Arc::new(AtomicU8::new(0));
    let watcher = media::manager().and_then(|mgr| {
        let dirty = Arc::clone(&dirty);
        MediaWatcher::start(mgr, move |s| {
            // Only the first signal of a burst wakes the thread; the rest just add bits.
            if dirty.fetch_or(s.bit(), Ordering::AcqRel) == 0 {
                let _ = wake.send(In::Wake);
            }
        })
    });
    let mut watcher = match watcher {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!(error = %e, "media controls unavailable; now playing disabled");
            return;
        }
    };
    let mut state = State::default();
    state.apply(SESSION_BITS, &mut watcher, true, out);
    let mut settle_at: Option<Instant> = None;
    loop {
        let msg = match settle_at {
            Some(t) => match rx.recv_timeout(t.saturating_duration_since(Instant::now())) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => break,
            },
        };
        match msg {
            Some(In::Quit) => break,
            Some(In::Action(action)) => match media::send(watcher.manager(), action) {
                Ok(true) => {}
                Ok(false) => tracing::debug!(?action, "media app refused the command"),
                Err(e) => tracing::debug!(?action, error = %e, "media command failed"),
            },
            Some(In::Wake) => {
                settle_at.get_or_insert_with(|| Instant::now() + SETTLE);
            }
            None => {
                settle_at = None;
                let bits = dirty.swap(0, Ordering::AcqRel);
                state.apply(bits, &mut watcher, false, out);
            }
        }
    }
}

#[derive(Default)]
struct State {
    last: Option<NowPlaying>,
    /// The art id the engine already has the PNG for.
    sent_art: Option<String>,
}

impl State {
    fn apply(&mut self, bits: u8, watcher: &mut MediaWatcher, first: bool, out: &dyn Fn(MediaUpdate)) {
        let session_changed = bits & SESSION_BITS != 0 && watcher.follow_current();
        let full = first || session_changed || bits & MediaSignal::PropertiesChanged.bit() != 0;
        if !full && bits & PLAYBACK_BITS == 0 {
            return; // e.g. another app's session opened or closed
        }
        let session = watcher.manager().GetCurrentSession().ok();
        let (media, art) = match session {
            None => (None, None),
            Some(s) => match self.last.clone().filter(|_| !full) {
                // Only status/timeline changed: cheap local reads, track and art kept.
                Some(mut now) => match media::refresh_playback(&s, &mut now) {
                    Ok(()) => (Some(now), None),
                    Err(e) => {
                        tracing::debug!(error = %e, "playback read failed; keeping the last state");
                        return;
                    }
                },
                // Art is re-read on every properties change: some apps send the new title first and the new
                // thumbnail in a second change, so "same title" doesn't mean "same art".
                None => match media::read(&s, Some(ART_PX)) {
                    Ok((now, art)) => (Some(now), art),
                    Err(e) => {
                        tracing::debug!(error = %e, "media read failed; keeping the last state");
                        return;
                    }
                },
            },
        };
        let art = art.filter(|a| self.sent_art.as_deref() != Some(a.info.id.as_str()));
        if media == self.last && art.is_none() {
            return;
        }
        if let Some(a) = &art {
            self.sent_art = Some(a.info.id.clone());
        }
        if let Some(n) = &media {
            tracing::debug!(app = %n.app_id, title = %n.title, status = ?n.status, "now playing");
        }
        self.last.clone_from(&media);
        out(MediaUpdate { media, art });
    }
}
