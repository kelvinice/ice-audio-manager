//! Music recognition engine: WASAPI Microphone / System Audio capture,
//! pure-Rust Shazam-compatible acoustic fingerprinting, and Shazam Discovery v5 lookup.

use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ice_core::device::{Flow, Role};
use ice_win::com::ComGuard;
use ice_win::endpoints::Endpoints;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::E_NOTIMPL;
use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, IAudioCaptureClient,
    IAudioClient, IMMDevice, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoTaskMemFree};
use windows::core::GUID;

use crate::http;
use crate::models::RecognizedTrack;
use crate::ytmusic::YtMusicClient;

const TARGET_SAMPLE_RATE: u32 = 16_000;
const FFT_SIZE: usize = 2048;
const FFT_OUTPUT_SIZE: usize = FFT_SIZE / 2 + 1;
const HOP_SIZE: usize = 128;
const RING_SLOTS: usize = 256;
const MAX_PEAKS: usize = 255;
const MAX_CAPTURE_SECS: f64 = 10.0;
const MIN_CAPTURE_SECS: f64 = 3.1;

const WAVE_FORMAT_PCM_TAG: u16 = 0x0001;
const WAVE_FORMAT_IEEE_FLOAT_TAG: u16 = 0x0003;
const WAVE_FORMAT_EXTENSIBLE_TAG: u16 = 0xFFFE;
const SUBTYPE_PCM: GUID = GUID::from_u128(0x0000_0001_0000_0010_8000_00aa_0038_9b71);
const SUBTYPE_IEEE_FLOAT: GUID = GUID::from_u128(0x0000_0003_0000_0010_8000_00aa_0038_9b71);

const SHAZAM_UAS: &[&str] = &[
    "Dalvik/2.1.0 (Linux; U; Android 11; Pixel 5 Build/RQ3A.210805.001.A1)",
    "Dalvik/2.1.0 (Linux; U; Android 10; SM-G973F Build/QP1A.190711.020)",
    "Dalvik/2.1.0 (Linux; U; Android 12; M2102J20SG Build/SKQ1.211006.001)",
];

const SHAZAM_TIMEZONES: &[&str] = &[
    "America/New_York",
    "America/Los_Angeles",
    "Europe/London",
    "Europe/Paris",
    "Asia/Tokyo",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureSource {
    SystemAudio,
    Microphone,
}

impl CaptureSource {
    pub fn from_str(s: &str) -> Self {
        if s.eq_ignore_ascii_case("mic") || s.eq_ignore_ascii_case("microphone") {
            Self::Microphone
        } else {
            Self::SystemAudio
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SystemAudio => "system",
            Self::Microphone => "mic",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognitionProgress {
    /// `"listening" | "matching" | "done" | "cancelled" | "error"`
    pub state: String,
    pub elapsed_ms: u64,
    pub level: f32,
}

fn emit_progress(app: &AppHandle, state: &str, elapsed_ms: u64, level: f32) {
    let _ = app.emit(
        "recognition://status/v1",
        RecognitionProgress {
            state: state.to_string(),
            elapsed_ms,
            level: level.clamp(0.0, 1.0),
        },
    );
}

pub fn recognize_from_audio(
    source: CaptureSource,
    cancel: &Arc<AtomicBool>,
    app: &AppHandle,
    ytm: &YtMusicClient,
) -> Result<RecognizedTrack, String> {
    cancel.store(false, Ordering::Release);
    let _com = ComGuard::init_mta().map_err(|e| format!("COM init failed: {e}"))?;
    let eps = Endpoints::new().map_err(|e| format!("Audio service error: {e}"))?;

    let device: IMMDevice = match source {
        CaptureSource::SystemAudio => eps
            .default_device(Flow::Render, Role::Multimedia)
            .map_err(|e| format!("No active playback output found: {e}"))?,
        CaptureSource::Microphone => eps
            .default_device(Flow::Capture, Role::Communications)
            .or_else(|_| eps.default_device(Flow::Capture, Role::Multimedia))
            .map_err(|e| format!("No microphone input device found: {e}"))?,
    };

    let capture = WasapiRawCapture::new(&device, source == CaptureSource::SystemAudio)
        .map_err(|e| format!("Could not open audio stream: {e}"))?;

    let mut resampler = LinearResampler::new(capture.sample_rate, TARGET_SAMPLE_RATE);
    let mut pcm_16k: Vec<i16> = Vec::with_capacity(TARGET_SAMPLE_RATE as usize * 11);

    let start = Instant::now();
    let mut next_emit = Instant::now();
    let mut tried_4s = false;
    let mut tried_7s = false;

    emit_progress(app, "listening", 0, 0.0);

    loop {
        if cancel.load(Ordering::Acquire) {
            emit_progress(app, "cancelled", start.elapsed().as_millis() as u64, 0.0);
            return Err("Cancelled".into());
        }

        let mono = capture
            .drain_mono()
            .map_err(|e| format!("Audio stream interrupted: {e}"))?;

        let mut rms = 0.0_f32;
        if !mono.is_empty() {
            let sum_sq: f32 = mono.iter().map(|s| s * s).sum();
            rms = ((sum_sq / mono.len() as f32).sqrt() * 4.5).clamp(0.0, 1.0);
            resampler.push(&mono, &mut pcm_16k);
        }

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;
        if Instant::now() >= next_emit {
            emit_progress(app, "listening", elapsed_ms, rms);
            next_emit = Instant::now() + Duration::from_millis(55);
        }

        let captured_secs = pcm_16k.len() as f64 / f64::from(TARGET_SAMPLE_RATE);
        if captured_secs >= 4.0 && !tried_4s {
            tried_4s = true;
            emit_progress(app, "matching", elapsed_ms, rms);
            if let Ok(Some(found)) = try_match_slice(&pcm_16k, source, ytm) {
                emit_progress(app, "done", start.elapsed().as_millis() as u64, 0.0);
                return Ok(found);
            }
        } else if captured_secs >= 7.0 && !tried_7s {
            tried_7s = true;
            emit_progress(app, "matching", elapsed_ms, rms);
            if let Ok(Some(found)) = try_match_slice(&pcm_16k, source, ytm) {
                emit_progress(app, "done", start.elapsed().as_millis() as u64, 0.0);
                return Ok(found);
            }
        }

        if elapsed.as_secs_f64() >= MAX_CAPTURE_SECS || captured_secs >= MAX_CAPTURE_SECS {
            break;
        }

        std::thread::sleep(Duration::from_millis(25));
    }

    if (pcm_16k.len() as f64 / f64::from(TARGET_SAMPLE_RATE)) < MIN_CAPTURE_SECS {
        emit_progress(app, "error", start.elapsed().as_millis() as u64, 0.0);
        return Err("Not enough audio captured. Play audio or speak near microphone and try again.".into());
    }

    emit_progress(app, "matching", start.elapsed().as_millis() as u64, 0.0);
    match try_match_slice(&pcm_16k, source, ytm)? {
        Some(found) => {
            emit_progress(app, "done", start.elapsed().as_millis() as u64, 0.0);
            Ok(found)
        }
        None => {
            emit_progress(app, "error", start.elapsed().as_millis() as u64, 0.0);
            Err("No matching song found. Try a clearer passage or switch between System Audio & Mic.".into())
        }
    }
}

fn try_match_slice(
    pcm_16k: &[i16],
    source: CaptureSource,
    ytm: &YtMusicClient,
) -> Result<Option<RecognizedTrack>, String> {
    let mut sig_gen = SignatureGenerator::new();
    sig_gen.feed_input(pcm_16k);
    let sig = sig_gen.next_signature()?;
    let sample_ms = (sig.num_samples as u64 * 1000) / u64::from(TARGET_SAMPLE_RATE);
    let uri = sig.encode_base64_uri();
    query_shazam(&uri, sample_ms, source, ytm)
}

fn query_shazam(
    signature_uri: &str,
    sample_ms: u64,
    source: CaptureSource,
    ytm: &YtMusicClient,
) -> Result<Option<RecognizedTrack>, String> {
    let ts_sec = now_ms() / 1000;
    let uuid1 = random_uuid_v4().to_ascii_uppercase();
    let uuid2 = random_uuid_v4();
    let idx = (now_ms() as usize) % SHAZAM_UAS.len();
    let ua = SHAZAM_UAS[idx];
    let tz = SHAZAM_TIMEZONES[(now_ms() as usize) % SHAZAM_TIMEZONES.len()];

    let body = serde_json::json!({
        "geolocation": {
            "altitude": 220.0,
            "latitude": 37.7749,
            "longitude": -122.4194,
        },
        "signature": {
            "samplems": sample_ms.max(3000),
            "timestamp": ts_sec,
            "uri": signature_uri,
        },
        "timestamp": ts_sec,
        "timezone": tz,
    })
    .to_string();

    let url = format!(
        "https://amp.shazam.com/discovery/v5/en/US/android/-/tag/{uuid1}/{uuid2}?sync=true&webv3=true&sampling=true&connected=&shazamapiversion=v3&sharehub=true&video=v3"
    );

    let (status, bytes) = http::post_json_with_ua(
        &url,
        &[("Content-Language", "en_US")],
        &body,
        Some(ua),
    )?;

    if status == 429 {
        return Err("Shazam rate limit reached; wait a moment and try again.".into());
    }
    if status == 404 {
        return Ok(None);
    }
    if !(200..=299).contains(&status) {
        return Err(format!("Recognition server returned HTTP {status}"));
    }

    let text = String::from_utf8_lossy(&bytes);
    parse_shazam_response(&text, source, ytm)
}

pub fn parse_shazam_response(
    json_text: &str,
    source: CaptureSource,
    ytm: &YtMusicClient,
) -> Result<Option<RecognizedTrack>, String> {
    let val: serde_json::Value =
        serde_json::from_str(json_text).map_err(|e| format!("Invalid recognition JSON: {e}"))?;
    let Some(track_obj) = val.get("track").filter(|v| v.is_object()) else {
        return Ok(None);
    };

    let title = track_obj
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Matched track missing title".to_string())?
        .to_string();

    let artist = track_obj
        .get("subtitle")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("Unknown Artist")
        .to_string();

    let id = track_obj
        .get("key")
        .and_then(|v| v.as_str())
        .or_else(|| val.get("tagid").and_then(|v| v.as_str()))
        .map(|s| format!("shz-{s}"))
        .unwrap_or_else(|| format!("shz-{}", now_ms()));

    let artwork_url = track_obj
        .pointer("/images/coverarthq")
        .and_then(|v| v.as_str())
        .or_else(|| track_obj.pointer("/images/coverart").and_then(|v| v.as_str()))
        .map(str::to_string);

    let genre = track_obj
        .pointer("/genres/primary")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let shazam_url = track_obj
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let mut album = None;
    let mut label = None;
    let mut release_date = None;
    let mut lyrics_snippet = None;

    if let Some(sections) = track_obj.get("sections").and_then(|v| v.as_array()) {
        for sec in sections {
            match sec.get("type").and_then(|v| v.as_str()) {
                Some("SONG") => {
                    if let Some(meta) = sec.get("metadata").and_then(|v| v.as_array()) {
                        for m in meta {
                            let k = m.get("title").and_then(|v| v.as_str()).unwrap_or("");
                            let txt = m.get("text").and_then(|v| v.as_str()).map(str::to_string);
                            if k.eq_ignore_ascii_case("Album") {
                                album = txt;
                            } else if k.eq_ignore_ascii_case("Label") {
                                label = txt;
                            } else if k.eq_ignore_ascii_case("Released") {
                                release_date = txt;
                            }
                        }
                    }
                }
                Some("LYRICS") => {
                    if let Some(lines) = sec.get("text").and_then(|v| v.as_array()) {
                        let joined: Vec<&str> = lines.iter().filter_map(|l| l.as_str()).collect();
                        if !joined.is_empty() {
                            lyrics_snippet = Some(joined.join("\n"));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Best-effort instant YouTube Music lookup so the user can hit Play immediately.
    let query = format!("{title} {artist}");
    let matched_track = ytm
        .search(&query, "songs")
        .tracks
        .into_iter()
        .next()
        .map(|mut t| {
            if t.artwork_url.is_none() {
                t.artwork_url = artwork_url.clone();
            }
            if t.album.is_empty()
                && let Some(ref alb) = album
            {
                t.album = alb.clone();
            }
            t
        });

    Ok(Some(RecognizedTrack {
        id,
        title,
        artist,
        album,
        artwork_url,
        genre,
        release_date,
        label,
        lyrics_snippet,
        shazam_url,
        matched_track,
        source: source.as_str().to_string(),
        recognized_at_ms: now_ms(),
    }))
}

// ============================================================================
// WASAPI Raw Audio Capture & Linear Resampler
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RawSampleFormat {
    Float32,
    Pcm16,
    Pcm24,
    Pcm32,
}

impl RawSampleFormat {
    fn bytes_per_sample(self) -> usize {
        match self {
            Self::Pcm16 => 2,
            Self::Pcm24 => 3,
            Self::Float32 | Self::Pcm32 => 4,
        }
    }
}

struct MixFormatGuard(*mut WAVEFORMATEX);

impl Drop for MixFormatGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: pointer was allocated by `IAudioClient::GetMixFormat`.
            unsafe { CoTaskMemFree(Some(self.0.cast())) };
        }
    }
}

struct PacketGuard<'a> {
    capture: &'a IAudioCaptureClient,
    frames: u32,
}

impl Drop for PacketGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: `self.frames` matches the preceding `GetBuffer` call.
        let _ = unsafe { self.capture.ReleaseBuffer(self.frames) };
    }
}

struct WasapiRawCapture {
    client: IAudioClient,
    capture: IAudioCaptureClient,
    pub sample_rate: u32,
    channels: usize,
    block_align: usize,
    format: RawSampleFormat,
}

impl WasapiRawCapture {
    fn new(dev: &IMMDevice, loopback: bool) -> windows::core::Result<Self> {
        // SAFETY: activating documented `IAudioClient` on a valid endpoint.
        let client: IAudioClient = unsafe { dev.Activate(CLSCTX_ALL, None)? };
        // SAFETY: `MixFormatGuard` frees the COM-allocated `WAVEFORMATEX`.
        let mix_ptr = unsafe { client.GetMixFormat()? };
        let _mix_guard = MixFormatGuard(mix_ptr);
        if mix_ptr.is_null() {
            return Err(E_NOTIMPL.into());
        }

        // SAFETY: `mix_ptr` is valid for reading `WAVEFORMATEX` / `WAVEFORMATEXTENSIBLE`.
        let (sample_rate, channels, block_align, format) = unsafe { parse_mix_format(mix_ptr)? };
        let stream_flags = if loopback { AUDCLNT_STREAMFLAGS_LOOPBACK } else { 0 };

        // SAFETY: shared-mode capture/loopback using the endpoint's own mix format.
        unsafe {
            client.Initialize(AUDCLNT_SHAREMODE_SHARED, stream_flags, 1_500_000, 0, mix_ptr, None)?;
        }
        // SAFETY: `IAudioCaptureClient` is provided by an initialized capture client.
        let capture: IAudioCaptureClient = unsafe { client.GetService()? };
        // SAFETY: start capture stream.
        unsafe { client.Start()? };

        Ok(Self {
            client,
            capture,
            sample_rate,
            channels,
            block_align,
            format,
        })
    }

    fn drain_mono(&self) -> windows::core::Result<Vec<f32>> {
        let mut out = Vec::new();
        let silent_flag = u32::try_from(AUDCLNT_BUFFERFLAGS_SILENT.0).unwrap_or(0);

        for _ in 0..64 {
            // SAFETY: non-blocking check on active `IAudioCaptureClient`.
            let packet_frames = unsafe { self.capture.GetNextPacketSize()? };
            if packet_frames == 0 {
                break;
            }
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut num_frames: u32 = 0;
            let mut flags: u32 = 0;
            // SAFETY: valid out pointers; `PacketGuard` always calls `ReleaseBuffer`.
            unsafe {
                self.capture
                    .GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)?;
            }
            let _guard = PacketGuard { capture: &self.capture, frames: num_frames };
            if num_frames == 0 {
                continue;
            }
            let frame_count = num_frames as usize;
            if (flags & silent_flag) != 0 || data_ptr.is_null() {
                out.resize(out.len() + frame_count, 0.0);
                continue;
            }
            let Some(byte_len) = frame_count.checked_mul(self.block_align) else {
                continue;
            };
            // SAFETY: WASAPI guarantees `data_ptr` is readable for `byte_len` bytes until `ReleaseBuffer`.
            let bytes = unsafe { std::slice::from_raw_parts(data_ptr, byte_len) };
            let bps = self.format.bytes_per_sample();
            let ch_count = self.channels.max(1);
            out.reserve(frame_count);
            for f in 0..frame_count {
                let base = f * self.block_align;
                let mut acc = 0.0_f32;
                for ch in 0..ch_count {
                    let off = base + ch * bps;
                    acc += decode_sample(self.format, &bytes[off..off + bps]);
                }
                out.push(acc / ch_count as f32);
            }
        }
        Ok(out)
    }
}

impl Drop for WasapiRawCapture {
    fn drop(&mut self) {
        // SAFETY: stop WASAPI stream before COM interface drops.
        let _ = unsafe { self.client.Stop() };
    }
}

/// # Safety
/// `ptr` must point to a valid `WAVEFORMATEX` allocated by Windows `GetMixFormat`.
unsafe fn parse_mix_format(ptr: *const WAVEFORMATEX) -> windows::core::Result<(u32, usize, usize, RawSampleFormat)> {
    // SAFETY: caller guarantees valid pointer.
    let wfx = unsafe { ptr.read_unaligned() };
    let sample_rate = wfx.nSamplesPerSec;
    let channels = usize::from(wfx.nChannels);
    let block_align = usize::from(wfx.nBlockAlign);
    let bits = wfx.wBitsPerSample;
    let tag = wfx.wFormatTag;

    let format = match tag {
        WAVE_FORMAT_IEEE_FLOAT_TAG if bits == 32 => RawSampleFormat::Float32,
        WAVE_FORMAT_PCM_TAG => pcm_format(bits)?,
        WAVE_FORMAT_EXTENSIBLE_TAG if wfx.cbSize >= 22 => {
            let ext_ptr = ptr.cast::<WAVEFORMATEXTENSIBLE>();
            // SAFETY: `cbSize >= 22` guarantees a full `WAVEFORMATEXTENSIBLE`.
            let subformat = unsafe { std::ptr::addr_of!((*ext_ptr).SubFormat).read_unaligned() };
            if subformat == SUBTYPE_IEEE_FLOAT && bits == 32 {
                RawSampleFormat::Float32
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

fn pcm_format(bits: u16) -> windows::core::Result<RawSampleFormat> {
    match bits {
        16 => Ok(RawSampleFormat::Pcm16),
        24 => Ok(RawSampleFormat::Pcm24),
        32 => Ok(RawSampleFormat::Pcm32),
        _ => Err(E_NOTIMPL.into()),
    }
}

#[inline]
fn decode_sample(format: RawSampleFormat, bytes: &[u8]) -> f32 {
    match format {
        RawSampleFormat::Float32 => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        RawSampleFormat::Pcm16 => f32::from(i16::from_le_bytes([bytes[0], bytes[1]])) / 32_768.0,
        RawSampleFormat::Pcm24 => {
            let raw = i32::from_le_bytes([0, bytes[0], bytes[1], bytes[2]]) >> 8;
            raw as f32 / 8_388_608.0
        }
        RawSampleFormat::Pcm32 => {
            let raw = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            raw as f32 / 2_147_483_648.0
        }
    }
}

struct LinearResampler {
    step: f64,
    pos: f64,
    prev: f32,
    has_prev: bool,
    lp_state: f32,
    lp_alpha: f32,
}

impl LinearResampler {
    fn new(in_rate: u32, out_rate: u32) -> Self {
        let step = f64::from(in_rate) / f64::from(out_rate.max(1));
        // Gentle single-pole anti-aliasing filter when downsampling from 44.1/48k to 16k
        let lp_alpha = if in_rate > out_rate {
            (0.45 * (out_rate as f32 / in_rate as f32) * std::f32::consts::PI).clamp(0.1, 1.0)
        } else {
            1.0
        };
        Self {
            step,
            pos: 0.0,
            prev: 0.0,
            has_prev: false,
            lp_state: 0.0,
            lp_alpha,
        }
    }

    fn push(&mut self, input: &[f32], out: &mut Vec<i16>) {
        if input.is_empty() {
            return;
        }
        let mut filtered = Vec::with_capacity(input.len());
        for &s in input {
            self.lp_state += self.lp_alpha * (s - self.lp_state);
            filtered.push(self.lp_state);
        }

        if !self.has_prev {
            self.prev = filtered[0];
            self.has_prev = true;
        }

        while self.pos < filtered.len() as f64 {
            let i0 = self.pos.floor() as isize;
            let frac = (self.pos - i0 as f64) as f32;
            let s0 = if i0 < 0 { self.prev } else { filtered[i0 as usize] };
            let i1 = (i0 + 1) as usize;
            let s1 = if i1 < filtered.len() {
                filtered[i1]
            } else {
                break;
            };
            let val = s0 + (s1 - s0) * frac;
            let pcm = (val * 32_767.0).round().clamp(-32_768.0, 32_767.0) as i16;
            out.push(pcm);
            self.pos += self.step;
        }

        self.prev = *filtered.last().unwrap_or(&0.0);
        self.pos -= filtered.len() as f64;
    }
}

// ============================================================================
// Shazam-Compatible Acoustic Fingerprint Engine
// ============================================================================

#[derive(Debug, Clone, Copy)]
struct FrequencyPeak {
    fft_pass_number: u32,
    peak_magnitude: u32,
    corrected_peak_frequency_bin: u32,
}

#[derive(Debug, Clone)]
pub struct ShazamSignature {
    pub sample_rate: u32,
    pub num_samples: u32,
    peaks_by_band: BTreeMap<u32, Vec<FrequencyPeak>>,
}

impl ShazamSignature {
    fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            num_samples: 0,
            peaks_by_band: BTreeMap::new(),
        }
    }

    fn total_peaks(&self) -> usize {
        self.peaks_by_band.values().map(Vec::len).sum()
    }

    pub fn encode_base64_uri(&self) -> String {
        let raw = self.encode_binary();
        format!("data:audio/vnd.shazam.sig;base64,{}", base64_encode(&raw))
    }

    pub fn encode_binary(&self) -> Vec<u8> {
        let mut contents = Vec::new();
        for (&band, peaks) in &self.peaks_by_band {
            let mut peak_buf = Vec::new();
            let mut last_pass: u32 = 0;

            for peak in peaks {
                let diff = peak.fft_pass_number.saturating_sub(last_pass);
                if diff >= 255 {
                    peak_buf.push(0xff);
                    peak_buf.extend_from_slice(&peak.fft_pass_number.to_le_bytes());
                    last_pass = peak.fft_pass_number;
                }
                let step = peak.fft_pass_number.saturating_sub(last_pass) as u8;
                peak_buf.push(step);
                peak_buf.extend_from_slice(&(peak.peak_magnitude as u16).to_le_bytes());
                peak_buf.extend_from_slice(&(peak.corrected_peak_frequency_bin as u16).to_le_bytes());
                last_pass = peak.fft_pass_number;
            }

            contents.extend_from_slice(&(0x6003_0040_u32 + band).to_le_bytes());
            contents.extend_from_slice(&(peak_buf.len() as u32).to_le_bytes());
            contents.extend_from_slice(&peak_buf);
            let pad = (4 - (peak_buf.len() % 4)) % 4;
            contents.resize(contents.len() + pad, 0);
        }

        let size_minus_header = (contents.len() + 8) as u32;
        let samples_plus =
            (f64::from(self.num_samples) + f64::from(self.sample_rate) * 0.24) as u32;
        let shifted_sample_rate_id: u32 = 3 << 27;
        let fixed_value: u32 = (15 << 19) + 0x40000;

        let mut buf = Vec::with_capacity(56 + contents.len());
        buf.extend_from_slice(&0xcafe_2580_u32.to_le_bytes());
        buf.extend_from_slice(&0_u32.to_le_bytes()); // crc32 placeholder
        buf.extend_from_slice(&size_minus_header.to_le_bytes());
        buf.extend_from_slice(&0x9411_9c00_u32.to_le_bytes());
        buf.extend_from_slice(&[0_u8; 12]); // void1[3]
        buf.extend_from_slice(&shifted_sample_rate_id.to_le_bytes());
        buf.extend_from_slice(&[0_u8; 8]); // void2[2]
        buf.extend_from_slice(&samples_plus.to_le_bytes());
        buf.extend_from_slice(&fixed_value.to_le_bytes());

        buf.extend_from_slice(&0x4000_0000_u32.to_le_bytes());
        buf.extend_from_slice(&size_minus_header.to_le_bytes());
        buf.extend_from_slice(&contents);

        let crc = crc32_ieee(&buf[8..]);
        buf[4..8].copy_from_slice(&crc.to_le_bytes());
        buf
    }
}

pub struct SignatureGenerator {
    pending: Vec<i16>,
    processed: usize,
    next_sig: ShazamSignature,
    ring_samples: [i16; FFT_SIZE],
    ring_pos: usize,
    fft_outputs: Vec<[f64; FFT_OUTPUT_SIZE]>,
    fft_pos: usize,
    spread_outputs: Vec<[f64; FFT_OUTPUT_SIZE]>,
    spread_pos: usize,
    spread_written: usize,
    hann: [f64; FFT_SIZE],
}

impl SignatureGenerator {
    pub fn new() -> Self {
        let mut hann = [0.0_f64; FFT_SIZE];
        for (i, w) in hann.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (2.0 * PI * ((i + 1) as f64) / ((FFT_SIZE + 1) as f64)).cos());
        }
        Self {
            pending: Vec::new(),
            processed: 0,
            next_sig: ShazamSignature::new(TARGET_SAMPLE_RATE),
            ring_samples: [0; FFT_SIZE],
            ring_pos: 0,
            fft_outputs: vec![[0.0; FFT_OUTPUT_SIZE]; RING_SLOTS],
            fft_pos: 0,
            spread_outputs: vec![[0.0; FFT_OUTPUT_SIZE]; RING_SLOTS],
            spread_pos: 0,
            spread_written: 0,
            hann,
        }
    }

    pub fn feed_input(&mut self, samples: &[i16]) {
        self.pending.extend_from_slice(samples);
    }

    pub fn next_signature(&mut self) -> Result<ShazamSignature, String> {
        if self.pending.len().saturating_sub(self.processed) < HOP_SIZE {
            return Err("Not enough audio input to build signature".into());
        }

        while self.pending.len().saturating_sub(self.processed) >= HOP_SIZE
            && (f64::from(self.next_sig.num_samples) / f64::from(self.next_sig.sample_rate)
                < MAX_CAPTURE_SECS
                || self.next_sig.total_peaks() < MAX_PEAKS)
        {
            let chunk = &self.pending[self.processed..self.processed + HOP_SIZE];
            let mut copy = [0_i16; HOP_SIZE];
            copy.copy_from_slice(chunk);
            self.process_chunk(&copy);
            self.processed += HOP_SIZE;
        }

        Ok(self.next_sig.clone())
    }

    fn process_chunk(&mut self, chunk: &[i16; HOP_SIZE]) {
        self.next_sig.num_samples += HOP_SIZE as u32;
        self.do_fft(chunk);
        self.do_peak_spreading();
        if self.spread_written >= 47 {
            self.do_peak_recognition();
        }
    }

    fn do_fft(&mut self, chunk: &[i16; HOP_SIZE]) {
        self.ring_samples[self.ring_pos..self.ring_pos + HOP_SIZE].copy_from_slice(chunk);
        self.ring_pos = (self.ring_pos + HOP_SIZE) % FFT_SIZE;

        let mut re = [0.0_f64; FFT_SIZE];
        let mut im = [0.0_f64; FFT_SIZE];

        let tail = FFT_SIZE - self.ring_pos;
        for i in 0..tail {
            re[i] = f64::from(self.ring_samples[self.ring_pos + i]) * self.hann[i];
        }
        for i in 0..self.ring_pos {
            re[tail + i] = f64::from(self.ring_samples[i]) * self.hann[tail + i];
        }

        fft_radix2_inplace(&mut re, &mut im);

        let scale = 1.0 / f64::from(1_u32 << 17);
        let out = &mut self.fft_outputs[self.fft_pos];
        for i in 0..FFT_OUTPUT_SIZE {
            let pwr = (re[i] * re[i] + im[i] * im[i]) * scale;
            out[i] = pwr.max(1e-10);
        }
        self.fft_pos = (self.fft_pos + 1) % RING_SLOTS;
    }

    fn do_peak_spreading(&mut self) {
        let last_idx = (self.fft_pos + RING_SLOTS - 1) % RING_SLOTS;
        let mut spread = self.fft_outputs[last_idx];

        for pos in 0..FFT_OUTPUT_SIZE {
            if pos < FFT_OUTPUT_SIZE - 2 {
                spread[pos] = spread[pos].max(spread[pos + 1]).max(spread[pos + 2]);
            }
            let mut max_val = spread[pos];
            for lag in [-1_isize, -3, -6] {
                let idx = (self.spread_pos as isize + RING_SLOTS as isize + lag) as usize % RING_SLOTS;
                max_val = max_val.max(self.spread_outputs[idx][pos]);
                self.spread_outputs[idx][pos] = max_val;
            }
        }

        self.spread_outputs[self.spread_pos] = spread;
        self.spread_pos = (self.spread_pos + 1) % RING_SLOTS;
        self.spread_written += 1;
    }

    fn do_peak_recognition(&mut self) {
        let idx_m46 = (self.fft_pos + RING_SLOTS - 46) % RING_SLOTS;
        let idx_m49 = (self.spread_pos + RING_SLOTS - 49) % RING_SLOTS;

        const NEIGHBOR_OFFSETS: [isize; 8] = [-10, -7, -4, -3, 1, 2, 5, 8];
        const TIME_OFFSETS: [isize; 14] = [-53, -45, 165, 172, 179, 186, 193, 200, 214, 221, 228, 235, 242, 249];

        for bin in 10..(FFT_OUTPUT_SIZE - 8) {
            let center = self.fft_outputs[idx_m46][bin];
            if center < (1.0 / 64.0) || center < self.spread_outputs[idx_m49][bin] {
                continue;
            }

            let mut max_neighbor = 0.0_f64;
            for off in NEIGHBOR_OFFSETS {
                let b = (bin as isize + off) as usize;
                max_neighbor = max_neighbor.max(self.spread_outputs[idx_m49][b]);
            }
            if center <= max_neighbor {
                continue;
            }

            let mut max_other = max_neighbor;
            for t_off in TIME_OFFSETS {
                let idx = ((self.spread_pos as isize + t_off).rem_euclid(RING_SLOTS as isize)) as usize;
                max_other = max_other.max(self.spread_outputs[idx][bin - 1]);
            }
            if center <= max_other {
                continue;
            }

            let fft_number = (self.spread_written - 46) as u32;
            let mag_center = center.max(1.0 / 64.0).ln() * 1477.3 + 6144.0;
            let mag_before = self.fft_outputs[idx_m46][bin - 1].max(1.0 / 64.0).ln() * 1477.3 + 6144.0;
            let mag_after = self.fft_outputs[idx_m46][bin + 1].max(1.0 / 64.0).ln() * 1477.3 + 6144.0;

            let v1 = mag_center * 2.0 - mag_before - mag_after;
            let v2 = if v1.abs() > 1e-9 {
                (mag_after - mag_before) * 32.0 / v1
            } else {
                0.0
            };
            let corrected_bin = (bin as f64) * 64.0 + v2;
            let freq_hz = corrected_bin * (16_000.0 / 2.0 / 1024.0 / 64.0);

            let band = if freq_hz < 250.0 {
                continue;
            } else if freq_hz < 520.0 {
                0
            } else if freq_hz < 1450.0 {
                1
            } else if freq_hz < 3500.0 {
                2
            } else if freq_hz <= 5500.0 {
                3
            } else {
                continue;
            };

            self.next_sig
                .peaks_by_band
                .entry(band)
                .or_default()
                .push(FrequencyPeak {
                    fft_pass_number: fft_number,
                    peak_magnitude: mag_center.max(0.0) as u32,
                    corrected_peak_frequency_bin: corrected_bin.max(0.0) as u32,
                });
        }
    }
}

fn fft_radix2_inplace(re: &mut [f64; FFT_SIZE], im: &mut [f64; FFT_SIZE]) {
    let mut j = 0usize;
    for i in 1..FFT_SIZE {
        let mut bit = FFT_SIZE >> 1;
        while (j & bit) != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut len = 2usize;
    while len <= FFT_SIZE {
        let half = len >> 1;
        let ang = -2.0 * PI / (len as f64);
        let wlen_r = ang.cos();
        let wlen_i = ang.sin();
        let mut i = 0usize;
        while i < FFT_SIZE {
            let mut wr = 1.0_f64;
            let mut wi = 0.0_f64;
            for k in 0..half {
                let u_r = re[i + k];
                let u_i = im[i + k];
                let t_r = re[i + k + half] * wr - im[i + k + half] * wi;
                let t_i = re[i + k + half] * wi + im[i + k + half] * wr;
                re[i + k] = u_r + t_r;
                im[i + k] = u_i + t_i;
                re[i + k + half] = u_r - t_r;
                im[i + k + half] = u_i - t_i;
                let next_wr = wr * wlen_r - wi * wlen_i;
                wi = wr * wlen_i + wi * wlen_r;
                wr = next_wr;
            }
            i += len;
        }
        len <<= 1;
    }
}

fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = if (crc & 1) != 0 { 0xedb8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[(b2 & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn random_uuid_v4() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let a = RandomState::new().hash_one(nanos);
    let b = RandomState::new().hash_one((nanos.wrapping_add(1), &a as *const u64 as usize));
    let mut bytes = [0u8; 16];
    bytes[0..8].copy_from_slice(&a.to_le_bytes());
    bytes[8..16].copy_from_slice(&b.to_le_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_and_base64_match_standard_vectors() {
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(b"f"), "Zg==");
    }

    #[test]
    fn fft_detects_exact_frequency_bin_for_1khz_tone() {
        let mut re = [0.0_f64; FFT_SIZE];
        let mut im = [0.0_f64; FFT_SIZE];
        // Bin 128 at 16 kHz / 2048 FFT = 1000.0 Hz exact integer cycles
        for (i, sample) in re.iter_mut().enumerate() {
            *sample = (2.0 * PI * 128.0 * (i as f64) / (FFT_SIZE as f64)).sin();
        }
        fft_radix2_inplace(&mut re, &mut im);
        let (peak_bin, _) = (0..FFT_OUTPUT_SIZE)
            .map(|b| (b, re[b] * re[b] + im[b] * im[b]))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        assert_eq!(peak_bin, 128);
    }

    #[test]
    fn synthetic_chord_produces_valid_shazam_signature_header_and_crc() {
        let total = 16_000 * 4;
        let mut samples = Vec::with_capacity(total);
        for i in 0..total {
            let t = i as f64 / 16_000.0;
            // Pulsed multi-band chord across 400 Hz, 900 Hz, 2100 Hz so temporal/spectral peaks stand out
            let env = (t * 6.0).sin().abs();
            let v = ((2.0 * PI * 410.0 * t).sin()
                + (2.0 * PI * 920.0 * t).sin()
                + (2.0 * PI * 2150.0 * t).sin())
                * env
                * 8_000.0;
            samples.push(v as i16);
        }
        let mut sig_gen = SignatureGenerator::new();
        sig_gen.feed_input(&samples);
        let sig = sig_gen.next_signature().unwrap();
        assert!(sig.total_peaks() > 0);

        let bin = sig.encode_binary();
        assert!(bin.len() >= 56);
        assert_eq!(u32::from_le_bytes(bin[0..4].try_into().unwrap()), 0xcafe_2580);
        let stored_crc = u32::from_le_bytes(bin[4..8].try_into().unwrap());
        assert_eq!(stored_crc, crc32_ieee(&bin[8..]));
        assert_eq!(u32::from_le_bytes(bin[12..16].try_into().unwrap()), 0x9411_9c00);

        let uri = sig.encode_base64_uri();
        assert!(uri.starts_with("data:audio/vnd.shazam.sig;base64,"));
    }
}
