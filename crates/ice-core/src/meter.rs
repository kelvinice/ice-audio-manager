//! Peak levels for the notch visualizer (plan.md Phase 3, F17).
//!
//! Levels are transient, not state: they never enter [`crate::engine::EngineSnapshot`] or the revisioned
//! delta stream, whose consumers re-read the whole snapshot per event. Instead a consumer holds a
//! [`MeterSubscription`] while the visualizer is on screen.
//!
//! **Stop condition (code-practices §2):** an engine reads levels from Windows only while at least one
//! subscription is alive. Dropping the last one stops the polling within one [`METER_INTERVAL`], even during
//! silence, when no frame is sent that could fail.

use std::sync::mpsc::{Receiver, RecvError, RecvTimeoutError, SyncSender, TryRecvError, TrySendError, sync_channel};
use std::sync::{Arc, Weak};
use std::time::Duration;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Time between two reads: 30 Hz, the cap in plan.md and code-practices §2.
pub const METER_INTERVAL: Duration = Duration::from_millis(33);
/// Number of log-spaced spectrum bands (45 Hz – 14 kHz) carried in [`MeterFrame::bands`] (D-26).
pub const SPECTRUM_BANDS: usize = 14;
/// Frames queued per subscription. A consumer that falls behind loses frames, never memory.
const BUFFER: usize = 4;
/// Smallest change worth sending (well under one pixel of bar height).
const MIN_CHANGE: f32 = 0.005;
/// Below this every channel counts as silent.
const SILENT: f32 = 0.001;

/// One reading of the default playback device's meter.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct MeterFrame {
    /// Highest peak across channels, 0.0–1.0 linear amplitude, as Windows reports it.
    pub peak: f32,
    /// Per-channel peaks (0.0–1.0) in the endpoint's channel order (front left, front right, …).
    /// Empty when Windows doesn't report channels.
    pub channels: Vec<f32>,
    /// Optional 14-band log-spaced spectrum levels (0.0–1.0, ~45 Hz – 14 kHz) from local WASAPI loopback (D-26).
    /// `None` when using peak-meter-only fallback (`IAudioMeterInformation` or Privacy toggle off).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bands: Option<Vec<f32>>,
}

impl MeterFrame {
    /// Builds a peak-only frame from raw readings, clamping each value to 0.0–1.0 (NaN counts as 0).
    #[must_use]
    pub fn new(peak: f32, channels: Vec<f32>) -> Self {
        let clamp = |v: f32| if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
        Self { peak: clamp(peak), channels: channels.into_iter().map(clamp).collect(), bands: None }
    }

    /// Builds a frame with both endpoint peaks and 14-band spectrum levels (D-26).
    #[must_use]
    pub fn with_bands(peak: f32, channels: Vec<f32>, bands: Vec<f32>) -> Self {
        let clamp = |v: f32| if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
        Self {
            peak: clamp(peak),
            channels: channels.into_iter().map(clamp).collect(),
            bands: Some(bands.into_iter().map(clamp).collect()),
        }
    }

    #[must_use]
    pub fn is_silent(&self) -> bool {
        self.peak < SILENT
            && self.channels.iter().all(|c| *c < SILENT)
            && self.bands.as_ref().is_none_or(|b| b.iter().all(|v| *v < SILENT))
    }

    /// Whether a consumer showing `self` would visibly change on `next`.
    fn differs(&self, next: &Self) -> bool {
        if self.is_silent() && next.is_silent() {
            return false;
        }
        let bands_differ = match (&self.bands, &next.bands) {
            (None, None) => false,
            (Some(a), Some(b)) => a.len() != b.len() || a.iter().zip(b).any(|(x, y)| (x - y).abs() >= MIN_CHANGE),
            _ => true,
        };
        bands_differ
            || self.channels.len() != next.channels.len()
            || (self.peak - next.peak).abs() >= MIN_CHANGE
            || self.channels.iter().zip(&next.channels).any(|(a, b)| (a - b).abs() >= MIN_CHANGE)
    }
}

/// A new subscription and the sending half that the engine keeps.
#[must_use]
pub fn channel() -> (MeterSink, MeterSubscription) {
    let (tx, frames) = sync_channel(BUFFER);
    let alive = Arc::new(());
    (MeterSink { tx, alive: Arc::downgrade(&alive), last: None }, MeterSubscription { frames, _alive: alive })
}

/// The consumer's end. Keep it while the visualizer is visible and drop it to stop the meter.
pub struct MeterSubscription {
    frames: Receiver<MeterFrame>,
    /// Its strong count tells the engine the subscription still exists, without sending anything.
    _alive: Arc<()>,
}

impl MeterSubscription {
    /// Waits for the next frame.
    ///
    /// # Errors
    /// The engine stopped (shut down or restarted). Ask it for a new subscription.
    pub fn recv(&self) -> Result<MeterFrame, RecvError> {
        self.frames.recv()
    }

    /// # Errors
    /// `Timeout` when the level didn't change, or `Disconnected` when the engine stopped.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<MeterFrame, RecvTimeoutError> {
        self.frames.recv_timeout(timeout)
    }

    /// # Errors
    /// `Empty` when the level didn't change, or `Disconnected` when the engine stopped.
    pub fn try_recv(&self) -> Result<MeterFrame, TryRecvError> {
        self.frames.try_recv()
    }
}

/// The engine's end of one subscription.
pub struct MeterSink {
    tx: SyncSender<MeterFrame>,
    alive: Weak<()>,
    /// Last frame the consumer received, to send only changes.
    last: Option<MeterFrame>,
}

impl MeterSink {
    /// The consumer dropped its [`MeterSubscription`].
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.alive.strong_count() == 0
    }

    /// Sends `frame` if it visibly differs from the last one delivered. The first frame is always sent, and
    /// continued silence is sent once. When the consumer's queue is full the frame is skipped and offered
    /// again next time. Returns `false` once the subscription is gone, so the engine can forget this sink.
    pub fn offer(&mut self, frame: &MeterFrame) -> bool {
        if self.is_closed() {
            return false;
        }
        if self.last.as_ref().is_some_and(|last| !last.differs(frame)) {
            return true;
        }
        match self.tx.try_send(frame.clone()) {
            Ok(()) => {
                self.last = Some(frame.clone());
                true
            }
            Err(TrySendError::Full(_)) => true,
            Err(TrySendError::Disconnected(_)) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(l: f32, r: f32) -> MeterFrame {
        MeterFrame::new(l.max(r), vec![l, r])
    }

    #[test]
    fn values_are_clamped_and_nan_is_silence() {
        let f = MeterFrame::new(f32::NAN, vec![1.5, -0.2]);
        assert_eq!(f, MeterFrame { peak: 0.0, channels: vec![1.0, 0.0], bands: None });
        assert!(MeterFrame::new(0.0005, vec![0.0, 0.0009]).is_silent());
        assert!(!frame(0.0, 0.2).is_silent());
    }

    #[test]
    fn band_changes_are_delivered_even_when_peak_is_unchanged() {
        let (mut sink, sub) = channel();
        let low = MeterFrame::with_bands(0.5, vec![0.5, 0.5], vec![0.5, 0.4, 0.0, 0.0]);
        let high = MeterFrame::with_bands(0.5, vec![0.5, 0.5], vec![0.0, 0.0, 0.4, 0.5]);
        let tiny = MeterFrame::with_bands(0.5, vec![0.5, 0.5], vec![0.0, 0.0, 0.402, 0.5]);
        assert!(sink.offer(&low));
        assert!(sink.offer(&high), "same-peak spectrum shift must not be suppressed");
        assert!(sink.offer(&tiny), "sub-threshold band delta is skipped");
        assert_eq!(sub.try_recv().unwrap(), low);
        assert_eq!(sub.try_recv().unwrap(), high);
        assert_eq!(sub.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn only_visible_changes_are_sent() {
        let (mut sink, sub) = channel();
        assert!(sink.offer(&frame(0.5, 0.4)));
        assert!(sink.offer(&frame(0.502, 0.4)), "a tiny change is skipped, not an error");
        assert!(sink.offer(&frame(0.6, 0.4)));
        assert_eq!(sub.try_recv().unwrap(), frame(0.5, 0.4));
        assert_eq!(sub.try_recv().unwrap(), frame(0.6, 0.4));
        assert_eq!(sub.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn silence_is_sent_once() {
        let (mut sink, sub) = channel();
        sink.offer(&frame(0.3, 0.3));
        for _ in 0..10 {
            sink.offer(&MeterFrame::default());
        }
        assert_eq!(sub.try_recv().unwrap(), frame(0.3, 0.3));
        assert!(sub.try_recv().unwrap().is_silent());
        assert_eq!(sub.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn a_full_queue_retries_the_frame_later() {
        let (mut sink, sub) = channel();
        for i in 0..BUFFER {
            #[expect(clippy::cast_precision_loss, reason = "i < BUFFER, tiny")]
            sink.offer(&frame(0.1 * (i as f32 + 1.0), 0.0));
        }
        // Queue full: the drop to silence must not be marked as delivered.
        sink.offer(&MeterFrame::default());
        for _ in 0..BUFFER {
            assert!(!sub.try_recv().unwrap().is_silent());
        }
        sink.offer(&MeterFrame::default());
        assert!(sub.try_recv().unwrap().is_silent());
    }

    #[test]
    fn dropping_the_subscription_closes_the_sink_without_a_send() {
        let (mut sink, sub) = channel();
        sink.offer(&MeterFrame::default());
        assert!(!sink.is_closed());
        drop(sub);
        assert!(sink.is_closed(), "detectable during silence, when nothing is sent");
        assert!(!sink.offer(&frame(0.9, 0.9)));
    }

    #[test]
    fn json_shape() {
        let json = serde_json::to_string(&MeterFrame::new(0.5, vec![0.5, 0.25])).unwrap();
        assert_eq!(json, r#"{"peak":0.5,"channels":[0.5,0.25]}"#);
    }
}
