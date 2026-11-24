//! Shared-mode WASAPI render-endpoint loopback capture for the live spectrum visualizer (D-26, A2).
//!
//! Privacy contract (`docs/code-practices.md` §1.9):
//! - Opened ONLY on the default **render** endpoint (`AUDCLNT_STREAMFLAGS_LOOPBACK`) while a visualizer
//!   subscription is alive and `visualizer_spectrum` is enabled. Never opens a capture/microphone device.
//! - PCM samples are immediately reduced in memory on the `ice-meter` thread into 14 scalar band levels
//!   (`0.0–1.0`) via [`ice_core::spectrum::SpectrumAnalyzer`] and discarded — never saved, logged, or sent.
//! - Drained non-blockingly on the existing 30 Hz `ice-meter` tick (no extra thread or timer).

use ice_core::meter::SPECTRUM_BANDS;
use ice_core::spectrum::SpectrumAnalyzer;
use windows::Win32::Foundation::E_NOTIMPL;
use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, IAudioCaptureClient,
    IAudioClient, IMMDevice, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoTaskMemFree};
use windows::core::{GUID, Result};

/// Buffer duration requested from WASAPI in 100-nanosecond units (`100 ms` >= 3× the 33 ms meter tick).
const BUFFER_DURATION_100NS: i64 = 1_000_000;
/// Cap packets drained per tick so a stalled thread can never spin unboundedly.
const MAX_PACKETS_PER_TICK: usize = 32;
/// After this many consecutive empty ticks (`>= 66 ms` at 30 Hz), treat loopback silence as quiet rest.
const SILENT_EMPTY_TICKS: u8 = 2;

const WAVE_FORMAT_PCM_TAG: u16 = 0x0001;
const WAVE_FORMAT_IEEE_FLOAT_TAG: u16 = 0x0003;
const WAVE_FORMAT_EXTENSIBLE_TAG: u16 = 0xFFFE;
const SUBTYPE_PCM: GUID = GUID::from_u128(0x0000_0001_0000_0010_8000_00aa_0038_9b71);
const SUBTYPE_IEEE_FLOAT: GUID = GUID::from_u128(0x0000_0003_0000_0010_8000_00aa_0038_9b71);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SampleFormat {
    Float32,
    Pcm16,
    Pcm24,
    Pcm32,
}

impl SampleFormat {
    fn bytes_per_sample(self) -> usize {
        match self {
            Self::Pcm16 => 2,
            Self::Pcm24 => 3,
            Self::Float32 | Self::Pcm32 => 4,
        }
    }
}

/// RAII wrapper ensuring `CoTaskMemFree` runs for the `WAVEFORMATEX*` returned by `GetMixFormat`.
struct MixFormatGuard(*mut WAVEFORMATEX);

impl Drop for MixFormatGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: pointer was allocated by `IAudioClient::GetMixFormat` via the COM task allocator.
            unsafe { CoTaskMemFree(Some(self.0.cast())) };
        }
    }
}

/// RAII wrapper ensuring `IAudioCaptureClient::ReleaseBuffer` always pairs with `GetBuffer`.
struct PacketGuard<'a> {
    capture: &'a IAudioCaptureClient,
    frames: u32,
}

impl Drop for PacketGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: `self.frames` was returned by the matching `GetBuffer` call on `self.capture`.
        let _ = unsafe { self.capture.ReleaseBuffer(self.frames) };
    }
}

/// Active shared-mode render loopback capture paired with a sample-rate-tuned 14-band filter bank.
pub struct LoopbackCapture {
    client: IAudioClient,
    capture: IAudioCaptureClient,
    channels: usize,
    block_align: usize,
    format: SampleFormat,
    analyzer: SpectrumAnalyzer,
    empty_ticks: u8,
    last_bands: [f32; SPECTRUM_BANDS],
}

impl LoopbackCapture {
    /// Activates shared-mode loopback on a **render** endpoint and tunes the 14-band filter bank to its mix rate.
    ///
    /// # Errors
    /// Fails if the device refuses shared-mode loopback (e.g. exclusive mode, virtual driver, or unsupported format).
    pub fn new(dev: &IMMDevice) -> Result<Self> {
        // SAFETY: documented activation of `IAudioClient` on a valid render endpoint.
        let client: IAudioClient = unsafe { dev.Activate(CLSCTX_ALL, None)? };
        // SAFETY: returns a COM-allocated `WAVEFORMATEX` that `MixFormatGuard` frees on drop.
        let mix_ptr = unsafe { client.GetMixFormat()? };
        let _mix_guard = MixFormatGuard(mix_ptr);
        if mix_ptr.is_null() {
            return Err(E_NOTIMPL.into());
        }

        // SAFETY: `mix_ptr` is non-null and valid for reading `WAVEFORMATEX` (and `WAVEFORMATEXTENSIBLE`
        // when `wFormatTag == WAVE_FORMAT_EXTENSIBLE_TAG` and `cbSize >= 22`).
        let (sample_rate, channels, block_align, format) = unsafe { parse_mix_format(mix_ptr)? };

        // SAFETY: shared-mode loopback (`AUDCLNT_STREAMFLAGS_LOOPBACK`) on the render device's own mix format.
        unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                BUFFER_DURATION_100NS,
                0,
                mix_ptr,
                None,
            )?;
        }
        // SAFETY: `IAudioCaptureClient` is available on an initialized loopback `IAudioClient`.
        let capture: IAudioCaptureClient = unsafe { client.GetService()? };
        // SAFETY: starts the initialized loopback stream.
        unsafe { client.Start()? };

        tracing::debug!(sample_rate, channels, ?format, "WASAPI loopback spectrum capture started");
        Ok(Self {
            client,
            capture,
            channels,
            block_align,
            format,
            analyzer: SpectrumAnalyzer::new(sample_rate),
            empty_ticks: 0,
            last_bands: [0.0; SPECTRUM_BANDS],
        })
    }

    /// Drains all queued loopback packets without blocking and returns the 14 band magnitudes (`0.0–1.0`).
    ///
    /// # Errors
    /// Fails if the endpoint was invalidated (`AUDCLNT_E_DEVICE_INVALIDATED`).
    pub fn drain_bands(&mut self) -> Result<[f32; SPECTRUM_BANDS]> {
        let mut total_frames: u32 = 0;
        let mut non_silent_frames: u32 = 0;
        let silent_flag = u32::try_from(AUDCLNT_BUFFERFLAGS_SILENT.0).unwrap_or(0);

        for _ in 0..MAX_PACKETS_PER_TICK {
            // SAFETY: non-blocking query on an active `IAudioCaptureClient`.
            let packet_frames = unsafe { self.capture.GetNextPacketSize()? };
            if packet_frames == 0 {
                break;
            }
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut num_frames: u32 = 0;
            let mut flags: u32 = 0;
            // SAFETY: valid out-pointers; `PacketGuard` ensures `ReleaseBuffer(num_frames)` runs.
            unsafe {
                self.capture.GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)?;
            }
            let _guard = PacketGuard { capture: &self.capture, frames: num_frames };
            if num_frames == 0 {
                continue;
            }
            total_frames = total_frames.saturating_add(num_frames);
            let is_silent = (flags & silent_flag) != 0 || data_ptr.is_null();
            if is_silent {
                continue;
            }
            let frame_count = usize::try_from(num_frames).unwrap_or(0);
            let Some(byte_len) = frame_count.checked_mul(self.block_align) else {
                continue;
            };
            // SAFETY: WASAPI guarantees `data_ptr` is readable for `num_frames * block_align` bytes until
            // `ReleaseBuffer` is called when `_guard` drops.
            let bytes = unsafe { std::slice::from_raw_parts(data_ptr, byte_len) };
            feed_bytes(&mut self.analyzer, self.format, self.channels, self.block_align, bytes, frame_count);
            non_silent_frames = non_silent_frames.saturating_add(num_frames);
        }

        if non_silent_frames > 0 {
            self.last_bands = self.analyzer.finish_block();
            self.empty_ticks = 0;
        } else if total_frames > 0 {
            // Explicit WASAPI silence packet: settle cleanly to rest and discard filter tails.
            self.analyzer.reset();
            self.last_bands = [0.0; SPECTRUM_BANDS];
            self.empty_ticks = 0;
        } else {
            // No packets queued: render endpoint is idle (or slight 1-tick scheduling jitter).
            self.empty_ticks = self.empty_ticks.saturating_add(1);
            if self.empty_ticks >= SILENT_EMPTY_TICKS {
                self.analyzer.reset();
                self.last_bands = [0.0; SPECTRUM_BANDS];
            } else {
                for v in &mut self.last_bands {
                    *v *= 0.6;
                }
            }
        }
        Ok(self.last_bands)
    }
}

fn feed_bytes(
    analyzer: &mut SpectrumAnalyzer,
    format: SampleFormat,
    channels: usize,
    block_align: usize,
    bytes: &[u8],
    frame_count: usize,
) {
    let bps = format.bytes_per_sample();
    let stereo = channels >= 2;
    for f in 0..frame_count {
        let base = f * block_align;
        let l = decode_sample(format, &bytes[base..base + bps]);
        let mono = if stereo {
            let r = decode_sample(format, &bytes[base + bps..base + 2 * bps]);
            0.5 * (l + r)
        } else {
            l
        };
        analyzer.push_sample(mono);
    }
}

impl Drop for LoopbackCapture {
    fn drop(&mut self) {
        // SAFETY: stops the active loopback stream before the COM interface is released.
        let _ = unsafe { self.client.Stop() };
    }
}

/// # Safety
/// `ptr` must point to a valid `WAVEFORMATEX` buffer allocated by Windows `GetMixFormat`.
unsafe fn parse_mix_format(ptr: *const WAVEFORMATEX) -> Result<(u32, usize, usize, SampleFormat)> {
    // SAFETY: caller guarantees `ptr` is valid for reading `WAVEFORMATEX` (unaligned-safe).
    let wfx = unsafe { ptr.read_unaligned() };
    let sample_rate = wfx.nSamplesPerSec;
    let channels = usize::from(wfx.nChannels);
    let block_align = usize::from(wfx.nBlockAlign);
    let bits = wfx.wBitsPerSample;
    let tag = wfx.wFormatTag;

    let format = match tag {
        WAVE_FORMAT_IEEE_FLOAT_TAG if bits == 32 => SampleFormat::Float32,
        WAVE_FORMAT_PCM_TAG => pcm_format(bits)?,
        WAVE_FORMAT_EXTENSIBLE_TAG if wfx.cbSize >= 22 => {
            let ext_ptr = ptr.cast::<WAVEFORMATEXTENSIBLE>();
            // SAFETY: `wFormatTag == 0xFFFE` and `cbSize >= 22` guarantees a full `WAVEFORMATEXTENSIBLE`.
            let subformat = unsafe { std::ptr::addr_of!((*ext_ptr).SubFormat).read_unaligned() };
            if subformat == SUBTYPE_IEEE_FLOAT && bits == 32 {
                SampleFormat::Float32
            } else if subformat == SUBTYPE_PCM {
                pcm_format(bits)?
            } else {
                return Err(E_NOTIMPL.into());
            }
        }
        _ => return Err(E_NOTIMPL.into()),
    };

    let min_stride = channels.saturating_mul(format.bytes_per_sample());
    if sample_rate == 0 || channels == 0 || min_stride == 0 || block_align < min_stride {
        return Err(E_NOTIMPL.into());
    }
    Ok((sample_rate, channels, block_align, format))
}

fn pcm_format(bits: u16) -> Result<SampleFormat> {
    match bits {
        16 => Ok(SampleFormat::Pcm16),
        24 => Ok(SampleFormat::Pcm24),
        32 => Ok(SampleFormat::Pcm32),
        _ => Err(E_NOTIMPL.into()),
    }
}

#[inline]
fn decode_sample(format: SampleFormat, bytes: &[u8]) -> f32 {
    match format {
        SampleFormat::Float32 => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        SampleFormat::Pcm16 => f32::from(i16::from_le_bytes([bytes[0], bytes[1]])) / 32_768.0,
        SampleFormat::Pcm24 => {
            // Sign-extend 24-bit little-endian signed integer into i32.
            let raw = i32::from_le_bytes([0, bytes[0], bytes[1], bytes[2]]) >> 8;
            #[expect(clippy::cast_precision_loss, reason = "24-bit PCM fits cleanly in f32 mantissa")]
            let val = raw as f32 / 8_388_608.0;
            val
        }
        SampleFormat::Pcm32 => {
            let raw = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            #[expect(clippy::cast_precision_loss, reason = "32-bit PCM to f32 visualiser scaling")]
            let val = raw as f32 / 2_147_483_648.0;
            val
        }
    }
}
