//! Now playing via Global System Media Transport Controls (GSMTC). Phase 3 (F15–F17).
//!
//! Event-driven: [`MediaWatcher`] forwards GSMTC change events to a non-blocking sink; the owner re-reads
//! what changed ([`read`] for track + art, [`refresh_playback`] for status and timeline). No polling
//! (code-practices §2). Album art is decoded and downscaled with Windows' own imaging API (no image crate),
//! hashed for caching (`iceart://<id>`), and given a readable accent colour (`ice_core::media`).
//!
//! **Threading (review MED-09):** every function here blocks on cross-process WinRT calls and must run on an
//! MTA thread ([`crate::com::ComGuard::init_mta`]), never on a UI (STA) thread. The engine runs them on its own
//! `ice-media` thread ([`crate::media_service`]).
//!
//! **Bounded (review MED-02/03):** a hung media app costs at most [`CALL_LIMIT`] per call and [`ART_BUDGET`]
//! for all of the art work; oversized art is rejected before it is decoded or copied.

use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use ice_core::engine::MediaAction;
use ice_core::media::{ArtInfo, NowPlaying, PlaybackStatus, accent_for_art, art_id};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Imaging::{
    BitmapAlphaMode, BitmapDecoder, BitmapEncoder, BitmapInterpolationMode, BitmapPixelFormat, BitmapTransform,
    ColorManagementMode, ExifOrientationMode,
};
use windows::Media::Control::{
    CurrentSessionChangedEventArgs, GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as MediaProperties,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    GlobalSystemMediaTransportControlsSessionTimelineProperties as Timeline, MediaPropertiesChangedEventArgs,
    PlaybackInfoChangedEventArgs, SessionsChangedEventArgs, TimelinePropertiesChangedEventArgs,
};
use windows::Storage::Streams::{DataReader, IRandomAccessStream, InMemoryRandomAccessStream};
use windows::Win32::Foundation::{E_BOUNDS, E_UNEXPECTED, ERROR_TIMEOUT};
use windows::core::{Error, Interface, Result, RuntimeType};
use windows_future::{AsyncActionCompletedHandler, AsyncOperationCompletedHandler, IAsyncAction, IAsyncOperation};

pub use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as MediaManager;

/// Longest wait for one metadata or transport call to another app.
pub const CALL_LIMIT: Duration = Duration::from_millis(500);
/// Longest wait for the session manager (once, at start-up; ~25 ms measured).
const MANAGER_LIMIT: Duration = Duration::from_secs(2);
/// Total time for one album art read: open, decode, scale, encode.
pub const ART_BUDGET: Duration = Duration::from_secs(2);
/// Downscaled art size limits (longer side).
const ART_PX: std::ops::RangeInclusive<u32> = 16..=512;
/// Ceilings applied before decoding or copying (review MED-03).
const MAX_ART_STREAM: u64 = 8 * 1024 * 1024;
const MAX_SOURCE_SIDE: u32 = 8192;
const MAX_SOURCE_PIXELS: u64 = 32_000_000;
const MAX_PNG: u64 = 512 * 1024;

fn timed_out() -> Error {
    Error::new(ERROR_TIMEOUT.to_hresult(), "the media app did not answer in time")
}

fn too_big(what: &str) -> Error {
    Error::new(E_BOUNDS, format!("{what} is too large"))
}

/// Waits for `op` for at most `limit`, cancelling it on expiry (review MED-02).
fn wait<T: RuntimeType + 'static>(op: &IAsyncOperation<T>, limit: Duration) -> Result<T> {
    let (tx, rx) = mpsc::sync_channel(1);
    // Runs at once if the operation already finished.
    op.SetCompleted(&AsyncOperationCompletedHandler::new(move |_, _| {
        let _ = tx.try_send(());
        Ok(())
    }))?;
    if rx.recv_timeout(limit).is_err() {
        let _ = op.Cancel();
        return Err(timed_out());
    }
    op.GetResults()
}

fn wait_action(op: &IAsyncAction, limit: Duration) -> Result<()> {
    let (tx, rx) = mpsc::sync_channel(1);
    op.SetCompleted(&AsyncActionCompletedHandler::new(move |_, _| {
        let _ = tx.try_send(());
        Ok(())
    }))?;
    if rx.recv_timeout(limit).is_err() {
        let _ = op.Cancel();
        return Err(timed_out());
    }
    op.GetResults()
}

/// A shared time budget for several waits.
struct Budget(Instant);

impl Budget {
    fn new(total: Duration) -> Self {
        Self(Instant::now() + total)
    }

    fn left(&self) -> Duration {
        self.0.saturating_duration_since(Instant::now())
    }
}

/// Requests the session manager.
///
/// # Errors
/// Fails if GSMTC is unavailable or doesn't answer within 2 s.
pub fn manager() -> Result<Manager> {
    wait(&Manager::RequestAsync()?, MANAGER_LIMIT)
}

/// What Windows considers the current session, without art. `Ok(None)` when nothing is playing.
///
/// # Errors
/// Fails if the current session's properties can't be read (review MED-09: not masked as "nothing").
pub fn now_playing(mgr: &Manager) -> Result<Option<NowPlaying>> {
    // `GetCurrentSession` fails with a null result when there is no session.
    let Ok(s) = mgr.GetCurrentSession() else { return Ok(None) };
    read(&s, None).map(|(n, _)| Some(n))
}

/// Every media session Windows knows about, without art. Sessions that fail to read (typically closing
/// right now) are skipped rather than failing the whole list (review MED-09).
///
/// # Errors
/// Fails if the session list can't be read.
pub fn sessions(mgr: &Manager) -> Result<Vec<NowPlaying>> {
    Ok(mgr
        .GetSessions()?
        .into_iter()
        .filter_map(|s| read(&s, None).inspect_err(|e| tracing::debug!(error = %e, "skipping a media session")).ok())
        .map(|(n, _)| n)
        .collect())
}

/// Sends a transport command to the current session. `Ok(false)` if the app refused it.
///
/// # Errors
/// Fails if there is no current session or the app doesn't answer in time.
pub fn send(mgr: &Manager, action: MediaAction) -> Result<bool> {
    let s = mgr.GetCurrentSession()?;
    let op = match action {
        MediaAction::PlayPause => s.TryTogglePlayPauseAsync()?,
        MediaAction::Play => s.TryPlayAsync()?,
        MediaAction::Pause => s.TryPauseAsync()?,
        MediaAction::Next => s.TrySkipNextAsync()?,
        MediaAction::Previous => s.TrySkipPreviousAsync()?,
        MediaAction::Seek { position_ms } => s.TryChangePlaybackPositionAsync(seek_ticks(&s, position_ms))?,
    };
    wait(&op, CALL_LIMIT)
}

/// Resolves a millisecond track offset into GSMTC's 100 ns timeline coordinates, anchored to `StartTime`.
fn seek_ticks(s: &Session, position_ms: u64) -> i64 {
    let (start, end) = s.GetTimelineProperties().ok().map_or((0, 0), |t: Timeline| {
        let tick = |r: Result<windows::Foundation::TimeSpan>| r.map_or(0, |v| v.Duration);
        (tick(t.StartTime()), tick(t.EndTime()))
    });
    clamp_seek_ticks(start, end, position_ms)
}

/// Converts a millisecond offset relative to `start` into 100 ns ticks clamped to `[start, end]`.
fn clamp_seek_ticks(start: i64, end: i64, position_ms: u64) -> i64 {
    let offset = i64::try_from(position_ms.saturating_mul(10_000)).unwrap_or(i64::MAX);
    if end > start { start.saturating_add(offset).clamp(start, end) } else { offset }
}

/// Downscaled album art: PNG bytes for the UI plus its id and accent.
#[derive(Debug, Clone)]
pub struct Art {
    pub info: ArtInfo,
    pub png: Vec<u8>,
}

/// The current session's art, downscaled so the longer side is at most `max_px` (clamped to 16–512).
/// `Ok(None)` without a session or art.
///
/// # Errors
/// Fails if the image is too large, can't be decoded or encoded, or takes longer than [`ART_BUDGET`].
pub fn current_art(mgr: &Manager, max_px: u32) -> Result<Option<Art>> {
    let Ok(session) = mgr.GetCurrentSession() else { return Ok(None) };
    read_art(&wait(&session.TryGetMediaPropertiesAsync()?, CALL_LIMIT)?, max_px, ART_BUDGET)
}

/// Reads a session once: track, playback status, timeline and, with `art_px`, the art from the *same*
/// properties, so track and art can't tear (review MED-02). An art failure is logged and leaves `art` empty;
/// the track is still returned.
///
/// # Errors
/// Fails if the track or playback state can't be read.
pub fn read(s: &Session, art_px: Option<u32>) -> Result<(NowPlaying, Option<Art>)> {
    read_within(s, art_px, ART_BUDGET)
}

/// [`read`] with a custom art budget. A short budget keeps a slow cover from holding back a new title; the
/// caller re-reads later to pick the cover up.
///
/// # Errors
/// Fails if the track or playback state can't be read.
pub fn read_within(s: &Session, art_px: Option<u32>, art_budget: Duration) -> Result<(NowPlaying, Option<Art>)> {
    read_within_cached(s, art_px, art_budget, None)
}

/// Like [`read_within`], but reuses `known_art` when the decoded thumbnail hashes to the same `id`, skipping
/// accent extraction and PNG re-encoding.
///
/// # Errors
/// Fails if the track or playback state can't be read.
pub(crate) fn read_within_cached(
    s: &Session,
    art_px: Option<u32>,
    art_budget: Duration,
    known_art: Option<&ArtInfo>,
) -> Result<(NowPlaying, Option<Art>)> {
    let props = wait(&s.TryGetMediaPropertiesAsync()?, CALL_LIMIT)?;
    let mut now = NowPlaying {
        app_id: s.SourceAppUserModelId()?.to_string(),
        title: props.Title()?.to_string(),
        artist: props.Artist()?.to_string(),
        album: props.AlbumTitle()?.to_string(),
        status: PlaybackStatus::Closed,
        position_ms: None,
        duration_ms: None,
        position_updated_ms: None,
        art: None,
    };
    refresh_playback(s, &mut now)?;
    let outcome = art_px.and_then(|px| {
        read_art_cached(&props, px, art_budget, known_art)
            .inspect_err(|e| tracing::debug!(error = %e, "album art unavailable"))
            .ok()
            .flatten()
    });
    let art = match outcome {
        Some(ArtOutcome::New(art)) => {
            now.art = Some(art.info.clone());
            Some(art)
        }
        Some(ArtOutcome::Reused(info)) => {
            now.art = Some(info);
            None
        }
        None => None,
    };
    Ok((now, art))
}

/// Windows `FILETIME` / WinRT `DateTime` 100 ns ticks between 1601-01-01 and Unix epoch 1970-01-01.
const WINDOWS_TO_UNIX_EPOCH_TICKS: i64 = 116_444_736_000_000_000;

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

/// Converts a WinRT `DateTime.UniversalTime` (100 ns since 1601-01-01 UTC) to Unix epoch milliseconds.
fn last_updated_unix_ms(universal_time: i64, now_ms: u64) -> Option<u64> {
    if universal_time <= WINDOWS_TO_UNIX_EPOCH_TICKS {
        return None;
    }
    let unix_ms = u64::try_from((universal_time - WINDOWS_TO_UNIX_EPOCH_TICKS) / 10_000).ok()?;
    Some(if unix_ms > now_ms.saturating_add(5_000) { now_ms } else { unix_ms })
}

#[expect(clippy::too_many_arguments, reason = "pure timeline timestamp helper keeps refresh_playback testable")]
fn resolve_position_updated_ms(
    prev_status: PlaybackStatus,
    prev_pos: Option<u64>,
    prev_updated: Option<u64>,
    status: PlaybackStatus,
    pos: Option<u64>,
    dur: Option<u64>,
    last_updated_ticks: i64,
    now_ms: u64,
) -> Option<u64> {
    let pos_ms = pos?;
    if let Some(ts) = last_updated_unix_ms(last_updated_ticks, now_ms) {
        if status == PlaybackStatus::Playing {
            let elapsed = now_ms.saturating_sub(ts);
            let stale_resume = prev_status != PlaybackStatus::Playing && elapsed > 5_000;
            let stale_overshoot = dur.is_some_and(|d| pos_ms.saturating_add(elapsed) > d.saturating_add(15_000));
            if stale_resume || stale_overshoot {
                return Some(now_ms);
            }
        }
        return Some(ts);
    }
    if status == PlaybackStatus::Playing
        && prev_status == PlaybackStatus::Playing
        && prev_pos == pos
        && prev_updated.is_some()
    {
        return prev_updated;
    }
    Some(now_ms)
}

/// Updates only the playback status and timeline (local reads; no request to the media app).
///
/// # Errors
/// Fails if the playback state can't be read.
pub fn refresh_playback(s: &Session, now: &mut NowPlaying) -> Result<()> {
    let prev_status = now.status;
    let prev_pos = now.position_ms;
    let prev_updated = now.position_updated_ms;
    now.status = match s.GetPlaybackInfo()?.PlaybackStatus()? {
        st if st == Status::Playing => PlaybackStatus::Playing,
        st if st == Status::Paused => PlaybackStatus::Paused,
        st if st == Status::Stopped => PlaybackStatus::Stopped,
        st if st == Status::Changing => PlaybackStatus::Changing,
        st if st == Status::Opened => PlaybackStatus::Opened,
        _ => PlaybackStatus::Closed,
    };
    let (pos, dur, last_updated_ticks) = s.GetTimelineProperties().ok().map_or((None, None, 0), |t: Timeline| {
        let tick = |r: Result<windows::Foundation::TimeSpan>| r.map_or(0, |t| t.Duration);
        let (p, d) = timeline_ms(tick(t.StartTime()), tick(t.EndTime()), tick(t.Position()));
        let updated = t.LastUpdatedTime().map_or(0, |dt| dt.UniversalTime);
        (p, d, updated)
    });
    now.position_ms = pos;
    now.duration_ms = dur;
    now.position_updated_ms = resolve_position_updated_ms(
        prev_status,
        prev_pos,
        prev_updated,
        now.status,
        pos,
        dur,
        last_updated_ticks,
        now_unix_ms(),
    );
    Ok(())
}

/// `(position, duration)` in ms from GSMTC's 100 ns ticks. Both are relative to the start time, and the
/// position is clamped into the track (review MED-09). No duration (live streams, some browsers) = neither.
fn timeline_ms(start: i64, end: i64, position: i64) -> (Option<u64>, Option<u64>) {
    let duration = end.saturating_sub(start);
    if duration <= 0 {
        return (None, None);
    }
    let position = position.saturating_sub(start).clamp(0, duration);
    let ms = |ticks: i64| u64::try_from(ticks / 10_000).unwrap_or(0);
    (Some(ms(position)), Some(ms(duration)))
}

/// The size `(w, h)` scaled so the longer side is at most `max_px`, never below 1 px.
fn fit(w: u32, h: u32, max_px: u32) -> (u32, u32) {
    let scale = (f64::from(max_px) / f64::from(w.max(h))).min(1.0);
    // Both results are ≤ the inputs (scale ≤ 1), so the casts can't truncate or go negative.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = |v: u32| (f64::from(v) * scale).round().max(1.0) as u32;
    (px(w), px(h))
}

enum ArtOutcome {
    New(Art),
    Reused(ArtInfo),
}

fn read_art(props: &MediaProperties, max_px: u32, total: Duration) -> Result<Option<Art>> {
    match read_art_cached(props, max_px, total, None)? {
        Some(ArtOutcome::New(art)) => Ok(Some(art)),
        Some(ArtOutcome::Reused(_)) | None => Ok(None),
    }
}

fn read_art_cached(
    props: &MediaProperties,
    max_px: u32,
    total: Duration,
    known_art: Option<&ArtInfo>,
) -> Result<Option<ArtOutcome>> {
    let budget = Budget::new(total.min(ART_BUDGET));
    let max_px = max_px.clamp(*ART_PX.start(), *ART_PX.end());
    let Ok(thumb) = props.Thumbnail() else { return Ok(None) };
    let stream: IRandomAccessStream = wait(&thumb.OpenReadAsync()?, budget.left())?.cast()?;
    match stream.Size()? {
        0 => return Ok(None),
        n if n > MAX_ART_STREAM => return Err(too_big("album art")),
        _ => {}
    }
    let decoder = wait(&BitmapDecoder::CreateAsync(&stream)?, budget.left())?;
    let (w, h) = (decoder.PixelWidth()?, decoder.PixelHeight()?);
    if w == 0 || h == 0 {
        return Ok(None);
    }
    if w.max(h) > MAX_SOURCE_SIDE || u64::from(w) * u64::from(h) > MAX_SOURCE_PIXELS {
        return Err(too_big("album art"));
    }
    // Scaling happens before the EXIF rotation, so a 90°/270° image comes out transposed (review MED-05).
    let transposed = (decoder.OrientedPixelWidth()?, decoder.OrientedPixelHeight()?) != (w, h);
    let (sw, sh) = fit(w, h, max_px);
    let (out_w, out_h) = if transposed { (sh, sw) } else { (sw, sh) };
    let transform = BitmapTransform::new()?;
    transform.SetScaledWidth(sw)?;
    transform.SetScaledHeight(sh)?;
    transform.SetInterpolationMode(BitmapInterpolationMode::Fant)?;
    let pixels = wait(
        &decoder.GetPixelDataTransformedAsync(
            BitmapPixelFormat::Bgra8,
            // Straight alpha: what the colour maths and PNG expect (review MED-06).
            BitmapAlphaMode::Straight,
            &transform,
            ExifOrientationMode::RespectExifOrientation,
            ColorManagementMode::DoNotColorManage,
        )?,
        budget.left(),
    )?
    .DetachPixelData()?;
    let pixels: &[u8] = &pixels;
    if pixels.len() as u64 != u64::from(out_w) * u64::from(out_h) * 4 {
        return Err(Error::new(E_UNEXPECTED, "album art pixel buffer has an unexpected size"));
    }
    let id = art_id(out_w, out_h, pixels);
    if let Some(known) = known_art
        && known.id == id
        && known.width == out_w
        && known.height == out_h
    {
        return Ok(Some(ArtOutcome::Reused(known.clone())));
    }
    let info = ArtInfo { id, width: out_w, height: out_h, accent: accent_for_art(Some(pixels)) };
    Ok(Some(ArtOutcome::New(Art { info, png: encode_png(pixels, out_w, out_h, &budget)? })))
}

fn encode_png(pixels: &[u8], w: u32, h: u32, budget: &Budget) -> Result<Vec<u8>> {
    let mem = InMemoryRandomAccessStream::new()?;
    let encoder = wait(&BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId()?, &mem)?, budget.left())?;
    encoder.SetPixelData(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Straight, w, h, 96.0, 96.0, pixels)?;
    wait_action(&encoder.FlushAsync()?, budget.left())?;
    let size = mem.Size()?;
    if size > MAX_PNG {
        return Err(too_big("encoded album art"));
    }
    let len = u32::try_from(size).map_err(|_| too_big("encoded album art"))?;
    let reader = DataReader::CreateDataReader(&mem.GetInputStreamAt(0)?)?;
    let loaded = wait(&reader.LoadAsync(len)?, budget.left())?;
    if loaded != len {
        return Err(Error::new(E_UNEXPECTED, "album art PNG was read short"));
    }
    let mut buf = vec![0u8; len as usize];
    reader.ReadBytes(&mut buf)?;
    Ok(buf)
}

/// What changed. The owner re-reads what it needs; callbacks never do work themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaSignal {
    /// Another app became the current session: call [`MediaWatcher::follow_current`] and re-read.
    CurrentSessionChanged,
    /// An app opened or closed a session (often not the current one; review MED-07). Call
    /// [`MediaWatcher::follow_current`]; re-read only if it reports a change.
    SessionsChanged,
    /// Track, artist, album or art changed.
    PropertiesChanged,
    /// Playing/paused/stopped changed.
    PlaybackChanged,
    /// Position/duration changed (apps report this on seek or track change, not continuously).
    TimelineChanged,
}

impl MediaSignal {
    /// A distinct bit per signal, for coalescing bursts into one dirty mask.
    #[must_use]
    pub const fn bit(self) -> u8 {
        1 << self as u8
    }
}

type Sink = Arc<dyn Fn(MediaSignal) + Send + Sync>;

fn handler<S, A: RuntimeType + 'static>(sink: &Sink, signal: MediaSignal) -> TypedEventHandler<S, A>
where
    S: RuntimeType + 'static,
{
    let sink = Arc::clone(sink);
    TypedEventHandler::new(move |_, _| {
        sink(signal);
        Ok(())
    })
}

/// The followed session's event registrations. Each token is removed on drop, so a subscription that fails
/// half-way leaks nothing (review MED-01).
struct SessionSubs {
    session: Session,
    properties: Option<i64>,
    playback: Option<i64>,
    timeline: Option<i64>,
}

impl SessionSubs {
    fn subscribe(session: Session, sink: &Sink) -> Result<Self> {
        let mut subs = Self { session, properties: None, playback: None, timeline: None };
        subs.properties = Some(subs.session.MediaPropertiesChanged(&handler::<
            Session,
            MediaPropertiesChangedEventArgs,
        >(sink, MediaSignal::PropertiesChanged))?);
        subs.playback = Some(subs.session.PlaybackInfoChanged(&handler::<Session, PlaybackInfoChangedEventArgs>(
            sink,
            MediaSignal::PlaybackChanged,
        ))?);
        subs.timeline = Some(subs.session.TimelinePropertiesChanged(&handler::<
            Session,
            TimelinePropertiesChangedEventArgs,
        >(sink, MediaSignal::TimelineChanged))?);
        Ok(subs)
    }
}

impl Drop for SessionSubs {
    fn drop(&mut self) {
        if let Some(t) = self.properties {
            let _ = self.session.RemoveMediaPropertiesChanged(t);
        }
        if let Some(t) = self.playback {
            let _ = self.session.RemovePlaybackInfoChanged(t);
        }
        if let Some(t) = self.timeline {
            let _ = self.session.RemoveTimelinePropertiesChanged(t);
        }
    }
}

/// Subscribes to GSMTC events for the manager and the current session; unsubscribes on drop.
pub struct MediaWatcher {
    session: Option<SessionSubs>,
    current_token: Option<i64>,
    list_token: Option<i64>,
    mgr: Manager,
    sink: Sink,
}

impl MediaWatcher {
    /// `sink` runs on WinRT thread-pool threads and must not block (e.g. set a flag and send a wake-up).
    ///
    /// # Errors
    /// Fails if the manager events can't be subscribed (anything registered so far is removed again).
    pub fn start(mgr: Manager, sink: impl Fn(MediaSignal) + Send + Sync + 'static) -> Result<Self> {
        let mut w = Self { session: None, current_token: None, list_token: None, mgr, sink: Arc::new(sink) };
        w.current_token = Some(w.mgr.CurrentSessionChanged(&handler::<Manager, CurrentSessionChangedEventArgs>(
            &w.sink,
            MediaSignal::CurrentSessionChanged,
        ))?);
        w.list_token =
            Some(w.mgr.SessionsChanged(&handler::<Manager, SessionsChangedEventArgs>(
                &w.sink,
                MediaSignal::SessionsChanged,
            ))?);
        w.follow_current();
        Ok(w)
    }

    #[must_use]
    pub fn manager(&self) -> &Manager {
        &self.mgr
    }

    /// Moves the session subscriptions to whatever is current now. Keeps them (no gap, no re-subscribe) when
    /// the current session hasn't changed (review MED-01). Returns whether the followed session changed.
    pub fn follow_current(&mut self) -> bool {
        let current = self.mgr.GetCurrentSession().ok();
        if current.is_some() && current.as_ref() == self.session.as_ref().map(|s| &s.session) {
            return false;
        }
        let had = self.session.take().is_some();
        let Some(session) = current else { return had };
        match SessionSubs::subscribe(session, &self.sink) {
            Ok(subs) => self.session = Some(subs),
            Err(e) => tracing::warn!(error = %e, "media session events unavailable"),
        }
        true
    }
}

impl Drop for MediaWatcher {
    fn drop(&mut self) {
        self.session = None;
        if let Some(t) = self.current_token {
            let _ = self.mgr.RemoveCurrentSessionChanged(t);
        }
        if let Some(t) = self.list_token {
            let _ = self.mgr.RemoveSessionsChanged(t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_is_relative_to_start_and_clamped() {
        // 100 ns ticks: 10_000 per ms.
        assert_eq!(timeline_ms(0, 200_000_000, 50_000_000), (Some(5_000), Some(20_000)));
        assert_eq!(timeline_ms(10_000_000, 210_000_000, 60_000_000), (Some(5_000), Some(20_000)));
        assert_eq!(timeline_ms(0, 200_000_000, 900_000_000), (Some(20_000), Some(20_000)));
        assert_eq!(timeline_ms(0, 200_000_000, -5), (Some(0), Some(20_000)));
        assert_eq!(timeline_ms(0, 0, 5), (None, None));
    }

    #[test]
    fn seek_ticks_are_relative_to_start_and_clamped() {
        assert_eq!(clamp_seek_ticks(0, 200_000_000, 5_000), 50_000_000);
        assert_eq!(clamp_seek_ticks(10_000_000, 210_000_000, 5_000), 60_000_000);
        assert_eq!(clamp_seek_ticks(0, 200_000_000, 90_000), 200_000_000);
        assert_eq!(clamp_seek_ticks(0, 0, 12_345), 123_450_000);
    }

    #[test]
    fn last_updated_converts_winrt_ticks_and_guards_stale_resumes() {
        let now = 1_700_000_050_000_u64;
        let ten_secs_ago_ticks = WINDOWS_TO_UNIX_EPOCH_TICKS + ((now - 10_000) as i64) * 10_000;
        assert_eq!(last_updated_unix_ms(0, now), None);
        assert_eq!(last_updated_unix_ms(ten_secs_ago_ticks, now), Some(now - 10_000));
        // Playing with valid recent LastUpdatedTime keeps OS-stamped timestamp.
        assert_eq!(
            resolve_position_updated_ms(
                PlaybackStatus::Playing,
                Some(20_000),
                None,
                PlaybackStatus::Playing,
                Some(20_000),
                Some(200_000),
                ten_secs_ago_ticks,
                now,
            ),
            Some(now - 10_000),
        );
        // Resuming from Paused with a stale LastUpdatedTime anchors cleanly to now instead of jumping ahead.
        assert_eq!(
            resolve_position_updated_ms(
                PlaybackStatus::Paused,
                Some(20_000),
                None,
                PlaybackStatus::Playing,
                Some(20_000),
                Some(200_000),
                ten_secs_ago_ticks,
                now,
            ),
            Some(now),
        );
    }

    #[test]
    fn fit_keeps_the_aspect_and_never_upscales() {
        assert_eq!(fit(640, 640, 128), (128, 128));
        assert_eq!(fit(1000, 500, 128), (128, 64));
        assert_eq!(fit(64, 32, 128), (64, 32));
        assert_eq!(fit(8000, 1, 128), (128, 1));
    }

    #[test]
    fn signal_bits_are_distinct() {
        let all = [
            MediaSignal::CurrentSessionChanged,
            MediaSignal::SessionsChanged,
            MediaSignal::PropertiesChanged,
            MediaSignal::PlaybackChanged,
            MediaSignal::TimelineChanged,
        ];
        let mask = all.iter().fold(0u8, |m, s| {
            assert_eq!(m & s.bit(), 0);
            m | s.bit()
        });
        assert_eq!(mask.count_ones(), 5);
    }
}
