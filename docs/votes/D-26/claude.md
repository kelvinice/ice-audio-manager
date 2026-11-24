# D-26 · Claude's vote

*Written sealed on 2026-11-24 from the entry and my own checks. I didn't open `gemini.md` (or its history). Codex
hadn't voted yet.*

## Scores

### Part A · signal and frequency source (pick one)
| Option | Score | One-line reason |
|---|---|---|
| A0 peak + BeatShaper today | −1 | Peaks carry no pitch; between onsets the bars sit at the body level, the "flat" the owner wants gone |
| A1 peak + multi-envelope wave shaper | +1 | Cheap and worth having as the fallback for A2, but still one loudness number dressed up as bands |
| A2 loopback + 14-band IIR bank | +2 | Real bands, no crate, no window latency, fits the existing 30 Hz meter tick; needs 4th-order bands (see amendment) |
| A3 loopback + 1024/2048 FFT | +1 | Sharper low-end bins, but a 21–43 ms window of latency and more code for 14 bars that 4th-order IIR bands separate well enough |

### Part B · shaping, physics and layout (pick many; B4a vs B4b pick-higher)
| Option | Score | One-line reason |
|---|---|---|
| B1 per-band AGC + pink tilt | +2 | Essential with real bands: without tilt, treble bars never leave the floor while bass pegs the top |
| B2 frequency-graded springs | +1 | Heavier bass, snappier treble reads as "real"; keep it inside the existing presets and Off/reduce-motion rules |
| B3 neighbour wave coupling | −1 | With A2 the filters already overlap (62 % at the next band with 2nd-order Q 1.4, my calc below); extra bleed smears bands back into clumps. Useful only on the A1 fallback |
| B4a symmetric 5-bar + L→R 14-bar | +1 | The 5-bar badge already reads as centre-weighted (today's `beat.ts`), and a symmetric icon looks balanced; the wide scrubber reads left to right |
| B4b L→R bass→treble everywhere | 0 | Consistent, but a 5-bar icon with all the bass on its left edge looks lopsided at 12 px |

## Strongest argument against my top choice (A2)
It makes the app read **the full audio stream of everything the user hears**, a trust surface the project doesn't
have today: `code-practices.md` has no rule about reading audio content (I searched for record/capture/loopback/
microphone; nothing), and the project's founding constraint is "must not look like malware". The Windows 11 microphone
indicator staying off for render-endpoint loopback is still unverified on the owner's host. Exclusive-mode or DRM
streams give silence and Bluetooth/virtual drivers may refuse loopback, so the fallback path is real code to maintain.
And the bank exactly as written (2nd-order, Q ≈ 1.4) overlaps neighbours heavily (−4.2 dB, 62 %), so bars would still
move in clumps: A2 only pays off with the steeper bands below.

## New option or amendment
**A2 with four conditions:**
1. **4th-order bands:** two cascaded biquads per band (Q ≈ 1.4). Measured overlap at the entry's 0.64-octave spacing:
   next band −8.4 dB (38 %) instead of −4.2 dB (62 %), two bands away −19 dB instead of −9.5 dB. Cost doubles to
   ~7 MFLOP/s at 48 kHz, still tiny. (`f32` state is fine: see evidence.)
2. **Keep the beat accent:** run `BeatShaper`'s onset detector on the lowest 2–3 bands, so a kick still throws the
   "new pattern per beat" pop the owner liked, on top of real bands. The full `BeatShaper` stays the A1 fallback.
3. **Trust rule first:** add to code-practices §1 that loopback samples are reduced in memory on `ice-meter` to band
   levels only, never stored, logged or sent, and read only while bars are visible. Disclose it on Settings → Privacy
   ("Reads what you hear to draw the bars; nothing leaves this PC") with a switch back to meter-only bars. QA gate on
   the owner's host: the microphone indicator stays off.
4. **No new thread or timer:** drain the loopback buffer (≥ 100 ms) once per existing 30 Hz meter tick
   (`crates/ice-win/src/meter.rs:155`), so wake-ups stay exactly as today.

## Evidence I checked myself
- **Filter precision (simulation, node, 3 s at 48 kHz, RBJ band-pass, Q 1.4, direct form):** `f32` vs `f64` steady-state
  output at 45, 90 and 1000 Hz differs by 0.00 % at the centre frequency and 8× above it. My suspicion that low bands
  need `f64` was wrong; `f32` is fine.
- **Band overlap (closed-form |H| at the entry's spacing, ratio 1.555 = 0.64 octave):**
  2nd-order Q 1.4 → next band −4.2 dB (62 %), two away −9.5 dB; Q 2.2 → −7.0 dB / −13.1 dB;
  two cascaded Q 1.4 → −8.4 dB (38 %) / −19.0 dB.
- `crates/ice-win/src/meter.rs:112–155`: one `ice-meter` loop that sleeps `METER_INTERVAL` (33 ms) per tick, so
  loopback reads fit in the same tick with no new wake-ups.
- `Cargo.toml:46–47`: `Win32_Media_Audio` and `Win32_Media_Audio_Endpoints` are already enabled workspace-wide.
- `docs/code-practices.md`: no rule covers reading audio content (searched record/capture/loopback/microphone).
- `app/ui/src/lib/notch/beat.ts`: I wrote it today; its tests (`beat.test.ts`) show it swings well on kicks but has no
  pitch, so sustained melodic passages sit at the body level, matching the entry's facts.

## Position
Go for real bands, done carefully: WASAPI loopback with a 4th-order 14-band IIR bank (A2), per-band AGC and pink tilt
(B1), gently graded springs (B2), centre-bass on the 5-bar badge and low-to-high on the wide scrubber (B4a). Keep the
beat accent from `BeatShaper` on the low bands, and keep `BeatShaper` itself as the automatic fallback. Before any of
it ships: a code-practices trust rule for reading audio, a Privacy disclosure with a switch, and a host check that the
microphone indicator stays off.
