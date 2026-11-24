# D-26 · Gemini's vote

*Written sealed on 2026-11-24 from the D-26 entry and my own code inspections (`crates/ice-win/src/meter.rs`,
`crates/ice-core/src/meter.rs`, `app/ui/src/lib/notch/beat.ts`, `app/ui/src/lib/notch/visualizer.ts`,
`app/ui/src/notch/activities/VisualizerBars.svelte`).*

## Scores

### Part A · Signal and frequency source (*pick one*)

| Option | Score | One-line reason |
|---|---|---|
| **A0** · Keep `IAudioMeterInformation` peaks + today's `BeatShaper` | **−1** | Only sees two scalar `[L, R]` peaks (`meter.rs:48–59`); all 14 bars jump on the same frame and drop into a flat `0.1–0.2` line whenever melodic passages lack sharp `+1.8 dB/frame` onsets (`beat.ts:129, 145`). |
| **A1** · `IAudioMeterInformation` peaks + multi-envelope travelling-wave shaper | **+1** | Great automatic fallback when loopback is unavailable (and we should ship its wave shaper inside `beat.ts` either way), but scalar L/R peaks still cannot tell a bass kick from a vocal note or hi-hat. |
| **A2** · WASAPI loopback capture + in-tree 14-band log-spaced IIR biquad filter bank (with `A1` fallback) | **+2** | Real sub-bass, kick, vocal, and treble separation with zero external DSP crates, zero per-tick heap allocations, zero windowing latency (`~112k` float ops/tick at 30 Hz ≈ `< 0.03 ms` CPU), and an automatic `IAudioMeterInformation` + `A1` fallback. |
| **A3** · WASAPI loopback capture + windowed `1024`/`2048`-point FFT (with `A1` fallback) | **+1** | Also delivers real spectrum bands and I'm happy with it if Claude prefers an in-tree radix-2 FFT, though a `2048`-sample window adds `~43 ms` of ring-buffer smoothing latency on percussive transients compared to time-domain biquads. |

### Part B · Per-band shaping, physics, and layout (*pick many*; `B4a` vs `B4b` pick higher)

| Option | Score | One-line reason |
|---|---|---|
| **B1** · Per-band adaptive range (AGC) + pink-noise tilt (`+3.5 to +4.5 dB/octave`) | **+2** | Essential so treble and vocal bars swing across the full `0.10–1.0` height instead of sitting flat at the bottom while bass pegs the top. |
| **B2** · Frequency-graded `(k, c, m)` spring physics across the bars | **+2** | Gives bass bars a weighty, punchy rebound and treble bars a fast, shimmering snap instead of all 14 springs moving at identical speed (`visualizer.ts:217`). |
| **B3** · Lateral neighbor coupling (`~15–20 %` bleed with 1-tick lag) + melodic motion floor | **+2** | Turns isolated single-band hits into a connected, living wave silhouette and prevents flat dead zones during solo vocal or acoustic passages. |
| **B4a** · Symmetric centre-bass (`[treble, mid, bass, mid, treble]`) on 5-bar badge + `bass → treble` L→R on 14-bar wide scrubber | **+2** | The 5-bar pill (`18 px` wide) reads best as a symmetric Dynamic-Island badge anchored by a strong centre kick, while the 14-bar wide scrubber (`TrackActivity.svelte`) reads naturally left-to-right alongside track progress. |
| **B4b** · Left-to-right `bass → treble` everywhere (both 5-bar and 14-bar) | **0** | Works well on the 14-bar wide waveform, but on the tiny 5-bar `12–16 px` badge it makes the left edge permanently tall (bass-heavy) and the right edge lopsided. |

## Strongest argument against my top choice (`A2` + `B1/B2/B3/B4a`)

Replacing or augmenting `IAudioMeterInformation` (`crates/ice-win/src/meter.rs:25–60`, which is just two trivial COM getter calls, `GetPeakValue` and `GetChannelsPeakValues`) with a WASAPI `IAudioClient` + `IAudioCaptureClient` shared-mode loopback stream (`AUDCLNT_STREAMFLAGS_LOOPBACK`) introduces real Windows audio stream lifecycle edge cases:
1. **Silence behavior on WASAPI loopback:** When no application is writing audio to the render endpoint, WASAPI loopback does not always emit periodic packets — `GetNextPacketSize` can return `0` indefinitely until audio resumes. If the `ice-meter` thread blocks waiting for loopback packets, it could fail to notice `sinks.is_empty()` (`crates/ice-win/src/meter.rs:125`) or fail to emit the single transition-to-silence `MeterFrame` (`crates/ice-core/src/meter.rs:165`).
2. **Sample-rate & channel-mix variation:** Render endpoints run at `44.1 kHz`, `48 kHz`, `96 kHz`, or `192 kHz` and in 2.0 stereo, 5.1, or 7.1 float32 (or occasionally PCM16 extensible formats); hardcoding biquad coefficients for `48 kHz` stereo would shift filter center frequencies on `96 kHz` DACs or fail on multi-channel formats unless coefficients are computed from `WAVEFORMATEX.nSamplesPerSec` on bind.
3. **Slightly higher resource footprint while visible:** Although `14` biquads cost `< 0.03 ms` of math per `33 ms` tick, opening an `IAudioClient` loopback buffer maps a shared audio ring buffer into the process while the visualizer is visible.

## New option or amendment

**Amendment `A2-hybrid-packet-drain` (Non-blocking 33 ms packet drain + sample-rate-aware biquads + `A1` wave shaper when `channels.length <= 2`):**
1. **Keep the exact `33 ms` non-blocking sleep loop in `MeterHub::run` (`crates/ice-win/src/meter.rs:122–156`):** Do **not** use a blocking event handle on `IAudioClient`. On each `METER_INTERVAL` (`33 ms`) tick, drain all queued packets from `IAudioCaptureClient::GetNextPacketSize` / `GetBuffer` (treating `AUDCLNT_BUFFERFLAGS_SILENT` or `0` available packets after `> 66 ms` as silence), feed the front L/R channels through the 14 biquad filters (computing RBJ band-pass coefficients once from `mix_format.nSamplesPerSec` when activating the endpoint), and keep `MeterFrame { peak, channels: vec![b0..b13] }` so the IPC type `MeterFrame` doesn't even need a breaking schema change (`channels.len() == 14` signals spectrum bands; `channels.len() <= 8` signals raw channel peaks).
2. **Upgrade `BeatShaper` (`beat.ts`) with `B1 + B2 + B3 + B4a` and `A1`'s multi-envelope wave fallback:**
   - When `frame.channels.length === 14` (from `A2`/`A3` Rust spectrum bands), `BeatShaper` applies per-band adaptive dB floor/ceiling (`B1`) + `18 %` 1-tick lateral neighbor coupling (`B3`) so every band has punchy vertical travel without flat gaps.
   - When `frame.channels.length <= 8` (fallback to `IAudioMeterInformation` or `--fake-engine`), `BeatShaper` uses `A1`'s 3-timescale travelling wave (fast transient + medium body + slow melodic swell with a 3-tick delay line across bars) so even without loopback capture, sustained melodies ripple smoothly instead of dropping flat when `onset < 0.3`.

## Evidence I checked myself

- **`app/ui/src/lib/notch/beat.ts:25–39, 68–84, 128–150`:**
  - Checked why today's `BeatShaper` can still look flat during non-percussive or steady-loud sections:
    1. In `Follower.update` (`lines 76–82`), `this.floor` tracks `tracked` with `FLOOR_TAU_S = 1.2 s`, while `this.onset = clamp01((db - this.prevDb) / 6)`. During a sustained chorus, synth pad, or smooth vocal line where frame-to-frame dB change is `< 1.8 dB` (`BEAT_ONSET = 0.3 × 6 dB`), `onset < BEAT_ONSET` (`line 129`), so `this.beat` stops incrementing and `flick` (`line 147`) drops near `0`.
    2. At the same time, within `~1.5 s` of a steady loud chorus, `this.floor` rises up to meet `db`, driving `this.punch = (db - this.floor) / (this.ceil - this.floor)` toward `0` (`line 80`). Once `punch` and `rise` both decay, `out[i]` collapses to `body = presence * (0.1 + 0.1 * (1 - edge))` (`line 145`) — meaning **all 14 bars sit flat at `0.10–0.20` (`10–20%` height) during a loud sustained note or smooth vocal chorus!**
- **`app/ui/src/lib/notch/visualizer.ts:23–28, 183–207, 213–218`:**
  - In `VisualizerBank.constructor` (`line 217`), all 14 `Spring1D` bars are initialized with the exact same `deriveVisualizerSpringParams(preset, 'decay')` and updated with identical attack (`4.5×` stiffness, `0.35×` mass) and decay (`2.4×` stiffness) multipliers (`lines 23–28`). Giving low-band bars higher mass/lower damping (`B2`) and high-band bars lighter mass/faster snap is a ~15-line pure function change in `deriveVisualizerSpringParams(preset, phase, barIndex, barCount)` that immediately makes the bars feel alive and organic.
- **`app/ui/src/notch/activities/VisualizerBars.svelte:35, 57–77, 108–132`:**
  - The 5-bar `compact` (`12 px` tall) and `expanded` (`16 px` tall) badges are only `18 px` wide (`5 × 2 px` bars + `4 × 2 px` gaps), while the `wide` waveform scrubber (`TrackActivity.svelte:80`) is `14` bars (`24 px` tall, `67 px` wide). On the `18 px`-wide 5-bar badge, a symmetric centre-anchored mapping (`B4a`: `[high, mid, bass, mid, high]`) keeps the silhouette balanced like an audio wave icon, whereas on the `67 px`-wide 14-bar scrubber, left-to-right `bass → treble` (`B4a`) lets the user watch bass kicks on the left, vocals in the middle, and hi-hats on the right.
- **`crates/ice-win/src/meter.rs:25–60, 112–157` & `crates/ice-core/src/meter.rs:32–62`:**
  - `MeterHub::run` (`crates/ice-win/src/meter.rs:112–157`) already manages `ComGuard::init_mta()`, endpoint rebinding on `generation` change (`MeterHub::rebind`), automatic `RETRY_AFTER = 1 s` recovery on `AUDCLNT_E_DEVICE_INVALIDATED`, and deterministic exit when `st.sinks.is_empty()`.
  - `MeterFrame` (`crates/ice-core/src/meter.rs:32–38`) already carries `pub peak: f32` and `pub channels: Vec<f32>`. Sending the 14 log-spaced band levels in `channels` (when `MAX_CHANNELS` is raised from `8` to `14`, or via loopback band extraction) works across the existing `audio://meter/v1` Tauri channel without adding a second IPC event stream.

## Position

### 1. Why the bars feel flat today, and why we need both real frequency bands (`A2`) and better bar physics (`B1 + B2 + B3 + B4a`)
When I inspected `app/ui/src/lib/notch/beat.ts:76-82, 145-148`, the root cause of the bars looking "too flat" jumped right out:
1. **No pitch separation (`IAudioMeterInformation`):** Windows' peak meter only gives one scalar peak per stereo channel. Every bar reacts to the same sum of audio, so you never see bass pumping on one set of bars while vocals and hi-hats dance on another.
2. **Sustained choruses collapse to `10–20%` height in `beat.ts`:** Because `Follower` adapts `this.floor` with a `1.2 s` time constant (`FLOOR_TAU_S = 1.2`) and `this.beat` only advances on sharp `+1.8 dB/frame` jumps (`BEAT_ONSET = 0.3`), any smooth vocal melody, guitar solo, or sustained loud chorus causes `punch` and `onset` to drop near zero within `~1.5 s`, leaving all 14 bars sitting nearly motionless at `body = 0.10–0.20`!

### 2. Why `A2` (WASAPI loopback + in-tree 14-band IIR biquads) beats `A0`, `A1`, and `A3`
- **Real bass, mid, and treble separation (`A2` vs `A0`/`A1`):** With shared-mode WASAPI loopback (`AUDCLNT_STREAMFLAGS_LOOPBACK`) on the `ice-meter` thread, a bass drum hits bands `0–2` (`45–110 Hz`), snare and vocals swell across bands `4–9` (`250 Hz – 2.5 kHz`), and hi-hats/cymbals shimmer on bands `10–13` (`4–14 kHz`). The bars move independently because different instruments actually occupy different bars.
- **Why in-tree biquads (`A2`, `+2`) slightly edge out a windowed FFT (`A3`, `+1`):**
  - 14 2nd-order RBJ biquad band-pass filters in Rust take ~70 lines of pure math, **zero external crates**, **zero heap allocations per tick**, and have **zero windowing latency** on drum transients (whereas a `2048`-sample FFT averages over a `42.6 ms` window).
  - At `48 kHz` (`~1,600` frames per `33 ms` tick), 14 biquads perform `~112,000` float multiply-adds per tick (`< 0.03 ms` of CPU every `33 ms`, well under `< 0.1%` CPU), and stop completely (`0%` CPU) the instant the last `MeterSubscription` drops (`crates/ice-win/src/meter.rs:125`).
  - That said, `A3` (in-tree radix-2 FFT) is also `+1` if Claude prefers writing the Rust side as a `1024`/`2048`-point FFT.

### 3. The 4 visual ingredients (`B1 + B2 + B3 + B4a`) that make the bars genuinely fun to watch
Even with real frequency bands from Rust, raw spectrum magnitudes look flat on treble unless shaped properly in TypeScript (`beat.ts` / `visualizer.ts`):
1. **Per-band AGC + pink-noise tilt (`B1`, `+2`):** Apply a `+4 dB/octave` tilt plus per-band running floor/ceiling normalization (with a higher minimum melodic floor so sustained vocals hover lively around `0.35–0.65` instead of collapsing to `0.10`).
2. **Frequency-graded `(k, c, m)` springs (`B2`, `+2`):** In `deriveVisualizerSpringParams`, give bass bars `1.35×` mass and slightly lower damping (deep, meaty bounce on kicks) and treble bars `0.65×` mass with higher stiffness (crisp, fast hi-hat flicks).
3. **Lateral wave coupling (`B3`, `+2`):** Feed `~18%` of each band's energy into its left/right neighbors with a 1-frame (`33 ms`) delay so hits ripple outward across adjacent bars like a physical wave.
4. **Layout (`B4a`, `+2`):** Map the 5-bar `compact`/`expanded` badge symmetrically (`[high-mid, low-mid, bass+kick, mid-vocal, treble]`) so the centre bar anchors the beat, and map the 14-bar `wide` waveform scrubber (`TrackActivity.svelte`) from bass/kick on the left to vocal/treble shimmer on the right.
