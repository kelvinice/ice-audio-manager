//! The engine's media workers: `ice-media` owns GSMTC observation/reads (its own MTA) and
//! `ice-media-cmd` owns outbound transport/seek calls (its own MTA). Kept off `ice-audio` so slow
//! media apps never delay device switches, and outbound commands never queue behind slow metadata or
//! album-art reads.
//!
//! Signals set bits in a dirty mask and wake `ice-media` at most once per burst (review MED-07); a
//! track change fires properties + playback + timeline together, so the watcher waits [`SETTLE`]
//! and then reads once. Track titles are only ever logged at `debug` (code-practices §4).

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
/// titles, so the notch showed the old song until some later event (owner report 2026-11-24: ~5 s). Bounded
/// follow-ups stop early once both a new track and new art have been observed.
const CONFIRM_AFTER: [Duration; 3] =
    [Duration::from_millis(200), Duration::from_millis(800), Duration::from_millis(2000)];
/// Art budget for the first read of a burst, so a cover that isn't ready can't hold back the new title.
const FIRST_ART_BUDGET: Duration = Duration::from_millis(250);
/// Art budget for follow-up confirm reads, bounded so an unresponsive app never stalls the watcher long.
const CONFIRM_ART_BUDGET: Duration = Duration::from_millis(1200);
/// Fallback local playback check after a sent transport command if GSMTC hasn't fired an event yet.
const ACTION_PLAYBACK_CHECK: Duration = Duration::from_millis(150);
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

enum Cmd {
    Action(MediaAction),
    Quit,
}

/// Cloneable handle to `ice-media-cmd` so callers (`AudioEngine::send`) can dispatch transport/seek
/// commands directly without queueing behind `ice-audio`.
#[derive(Clone)]
pub(crate) struct MediaCommandSender {
    tx: Sender<Cmd>,
}

impl MediaCommandSender {
    pub(crate) fn send(&self, action: MediaAction) {
        let _ = self.tx.send(Cmd::Action(action));
    }
}

enum WatchIn {
    Wake,
    ActionSent(MediaAction),
    Quit,
}

/// Handle to the media threads. Dropping stops both threads and unsubscribes everything.
pub struct MediaService {
    watch_tx: Sender<WatchIn>,
    cmd_tx: Sender<Cmd>,
    watch_join: Option<JoinHandle<()>>,
    cmd_join: Option<JoinHandle<()>>,
}

impl MediaService {
    /// Starts both media threads. `on_update` runs on `ice-media` and must not block (e.g. a channel send).
    ///
    /// # Errors
    /// Fails only if a thread can't be spawned. A missing GSMTC is logged and leaves media empty.
    pub fn start(on_update: impl Fn(MediaUpdate) + Send + 'static) -> std::io::Result<Self> {
        let (watch_tx, watch_rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let wake = watch_tx.clone();
        let cmd_watch_tx = watch_tx.clone();
        let watch_join = std::thread::Builder::new()
            .name("ice-media".into())
            .spawn(move || run_watcher(&watch_rx, wake, &on_update))?;
        let cmd_join = match std::thread::Builder::new()
            .name("ice-media-cmd".into())
            .spawn(move || run_commands(&cmd_rx, &cmd_watch_tx))
        {
            Ok(j) => j,
            Err(e) => {
                let _ = watch_tx.send(WatchIn::Quit);
                let _ = watch_join.join();
                return Err(e);
            }
        };
        Ok(Self { watch_tx, cmd_tx, watch_join: Some(watch_join), cmd_join: Some(cmd_join) })
    }

    #[must_use]
    pub(crate) fn command_sender(&self) -> MediaCommandSender {
        MediaCommandSender { tx: self.cmd_tx.clone() }
    }

    /// Sends a transport command to the current session (asynchronously; the result shows up as an update).
    pub fn send(&self, action: MediaAction) {
        let _ = self.cmd_tx.send(Cmd::Action(action));
    }
}

impl Drop for MediaService {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(Cmd::Quit);
        let _ = self.watch_tx.send(WatchIn::Quit);
        if let Some(j) = self.cmd_join.take() {
            let _ = j.join();
        }
        if let Some(j) = self.watch_join.take() {
            let _ = j.join();
        }
    }
}

const SESSION_BITS: u8 = MediaSignal::CurrentSessionChanged.bit() | MediaSignal::SessionsChanged.bit();
const PLAYBACK_BITS: u8 = MediaSignal::PlaybackChanged.bit() | MediaSignal::TimelineChanged.bit();

/// Coalesces queued transport commands so seek drags execute only the final position and track skips drop
/// any superseded seeks queued ahead of them.
fn coalesce_actions(actions: Vec<MediaAction>) -> Vec<MediaAction> {
    let mut out = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            MediaAction::Seek { .. } => {
                if matches!(out.last(), Some(MediaAction::Seek { .. })) {
                    out.pop();
                }
                out.push(action);
            }
            MediaAction::Next | MediaAction::Previous => {
                out.retain(|a| !matches!(a, MediaAction::Seek { .. }));
                out.push(action);
            }
            MediaAction::Play | MediaAction::Pause => {
                if out.last() != Some(&action) {
                    out.push(action);
                }
            }
            MediaAction::PlayPause => out.push(action),
        }
    }
    out
}

fn run_commands(rx: &Receiver<Cmd>, watch_tx: &Sender<WatchIn>) {
    let _com = match ComGuard::init_mta() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "media command thread: COM unavailable; controls disabled");
            return;
        }
    };
    let mgr = match media::manager() {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "media command thread: GSMTC unavailable; controls disabled");
            return;
        }
    };
    while let Ok(first) = rx.recv() {
        let mut actions = Vec::new();
        match first {
            Cmd::Quit => return,
            Cmd::Action(a) => actions.push(a),
        }
        while let Ok(more) = rx.try_recv() {
            match more {
                Cmd::Quit => return,
                Cmd::Action(a) => actions.push(a),
            }
        }
        for action in coalesce_actions(actions) {
            match media::send(&mgr, action) {
                Ok(true) => {
                    let _ = watch_tx.send(WatchIn::ActionSent(action));
                }
                Ok(false) => tracing::debug!(?action, "media app refused the command"),
                Err(e) => tracing::debug!(?action, error = %e, "media command failed"),
            }
        }
    }
}

fn run_watcher(rx: &Receiver<WatchIn>, wake: Sender<WatchIn>, out: &dyn Fn(MediaUpdate)) {
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
                let _ = wake.send(WatchIn::Wake);
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
    let mut playback_check_at: Option<Instant> = None;
    let mut confirm = Confirm::default();
    loop {
        let deadline = [settle_at, confirm.at, playback_check_at].into_iter().flatten().min();
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
            Some(WatchIn::Quit) => break,
            Some(WatchIn::Wake) => {
                settle_at.get_or_insert_with(|| Instant::now() + SETTLE);
            }
            Some(WatchIn::ActionSent(action)) => {
                let now = Instant::now();
                match action {
                    MediaAction::Next | MediaAction::Previous => {
                        playback_check_at = None;
                        confirm.arm(now, false, false);
                    }
                    MediaAction::Seek { .. } | MediaAction::PlayPause | MediaAction::Play | MediaAction::Pause => {
                        playback_check_at = Some(now + ACTION_PLAYBACK_CHECK);
                    }
                }
            }
            None => {
                let now = Instant::now();
                if settle_at.is_some_and(|t| t <= now) {
                    settle_at = None;
                    playback_check_at = None;
                    let bits = dirty.swap(0, Ordering::AcqRel);
                    let outcome = state.apply(bits, &mut watcher, false, FIRST_ART_BUDGET, out);
                    if outcome.read_track {
                        confirm.on_settle(now, outcome.track_changed, outcome.art_changed);
                    }
                } else if playback_check_at.is_some_and(|t| t <= now) {
                    playback_check_at = None;
                    let outcome = state.apply(PLAYBACK_BITS, &mut watcher, false, FIRST_ART_BUDGET, out);
                    if outcome.read_track {
                        confirm.on_settle(now, outcome.track_changed, outcome.art_changed);
                    }
                } else if confirm.at.is_some_and(|t| t <= now) {
                    let outcome =
                        state.apply(MediaSignal::PropertiesChanged.bit(), &mut watcher, false, CONFIRM_ART_BUDGET, out);
                    confirm.on_confirm(outcome.track_changed, outcome.art_changed);
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
    saw_track_change: bool,
    saw_art_change: bool,
}

impl Confirm {
    fn disarm(&mut self) {
        self.armed_at = None;
        self.step = 0;
        self.at = None;
        self.saw_track_change = false;
        self.saw_art_change = false;
    }

    fn arm(&mut self, now: Instant, track_changed: bool, art_changed: bool) {
        if track_changed && art_changed {
            self.disarm();
            return;
        }
        self.armed_at = Some(now);
        self.step = 0;
        self.at = Some(now + CONFIRM_AFTER[0]);
        self.saw_track_change = track_changed;
        self.saw_art_change = art_changed;
    }

    fn advance(&mut self) {
        self.step += 1;
        self.at = self.armed_at.zip(CONFIRM_AFTER.get(self.step)).map(|(t, d)| t + *d);
        if self.at.is_none() {
            self.disarm();
        }
    }

    fn on_settle(&mut self, now: Instant, track_changed: bool, art_changed: bool) {
        if self.at.is_none() {
            self.arm(now, track_changed, art_changed);
        } else {
            self.record(track_changed, art_changed, false);
        }
    }

    fn on_confirm(&mut self, track_changed: bool, art_changed: bool) {
        self.record(track_changed, art_changed, true);
    }

    fn record(&mut self, track_changed: bool, art_changed: bool, advance_if_pending: bool) {
        if track_changed {
            self.saw_track_change = true;
            self.saw_art_change = art_changed;
        } else if art_changed {
            self.saw_art_change = true;
        }
        if self.saw_track_change && self.saw_art_change {
            self.disarm();
        } else if advance_if_pending {
            self.advance();
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ApplyOutcome {
    read_track: bool,
    track_changed: bool,
    art_changed: bool,
}

/// A new track can arrive with only playback/timeline events (or with a stale properties read): a jump in
/// duration gives it away.
fn duration_jumped(prev: &NowPlaying, next: &NowPlaying) -> bool {
    matches!((prev.duration_ms, next.duration_ms), (Some(a), Some(b)) if a.abs_diff(b) > DURATION_JUMP_MS)
}

fn same_track(a: Option<&NowPlaying>, b: Option<&NowPlaying>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x.app_id == y.app_id && x.title == y.title && x.artist == y.artist && x.album == y.album,
        (None, None) => true,
        _ => false,
    }
}

#[derive(Default)]
struct State {
    last: Option<NowPlaying>,
    /// The art id the engine already has the PNG for.
    sent_art: Option<String>,
}

impl State {
    /// Reads and emits what changed. Reports whether a full track read occurred (so confirm reads can run when needed).
    fn apply(
        &mut self,
        bits: u8,
        watcher: &mut MediaWatcher,
        first: bool,
        art_budget: Duration,
        out: &dyn Fn(MediaUpdate),
    ) -> ApplyOutcome {
        let session_changed = bits & SESSION_BITS != 0 && watcher.follow_current();
        let mut full = first || session_changed || bits & MediaSignal::PropertiesChanged.bit() != 0;
        if !full && bits & PLAYBACK_BITS == 0 {
            return ApplyOutcome::default(); // e.g. another app's session opened or closed
        }
        let session = watcher.manager().GetCurrentSession().ok();
        // Only status/timeline changed: cheap local reads, track and art kept, unless the duration jumped.
        let mut quick = None;
        if let (Some(s), Some(last), false) = (&session, &self.last, full) {
            let mut now = last.clone();
            if let Err(e) = media::refresh_playback(s, &mut now) {
                tracing::debug!(error = %e, "playback read failed; keeping the last state");
                return ApplyOutcome::default();
            }
            if duration_jumped(last, &now) {
                full = true;
            } else {
                quick = Some(now);
            }
        }
        let known_art = self.last.as_ref().and_then(|m| m.art.as_ref());
        let (media, art) = match (session, quick) {
            (None, _) => (None, None),
            (Some(_), Some(now)) => (Some(now), None),
            // Art is re-read on every track read: some apps send the new title first and the new thumbnail in
            // a second change, so "same title" doesn't mean "same art". Matching hashes reuse `known_art`
            // without re-encoding the PNG.
            (Some(s), None) => match media::read_within_cached(&s, Some(ART_PX), art_budget, known_art) {
                Ok((now, art)) => (Some(now), art),
                Err(e) => {
                    tracing::debug!(error = %e, "media read failed; keeping the last state");
                    return ApplyOutcome { read_track: full, track_changed: false, art_changed: false };
                }
            },
        };
        let track_changed = !same_track(self.last.as_ref(), media.as_ref());
        let art_changed =
            media.as_ref().and_then(|m| m.art.as_ref()).map(|a| a.id.as_str()) != self.sent_art.as_deref();
        let art = art.filter(|a| self.sent_art.as_deref() != Some(a.info.id.as_str()));
        let outcome = ApplyOutcome {
            read_track: full && media.is_some(),
            track_changed,
            art_changed: art_changed && media.as_ref().is_some_and(|m| m.art.is_some()),
        };
        if media == self.last && art.is_none() {
            return outcome;
        }
        if let Some(a) = &art {
            self.sent_art = Some(a.info.id.clone());
        }
        if let Some(n) = &media {
            tracing::debug!(app = %n.app_id, title = %n.title, status = ?n.status, "now playing");
        }
        self.last.clone_from(&media);
        out(MediaUpdate { media, art });
        outcome
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
            position_updated_ms: None,
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
        c.arm(t0, false, false);
        let mut seen = Vec::new();
        while let Some(at) = c.at {
            seen.push(at - t0);
            c.on_confirm(false, false);
        }
        assert_eq!(seen, CONFIRM_AFTER.to_vec());
    }

    #[test]
    fn confirm_stops_early_once_track_and_art_both_arrive() {
        let t0 = Instant::now();
        let mut c = Confirm::default();
        // Already observed both on settle -> never arms.
        c.on_settle(t0, true, true);
        assert_eq!(c.at, None);

        // Title arrived first, art arrives on first confirm -> disarms cleanly.
        c.on_settle(t0, true, false);
        assert_eq!(c.at, Some(t0 + CONFIRM_AFTER[0]));
        c.on_confirm(false, true);
        assert_eq!(c.at, None);

        // Stale settle then title+art both arrive via wake before confirm fires -> disarms cleanly.
        c.on_settle(t0, false, false);
        assert!(c.at.is_some());
        c.on_settle(t0 + Duration::from_millis(50), true, true);
        assert_eq!(c.at, None);
    }

    #[test]
    fn coalesce_actions_keeps_latest_seek_and_drops_stale_seeks_before_skip() {
        assert_eq!(
            coalesce_actions(vec![
                MediaAction::Seek { position_ms: 1_000 },
                MediaAction::Seek { position_ms: 5_000 },
                MediaAction::Seek { position_ms: 9_000 },
            ]),
            vec![MediaAction::Seek { position_ms: 9_000 }]
        );
        assert_eq!(
            coalesce_actions(vec![MediaAction::Seek { position_ms: 4_000 }, MediaAction::Next, MediaAction::Next,]),
            vec![MediaAction::Next, MediaAction::Next]
        );
        assert_eq!(
            coalesce_actions(vec![
                MediaAction::Play,
                MediaAction::Play,
                MediaAction::Pause,
                MediaAction::Pause,
                MediaAction::PlayPause,
                MediaAction::PlayPause,
            ]),
            vec![MediaAction::Play, MediaAction::Pause, MediaAction::PlayPause, MediaAction::PlayPause]
        );
    }
}
