//! Pure 14-band log-spaced 4th-order IIR biquad spectrum analyser (D-26, A2).
//!
//! Designed for shared-mode WASAPI loopback packets on `ice-meter` (`crates/ice-win/src/loopback.rs`),
//! with zero external DSP crates, zero per-tick allocations, and sample-rate-aware RBJ band-pass
//! coefficients computed once when binding an endpoint (`44.1 / 48 / 96 / 192 kHz`).
//!
//! Each band cascades two identical 2nd-order constant-0 dB-peak RBJ band-pass sections (`Q = 1.4`)
//! so adjacent bands spaced ~0.64 octaves apart drop by ~−8.4 dB (38 % overlap) and two bands away
//! by ~−19 dB — preventing the clumping seen with single 2nd-order sections (D-26 Claude amendment).

use std::f32::consts::{PI, SQRT_2};

use crate::meter::SPECTRUM_BANDS;

/// Lowest band centre frequency in Hz (sub-bass / kick).
pub const LOW_BAND_HZ: f32 = 45.0;
/// Highest band centre frequency in Hz (air / hi-hat shimmer).
pub const HIGH_BAND_HZ: f32 = 14_000.0;
/// Quality factor per cascaded biquad section (D-26: two cascaded Q = 1.4 sections = 4th-order).
const SECTION_Q: f32 = 1.4;
/// Upper cutoff ratio against Nyquist so unusual low-rate devices (e.g. Bluetooth HFP) stay stable.
const MAX_NYQUIST_RATIO: f32 = 0.45;
/// Subnormal flush floor so decaying filter tails never trigger denormal CPU spikes.
const DENORMAL_FLOOR: f32 = 1e-20;

/// Single 2nd-order band-pass biquad (RBJ constant 0 dB peak gain, `b1 == 0`) in Transposed Direct Form II.
#[derive(Debug, Clone, Copy)]
struct Biquad {
    b0: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn band_pass(sample_rate: f32, center_hz: f32, q: f32) -> Self {
        let clamped_hz = center_hz.clamp(20.0, (sample_rate * MAX_NYQUIST_RATIO).max(20.0));
        let w0 = 2.0 * PI * clamped_hz / sample_rate;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        let alpha = sin_w0 / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self { b0: alpha / a0, b2: -alpha / a0, a1: (-2.0 * cos_w0) / a0, a2: (1.0 - alpha) / a0, z1: 0.0, z2: 0.0 }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        let mut z1 = -self.a1 * y + self.z2;
        let mut z2 = self.b2 * x - self.a2 * y;
        if z1.abs() < DENORMAL_FLOOR {
            z1 = 0.0;
        }
        if z2.abs() < DENORMAL_FLOOR {
            z2 = 0.0;
        }
        self.z1 = z1;
        self.z2 = z2;
        y
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// 4th-order band-pass filter formed by cascading two [`Biquad`] sections.
#[derive(Debug, Clone, Copy)]
struct BandFilter {
    first: Biquad,
    second: Biquad,
    sum_sq: f32,
}

impl BandFilter {
    fn new(sample_rate: f32, center_hz: f32) -> Self {
        let bq = Biquad::band_pass(sample_rate, center_hz, SECTION_Q);
        Self { first: bq, second: bq, sum_sq: 0.0 }
    }

    #[inline]
    fn push(&mut self, x: f32) {
        let y1 = self.first.process(x);
        let y2 = self.second.process(y1);
        self.sum_sq += y2 * y2;
    }

    fn reset(&mut self) {
        self.first.reset();
        self.second.reset();
        self.sum_sq = 0.0;
    }
}

/// Log-spaced centre frequencies for the [`SPECTRUM_BANDS`] bands (`45 Hz .. 14,000 Hz`).
#[must_use]
pub fn band_centers_hz() -> [f32; SPECTRUM_BANDS] {
    let ratio = (HIGH_BAND_HZ / LOW_BAND_HZ).powf(1.0 / (SPECTRUM_BANDS as f32 - 1.0));
    let mut out = [0.0_f32; SPECTRUM_BANDS];
    let mut f = LOW_BAND_HZ;
    for slot in &mut out {
        *slot = f;
        f *= ratio;
    }
    out
}

/// Stateful 14-band 4th-order IIR spectrum analyser.
///
/// Feed mono-mixed PCM samples (`-1.0..=1.0`) with [`Self::push_sample`], then call
/// [`Self::finish_block`] once per meter tick to obtain the 14 band magnitudes (`0.0..=1.0`).
/// Call [`Self::reset`] whenever the endpoint rebinds, falls back, or enters sustained silence.
#[derive(Debug, Clone)]
pub struct SpectrumAnalyzer {
    sample_rate: u32,
    bands: [BandFilter; SPECTRUM_BANDS],
    samples_in_block: u32,
}

impl SpectrumAnalyzer {
    /// Builds a fresh filter bank tuned to `sample_rate` (clamped to `>= 8_000 Hz`).
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        let safe_rate = sample_rate.max(8_000);
        #[expect(clippy::cast_precision_loss, reason = "audio sample rates <= 384 kHz are exact in f32")]
        let fs = safe_rate as f32;
        let centers = band_centers_hz();
        let bands = std::array::from_fn(|i| BandFilter::new(fs, centers[i]));
        Self { sample_rate: safe_rate, bands, samples_in_block: 0 }
    }

    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Processes one mono audio sample (`-1.0..=1.0`). Non-finite inputs count as `0.0`.
    #[inline]
    pub fn push_sample(&mut self, sample: f32) {
        let x = if sample.is_finite() { sample.clamp(-4.0, 4.0) } else { 0.0 };
        for band in &mut self.bands {
            band.push(x);
        }
        self.samples_in_block = self.samples_in_block.saturating_add(1);
    }

    /// Returns the RMS-equivalent peak amplitude (`0.0..=1.0`, so a full-scale sine at `fc` reads ~1.0)
    /// for each of the 14 bands over the samples pushed since the last call, and clears the block accumulators
    /// while preserving filter state across capture packets.
    #[must_use]
    pub fn finish_block(&mut self) -> [f32; SPECTRUM_BANDS] {
        let count = self.samples_in_block;
        self.samples_in_block = 0;
        let mut out = [0.0_f32; SPECTRUM_BANDS];
        if count == 0 {
            for band in &mut self.bands {
                band.sum_sq = 0.0;
            }
            return out;
        }
        #[expect(clippy::cast_precision_loss, reason = "block sample counts at 30 Hz are <= ~15,000")]
        let inv_n = 1.0 / (count as f32);
        for (slot, band) in out.iter_mut().zip(&mut self.bands) {
            let rms = (band.sum_sq * inv_n).sqrt();
            band.sum_sq = 0.0;
            *slot = (rms * SQRT_2).clamp(0.0, 1.0);
        }
        out
    }

    /// Discards all filter delay state and pending block energy (used on endpoint rebind, fallback, or silence).
    pub fn reset(&mut self) {
        self.samples_in_block = 0;
        for band in &mut self.bands {
            band.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_sine(analyzer: &mut SpectrumAnalyzer, freq_hz: f32, amp: f32, seconds: f32) -> [f32; SPECTRUM_BANDS] {
        #[expect(clippy::cast_precision_loss, reason = "test sample rates are exact in f32")]
        let fs = analyzer.sample_rate() as f32;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "positive test duration")]
        let total = (fs * seconds).round() as usize;
        let block = (analyzer.sample_rate() / 30).max(1) as usize;
        let mut last = [0.0_f32; SPECTRUM_BANDS];
        for n in 0..total {
            #[expect(clippy::cast_precision_loss, reason = "short test buffer")]
            let t = n as f32 / fs;
            analyzer.push_sample(amp * (2.0 * PI * freq_hz * t).sin());
            if (n + 1) % block == 0 {
                last = analyzer.finish_block();
            }
        }
        last
    }

    fn peak_band(levels: &[f32; SPECTRUM_BANDS]) -> usize {
        levels.iter().enumerate().max_by(|(_, a), (_, b)| a.total_cmp(b)).map_or(0, |(i, _)| i)
    }

    /// D-26 Codex acceptance check: shifting from a steady low tone to a steady high tone at the exact same
    /// amplitude moves active energy across the spectrum, then settles cleanly to silence.
    #[test]
    fn low_to_high_tone_shifts_active_bands_and_settles_on_silence() {
        let mut a = SpectrumAnalyzer::new(48_000);
        let low = feed_sine(&mut a, 109.0, 0.5, 0.4);
        assert_eq!(peak_band(&low), 2, "109 Hz should peak on band 2: {low:?}");
        assert!(low[2] > 0.45 && low[2] < 0.55, "unity gain at centre: {:.3}", low[2]);
        assert!(low[11] < 0.01, "treble stays quiet during low tone: {:.4}", low[11]);

        let high = feed_sine(&mut a, 5_798.0, 0.5, 0.4);
        assert_eq!(peak_band(&high), 11, "5.8 kHz should peak on band 11: {high:?}");
        assert!(high[11] > 0.45 && high[11] < 0.55, "unity gain at treble centre: {:.3}", high[11]);
        assert!(high[2] < 0.01, "bass settles during high tone: {:.4}", high[2]);

        let silent = feed_sine(&mut a, 0.0, 0.0, 0.3);
        assert!(silent.iter().all(|v| *v < 0.001), "settles cleanly to rest on silence: {silent:?}");
    }

    /// D-26 Claude 4th-order selectivity check: adjacent band overlap <= ~42 % (-7.5 dB), two away <= ~13 % (-17.5 dB).
    #[test]
    fn fourth_order_selectivity_separates_adjacent_bands() {
        let mut a = SpectrumAnalyzer::new(48_000);
        let centers = band_centers_hz();
        let levels = feed_sine(&mut a, centers[6], 0.8, 0.5);
        let center_lvl = levels[6];
        assert!(center_lvl > 0.72, "band 6 peak: {center_lvl:.3}");
        let adj_ratio = levels[5].max(levels[7]) / center_lvl;
        let two_away_ratio = levels[4].max(levels[8]) / center_lvl;
        assert!(adj_ratio < 0.44, "adjacent band overlap should be ~38%, got {:.1}%", adj_ratio * 100.0);
        assert!(two_away_ratio < 0.15, "two-away overlap should be ~11%, got {:.1}%", two_away_ratio * 100.0);
    }

    #[test]
    fn coefficients_track_endpoint_sample_rates_and_reset_clears_state() {
        for rate in [44_100, 48_000, 96_000, 192_000] {
            let mut a = SpectrumAnalyzer::new(rate);
            let levels = feed_sine(&mut a, 991.0, 0.6, 0.35);
            assert_eq!(peak_band(&levels), 7, "991 Hz peaks on band 7 at {rate} Hz: {levels:?}");
            a.reset();
            for _ in 0..160 {
                a.push_sample(0.0);
            }
            let after_reset = a.finish_block();
            assert!(after_reset.iter().all(|v| *v == 0.0), "reset discards filter state at {rate} Hz");
        }
    }
}
