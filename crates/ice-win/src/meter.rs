//! Peak meter (`IAudioMeterInformation`) + optional local WASAPI loopback 14-band spectrum (D-26).
//!
//! [`MeterHub`] serves [`ice_core::meter`] subscriptions from one `ice-meter` thread. The thread starts on
//! the first subscription and **exits once every subscription is dropped** (code-practices §2: the only
//! polling besides the cursor poll, ≤ 30 Hz, only while the visualizer is visible).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use ice_core::device::{Flow, Role};
use ice_core::meter::{METER_INTERVAL, MeterFrame, MeterSink, MeterSubscription};
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::IMMDevice;
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::core::Result;

use crate::com::ComGuard;
use crate::endpoints::Endpoints;
use crate::loopback::LoopbackCapture;

/// Channels read at most (7.1). More would only be folded into the same few bars.
const MAX_CHANNELS: usize = 8;
/// Wait before re-trying a device that failed to activate or read, unless the default changes first.
const RETRY_AFTER: Duration = Duration::from_secs(1);
/// If peak meter reports audio (`>= 0.02`) while loopback stays strictly silent for this many ticks
/// (~330 ms, e.g. DRM or exclusive-mode stream), emit peak-only frames so the UI falls back to `A1`.
const SILENT_LOOPBACK_FALLBACK_TICKS: u8 = 10;
const AUDIBLE_PEAK_THRESHOLD: f32 = 0.02;
const BAND_ACTIVE_EPS: f32 = 0.0005;

pub struct Meter(IAudioMeterInformation);

impl Meter {
    /// # Errors
    /// Fails if the endpoint can't be activated.
    pub fn new(dev: &IMMDevice) -> Result<Self> {
        // SAFETY: documented activation on a valid endpoint.
        Ok(Self(unsafe { dev.Activate(CLSCTX_ALL, None)? }))
    }

    /// Current peak sample value (0.0–1.0) across channels.
    ///
    /// # Errors
    /// Fails if the endpoint was invalidated.
    pub fn peak(&self) -> Result<f32> {
        // SAFETY: plain getter on a valid interface.
        unsafe { self.0.GetPeakValue() }
    }

    /// Overall and per-channel peaks (at most [`MAX_CHANNELS`]).
    ///
    /// # Errors
    /// Fails if the endpoint was invalidated (`AUDCLNT_E_DEVICE_INVALIDATED`).
    pub fn frame(&self) -> Result<MeterFrame> {
        let peak = self.peak()?;
        // SAFETY: plain getter on a valid interface.
        let count = unsafe { self.0.GetMeteringChannelCount()? };
        let mut channels = vec![0.0_f32; usize::try_from(count).unwrap_or(0).min(MAX_CHANNELS)];
        if !channels.is_empty() {
            // SAFETY: the slice is valid for writes; the windows crate passes its length as the channel count,
            // which is ≤ the count Windows reported, as the API requires.
            unsafe { self.0.GetChannelsPeakValues(&mut channels)? };
        }
        Ok(MeterFrame::new(peak, channels))
    }
}

/// Shares one meter thread between all subscriptions.
pub struct MeterHub(Mutex<HubState>);

struct HubState {
    sinks: Vec<MeterSink>,
    /// Bumped by [`MeterHub::rebind`] or [`MeterHub::set_spectrum_enabled`]; the thread re-activates the
    /// default device and resets any filter state when it changes.
    generation: u64,
    /// D-26 Privacy switch: when `false`, never opens WASAPI loopback (`IAudioMeterInformation` peaks only).
    spectrum_enabled: bool,
    running: bool,
}

impl MeterHub {
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(HubState {
            sinks: Vec::new(),
            generation: 0,
            spectrum_enabled: true,
            running: false,
        })))
    }

    fn lock(&self) -> MutexGuard<'_, HubState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Enables or disables local WASAPI loopback spectrum capture (D-26 Privacy switch).
    /// Immediately drops or opens the loopback stream on the next tick.
    pub fn set_spectrum_enabled(&self, enabled: bool) {
        let mut st = self.lock();
        if st.spectrum_enabled != enabled {
            st.spectrum_enabled = enabled;
            st.generation = st.generation.wrapping_add(1);
        }
    }

    /// The default playback device may have changed, or been reconnected under the same ID (which leaves
    /// old interfaces stale, review ENG-07). The meter re-activates it on its next tick.
    pub fn rebind(&self) {
        let mut st = self.lock();
        st.generation = st.generation.wrapping_add(1);
    }

    /// Ends every subscription (their `recv` fails) and lets the thread exit. For engine shutdown.
    pub fn close(&self) {
        self.lock().sinks.clear();
    }

    /// A new subscription; starts the meter thread if it isn't running. `None` if the thread can't start.
    pub fn subscribe(self: &Arc<Self>) -> Option<MeterSubscription> {
        let (sink, subscription) = ice_core::meter::channel();
        let mut st = self.lock();
        st.sinks.push(sink);
        if !st.running {
            let hub = Arc::clone(self);
            if let Err(error) = std::thread::Builder::new().name("ice-meter".into()).spawn(move || hub.run()) {
                tracing::warn!(%error, "meter thread failed to start; visualizer disabled");
                st.sinks.pop();
                return None;
            }
            st.running = true;
        }
        Some(subscription)
    }

    /// The meter thread. Ends when no subscription is left.
    fn run(&self) {
        // Declared first so it drops last, after every COM object below.
        let com = ComGuard::init_mta();
        let eps = com.as_ref().ok().and_then(|_| Endpoints::new().ok());
        if eps.is_none() {
            tracing::warn!("meter thread could not reach the audio service; sending silence");
        }
        let mut meter: Option<Meter> = None;
        let mut loopback: Option<LoopbackCapture> = None;
        let mut bound_generation = None;
        let mut retry_at = Instant::now();
        let mut silent_loopback_ticks: u8 = 0;
        loop {
            let (generation, spectrum_enabled) = {
                let mut st = self.lock();
                st.sinks.retain(|s| !s.is_closed());
                if st.sinks.is_empty() {
                    st.running = false;
                    tracing::debug!("meter stopped: no subscribers");
                    return;
                }
                (st.generation, st.spectrum_enabled)
            };
            let now = Instant::now();
            if bound_generation != Some(generation) || (meter.is_none() && now >= retry_at) {
                bound_generation = Some(generation);
                // Discard previous loopback & filter state on every rebind or fallback (D-26 Codex amendment).
                loopback = None;
                silent_loopback_ticks = 0;
                let dev = eps.as_ref().and_then(|eps| {
                    eps.default_device(Flow::Render, Role::Multimedia)
                        .inspect_err(|e| tracing::debug!(error = %e, "no default playback device for meter"))
                        .ok()
                });
                meter = dev.as_ref().and_then(|d| {
                    Meter::new(d)
                        .inspect_err(|e| tracing::debug!(error = %e, "no meter for the default playback device"))
                        .ok()
                });
                if spectrum_enabled && let Some(d) = dev.as_ref() {
                    loopback = LoopbackCapture::new(d)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "WASAPI loopback unavailable; falling back to peak meter");
                        })
                        .ok();
                }
                retry_at = now + RETRY_AFTER;
            }
            let mut frame = match meter.as_ref().map(Meter::frame) {
                Some(Ok(frame)) => frame,
                Some(Err(e)) => {
                    tracing::debug!(error = %e, "meter read failed; re-activating after a pause");
                    meter = None;
                    loopback = None;
                    silent_loopback_ticks = 0;
                    retry_at = now + RETRY_AFTER;
                    MeterFrame::default()
                }
                None => MeterFrame::default(),
            };
            if let Some(cap) = loopback.as_mut() {
                match cap.drain_bands() {
                    Ok(bands) => {
                        let bands_active = bands.iter().any(|&v| v >= BAND_ACTIVE_EPS);
                        if frame.peak >= AUDIBLE_PEAK_THRESHOLD && !bands_active {
                            silent_loopback_ticks = silent_loopback_ticks.saturating_add(1);
                        } else {
                            silent_loopback_ticks = 0;
                        }
                        if silent_loopback_ticks < SILENT_LOOPBACK_FALLBACK_TICKS {
                            frame = MeterFrame::with_bands(frame.peak, frame.channels, bands.to_vec());
                        }
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "WASAPI loopback read failed; falling back to peak meter");
                        loopback = None;
                        silent_loopback_ticks = 0;
                    }
                }
            }
            self.lock().sinks.retain_mut(|s| s.offer(&frame));
            std::thread::sleep(METER_INTERVAL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running(hub: &MeterHub) -> bool {
        hub.lock().running
    }

    /// Works with or without an audio device (silence then). Run with `--nocapture` while music plays to
    /// see real levels.
    #[test]
    fn thread_runs_only_while_subscribed() {
        let hub = MeterHub::new();
        let a = hub.subscribe().expect("thread starts");
        let b = hub.subscribe().expect("second subscriber shares the thread");
        let first = a.recv_timeout(Duration::from_secs(2)).expect("a first frame always arrives");
        println!("first frame: {first:?}");
        assert!(b.recv_timeout(Duration::from_secs(2)).is_ok());
        hub.rebind();
        drop(a);
        std::thread::sleep(METER_INTERVAL * 4);
        assert!(running(&hub), "one subscriber is left");
        drop(b);
        std::thread::sleep(METER_INTERVAL * 4);
        assert!(!running(&hub), "stops within a tick of the last drop, even in silence");
        // And starts again on demand.
        let c = hub.subscribe().expect("restarts");
        assert!(c.recv_timeout(Duration::from_secs(2)).is_ok());
        hub.close();
        assert!(c.recv_timeout(Duration::from_secs(1)).is_err(), "close ends subscriptions");
        std::thread::sleep(METER_INTERVAL * 4);
        assert!(!running(&hub));
    }

    /// Phase 3 budget: < 1% CPU while visible. Run on demand, with music playing:
    /// `cargo test -p ice-win --release meter_cpu -- --ignored --nocapture`
    #[test]
    #[ignore = "10 s measurement; run by hand"]
    fn meter_cpu_cost() {
        use crate::proc_metrics::{cpu_percent_between, current_pid, sample_pids};
        let pid = [(current_pid(), "test".to_string())];
        let hub = MeterHub::new();
        let sub = hub.subscribe().expect("thread starts");
        let (start, before) = (Instant::now(), sample_pids(&pid));
        let mut frames = 0;
        while start.elapsed() < Duration::from_secs(10) {
            if sub.recv_timeout(Duration::from_millis(100)).is_ok() {
                frames += 1;
            }
        }
        let cpu = cpu_percent_between(&before, &sample_pids(&pid), start.elapsed());
        println!("meter: {frames} frames in 10 s, {cpu:.3}% CPU (whole machine)");
        assert!(cpu < 1.0);
    }
}
