//! Now playing via Global System Media Transport Controls (GSMTC). Phase 3 (F15–F17).
//!
//! Event-driven: [`MediaWatcher`] forwards GSMTC change events to a non-blocking sink; the owner re-reads
//! [`now_playing`] (and [`current_art`] only when the track changed). No polling (code-practices §2).
//! Album art is decoded and downscaled with Windows' own imaging API (no image crate), hashed for caching
//! (`iceart://<id>`), and given a readable accent colour (`ice_core::media`).

use std::sync::Arc;

use ice_core::media::{ArtInfo, NowPlaying, PlaybackStatus, accent_for_art, content_id};
use windows::Foundation::{TimeSpan, TypedEventHandler};
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
use windows::core::{Interface, Result};

pub use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as MediaManager;

/// Requests the session manager (blocks until Windows answers; ~25 ms measured).
///
/// # Errors
/// Fails if GSMTC is unavailable.
pub fn manager() -> Result<Manager> {
    Manager::RequestAsync()?.join()
}

/// What Windows considers the current session, without art (fast).
///
/// # Errors
/// Fails if the session's properties can't be read.
pub fn now_playing(mgr: &Manager) -> Result<Option<NowPlaying>> {
    match mgr.GetCurrentSession() {
        Ok(s) => describe(&s).map(Some),
        Err(_) => Ok(None),
    }
}

/// Every media session Windows knows about, without art.
///
/// # Errors
/// Fails if the session list can't be read.
pub fn sessions(mgr: &Manager) -> Result<Vec<NowPlaying>> {
    mgr.GetSessions()?.into_iter().map(|s| describe(&s)).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    PlayPause,
    Play,
    Pause,
    Next,
    Previous,
}

/// Sends a transport command to the current session. `Ok(false)` if the app refused it.
///
/// # Errors
/// Fails if there is no current session.
pub fn send(mgr: &Manager, cmd: MediaCommand) -> Result<bool> {
    let s = mgr.GetCurrentSession()?;
    match cmd {
        MediaCommand::PlayPause => s.TryTogglePlayPauseAsync()?.join(),
        MediaCommand::Play => s.TryPlayAsync()?.join(),
        MediaCommand::Pause => s.TryPauseAsync()?.join(),
        MediaCommand::Next => s.TrySkipNextAsync()?.join(),
        MediaCommand::Previous => s.TrySkipPreviousAsync()?.join(),
    }
}

/// Downscaled album art: PNG bytes for the UI plus its id and accent.
#[derive(Debug, Clone)]
pub struct Art {
    pub info: ArtInfo,
    pub png: Vec<u8>,
}

/// The current session's art, downscaled so the longer side is at most `max_px`. `Ok(None)` without art.
///
/// # Errors
/// Fails if the image can't be decoded or encoded.
pub fn current_art(mgr: &Manager, max_px: u32) -> Result<Option<Art>> {
    let Ok(session) = mgr.GetCurrentSession() else { return Ok(None) };
    read_art(&session.TryGetMediaPropertiesAsync()?.join()?, max_px)
}

fn ms(t: TimeSpan) -> u64 {
    // TimeSpan counts 100 ns ticks.
    u64::try_from(t.Duration / 10_000).unwrap_or(0)
}

fn describe(s: &Session) -> Result<NowPlaying> {
    let props = s.TryGetMediaPropertiesAsync()?.join()?;
    let status = match s.GetPlaybackInfo()?.PlaybackStatus()? {
        st if st == Status::Playing => PlaybackStatus::Playing,
        st if st == Status::Paused => PlaybackStatus::Paused,
        st if st == Status::Stopped => PlaybackStatus::Stopped,
        st if st == Status::Changing => PlaybackStatus::Changing,
        st if st == Status::Opened => PlaybackStatus::Opened,
        _ => PlaybackStatus::Closed,
    };
    let timeline: Option<Timeline> = s.GetTimelineProperties().ok();
    let duration_ms = timeline.as_ref().and_then(|t| t.EndTime().ok()).map(ms).filter(|d| *d > 0);
    let position_ms = duration_ms.and(timeline.as_ref().and_then(|t| t.Position().ok()).map(ms));
    Ok(NowPlaying {
        app_id: s.SourceAppUserModelId()?.to_string(),
        title: props.Title()?.to_string(),
        artist: props.Artist()?.to_string(),
        album: props.AlbumTitle()?.to_string(),
        status,
        position_ms,
        duration_ms,
        art: None,
    })
}

fn read_art(props: &MediaProperties, max_px: u32) -> Result<Option<Art>> {
    let Ok(thumb) = props.Thumbnail() else { return Ok(None) };
    let stream: IRandomAccessStream = thumb.OpenReadAsync()?.join()?.cast()?;
    let decoder = BitmapDecoder::CreateAsync(&stream)?.join()?;
    let (w, h) = (decoder.PixelWidth()?, decoder.PixelHeight()?);
    if w == 0 || h == 0 {
        return Ok(None);
    }
    let scale = (f64::from(max_px) / f64::from(w.max(h))).min(1.0);
    // Bounded by `max_px` (≤ u32), so the casts can't truncate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (tw, th) = ((f64::from(w) * scale).round().max(1.0) as u32, (f64::from(h) * scale).round().max(1.0) as u32);
    let transform = BitmapTransform::new()?;
    transform.SetScaledWidth(tw)?;
    transform.SetScaledHeight(th)?;
    transform.SetInterpolationMode(BitmapInterpolationMode::Fant)?;
    let pixels = decoder
        .GetPixelDataTransformedAsync(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
            &transform,
            ExifOrientationMode::RespectExifOrientation,
            ColorManagementMode::DoNotColorManage,
        )?
        .join()?
        .DetachPixelData()?;
    let pixels: &[u8] = &pixels;
    let info = ArtInfo { id: content_id(pixels), width: tw, height: th, accent: accent_for_art(Some(pixels)) };
    Ok(Some(Art { info, png: encode_png(pixels, tw, th)? }))
}

fn encode_png(pixels: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    let mem = InMemoryRandomAccessStream::new()?;
    let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId()?, &mem)?.join()?;
    encoder.SetPixelData(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Premultiplied, w, h, 96.0, 96.0, pixels)?;
    encoder.FlushAsync()?.join()?;
    let len = u32::try_from(mem.Size()?).unwrap_or(u32::MAX);
    let reader = DataReader::CreateDataReader(&mem.GetInputStreamAt(0)?)?;
    reader.LoadAsync(len)?.join()?;
    let mut buf = vec![0u8; len as usize];
    reader.ReadBytes(&mut buf)?;
    Ok(buf)
}

/// What changed. The owner re-reads what it needs; callbacks never do work themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaSignal {
    /// Another app became the current session (or the session list changed): call [`MediaWatcher::follow_current`].
    SessionChanged,
    /// Track, artist, album or art changed.
    PropertiesChanged,
    /// Playing/paused/stopped changed.
    PlaybackChanged,
    /// Position/duration changed (apps report this on seek or track change, not continuously).
    TimelineChanged,
}

type Sink = Arc<dyn Fn(MediaSignal) + Send + Sync>;

/// Subscribes to GSMTC events for the manager and the current session; unsubscribes on drop.
pub struct MediaWatcher {
    mgr: Manager,
    sink: Sink,
    mgr_tokens: [i64; 2],
    session: Option<(Session, [i64; 3])>,
}

impl MediaWatcher {
    /// `sink` runs on WinRT thread-pool threads and must not block (e.g. a channel send).
    ///
    /// # Errors
    /// Fails if the manager events can't be subscribed.
    pub fn start(mgr: Manager, sink: impl Fn(MediaSignal) + Send + Sync + 'static) -> Result<Self> {
        let sink: Sink = Arc::new(sink);
        let (s1, s2) = (Arc::clone(&sink), Arc::clone(&sink));
        let current = mgr.CurrentSessionChanged(&TypedEventHandler::<Manager, CurrentSessionChangedEventArgs>::new(
            move |_, _| {
                s1(MediaSignal::SessionChanged);
                Ok(())
            },
        ))?;
        let list = mgr.SessionsChanged(&TypedEventHandler::<Manager, SessionsChangedEventArgs>::new(move |_, _| {
            s2(MediaSignal::SessionChanged);
            Ok(())
        }))?;
        let mut w = Self { mgr, sink, mgr_tokens: [current, list], session: None };
        w.follow_current();
        Ok(w)
    }

    #[must_use]
    pub fn manager(&self) -> &Manager {
        &self.mgr
    }

    /// Moves the session subscriptions to whatever is current now. Call after [`MediaSignal::SessionChanged`].
    pub fn follow_current(&mut self) {
        self.unfollow();
        let Ok(session) = self.mgr.GetCurrentSession() else { return };
        let (p, b, t) = (Arc::clone(&self.sink), Arc::clone(&self.sink), Arc::clone(&self.sink));
        let tokens = (|| -> Result<[i64; 3]> {
            Ok([
                session.MediaPropertiesChanged(&TypedEventHandler::<Session, MediaPropertiesChangedEventArgs>::new(
                    move |_, _| {
                        p(MediaSignal::PropertiesChanged);
                        Ok(())
                    },
                ))?,
                session.PlaybackInfoChanged(&TypedEventHandler::<Session, PlaybackInfoChangedEventArgs>::new(
                    move |_, _| {
                        b(MediaSignal::PlaybackChanged);
                        Ok(())
                    },
                ))?,
                session.TimelinePropertiesChanged(
                    &TypedEventHandler::<Session, TimelinePropertiesChangedEventArgs>::new(move |_, _| {
                        t(MediaSignal::TimelineChanged);
                        Ok(())
                    }),
                )?,
            ])
        })();
        match tokens {
            Ok(tokens) => self.session = Some((session, tokens)),
            Err(e) => tracing::warn!(error = %e, "media session events unavailable"),
        }
    }

    fn unfollow(&mut self) {
        if let Some((s, [a, b, c])) = self.session.take() {
            let _ = s.RemoveMediaPropertiesChanged(a);
            let _ = s.RemovePlaybackInfoChanged(b);
            let _ = s.RemoveTimelinePropertiesChanged(c);
        }
    }
}

impl Drop for MediaWatcher {
    fn drop(&mut self) {
        self.unfollow();
        let _ = self.mgr.RemoveCurrentSessionChanged(self.mgr_tokens[0]);
        let _ = self.mgr.RemoveSessionsChanged(self.mgr_tokens[1]);
    }
}
