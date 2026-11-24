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
/// Re-reads after a track read, measured from it. Spotify (and some browsers) fire "properties changed" before
/// the new title is readable, so the first read can still return the previous track; nothing else re-reads
/// titles, so the notch showed the old song until some later event (owner report 2026-11-24: ~5 s). Three
/// bounded reads per track change, never at idle; an unchanged result emits nothing.
const CONFIRM_AFTER: [Duration; 3] =
    [Duration::from_millis(300), Duration::from_millis(1000), Duration::from_millis(2500)];
/// Art budget for the first read of a burst, so a cover that isn't ready can't hold back the new title
/// (normal reads take 30–60 ms); the confirm reads use the full [`media::ART_BUDGET`] and pick the cover up.
const FIRST_ART_BUDGET: Duration = Duration::from_millis(400);
/// A timeline duration change this large means a new track even without a properties event.
const DURATION_JUMP_MS: u64 = 1500;
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
    state.apply(SESSION_BITS, &mut watcher, true, media::ART_BUDGET, out);
    let mut settle_at: Option<Instant> = None;
    let mut confirm = Confirm::default();
    loop {
        let deadline = [settle_at, confirm.at].into_iter().flatten().min();
        let msg = match deadline {
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
                let now = Instant::now();
                if settle_at.is_some_and(|t| t <= now) {
                    settle_at = None;
                    let bits = dirty.swap(0, Ordering::AcqRel);
                    if state.apply(bits, &mut watcher, false, FIRST_ART_BUDGET, out) {
                        confirm.arm(now);
                    }
                } else if confirm.at.is_some_and(|t| t <= now) {
                    confirm.advance();
                    state.apply(MediaSignal::PropertiesChanged.bit(), &mut watcher, false, media::ART_BUDGET, out);
                }
            }
        }
    }
}

/// The pending [`CONFIRM_AFTER`] re-reads of the last track read.
#[derive(Default)]
struct Confirm {
    armed_at: Option<Instant>,
    step: usize,
    at: Option<Instant>,
}

impl Confirm {
    fn arm(&mut self, now: Instant) {
        self.armed_at = Some(now);
        self.step = 0;
        self.at = Some(now + CONFIRM_AFTER[0]);
    }

    fn advance(&mut self) {
        self.step += 1;
        self.at = self.armed_at.zip(CONFIRM_AFTER.get(self.step)).map(|(t, d)| t + *d);
    }
}

/// A new track can arrive with only playback/timeline events (or with a stale properties read): a jump in
/// duration gives it away.
fn duration_jumped(prev: &NowPlaying, next: &NowPlaying) -> bool {
    matches!((prev.duration_ms, next.duration_ms), (Some(a), Some(b)) if a.abs_diff(b) > DURATION_JUMP_MS)
}

#[derive(Default)]
struct State {
    last: Option<NowPlaying>,
    /// The art id the engine already has the PNG for.
    sent_art: Option<String>,
}

impl State {
    /// Reads and emits what changed. Returns `true` when it read the track (not just playback/timeline), so the
    /// caller can schedule the confirm reads.
    fn apply(
        &mut self,
        bits: u8,
        watcher: &mut MediaWatcher,
        first: bool,
        art_budget: Duration,
        out: &dyn Fn(MediaUpdate),
    ) -> bool {
        let session_changed = bits & SESSION_BITS != 0 && watcher.follow_current();
        let mut full = first || session_changed || bits & MediaSignal::PropertiesChanged.bit() != 0;
        if !full && bits & PLAYBACK_BITS == 0 {
            return false; // e.g. another app's session opened or closed
        }
        let session = watcher.manager().GetCurrentSession().ok();
        // Only status/timeline changed: cheap local reads, track and art kept, unless the duration jumped.
        let mut quick = None;
        if let (Some(s), Some(last), false) = (&session, &self.last, full) {
            let mut now = last.clone();
            if let Err(e) = media::refresh_playback(s, &mut now) {
                tracing::debug!(error = %e, "playback read failed; keeping the last state");
                return false;
            }
            if duration_jumped(last, &now) {
                full = true;
            } else {
                quick = Some(now);
            }
        }
        let (media, art) = match (session, quick) {
            (None, _) => (None, None),
            (Some(_), Some(now)) => (Some(now), None),
            // Art is re-read on every track read: some apps send the new title first and the new thumbnail in
            // a second change, so "same title" doesn't mean "same art".
            (Some(s), None) => match media::read_within(&s, Some(ART_PX), art_budget) {
                Ok((now, art)) => (Some(now), art),
                Err(e) => {
                    tracing::debug!(error = %e, "media read failed; keeping the last state");
                    return full;
                }
            },
        };
        let art = art.filter(|a| self.sent_art.as_deref() != Some(a.info.id.as_str()));
        if media == self.last && art.is_none() {
            return full;
        }
        if let Some(a) = &art {
            self.sent_art = Some(a.info.id.clone());
        }
        if let Some(n) = &media {
            tracing::debug!(app = %n.app_id, title = %n.title, status = ?n.status, "now playing");
        }
        self.last.clone_from(&media);
        out(MediaUpdate { media, art });
        full
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ice_core::media::PlaybackStatus;

    fn track(duration_ms: Option<u64>) -> NowPlaying {
        NowPlaying {
            app_id: "Spotify.exe".into(),
            title: "A".into(),
            artist: "B".into(),
            album: String::new(),
            status: PlaybackStatus::Playing,
            position_ms: Some(0),
            duration_ms,
            art: None,
        }
    }

    #[test]
    fn a_duration_jump_means_a_new_track() {
        assert!(duration_jumped(&track(Some(200_000)), &track(Some(260_000))));
        assert!(!duration_jumped(&track(Some(200_000)), &track(Some(200_900))));
        assert!(!duration_jumped(&track(None), &track(Some(200_000))));
    }

    #[test]
    fn confirm_reads_run_once_each_then_stop() {
        let t0 = Instant::now();
        let mut c = Confirm::default();
        assert_eq!(c.at, None);
        c.arm(t0);
        let mut seen = Vec::new();
        while let Some(at) = c.at {
            seen.push(at - t0);
            c.advance();
        }
        assert_eq!(seen, CONFIRM_AFTER.to_vec());
    }
}
