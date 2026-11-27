# Pandora playback

> Paths: `pandora/ui/src/lib/{audio,session,spectrum,wave,parametricEq}.ts`; components `PlayerBar`, `NowPlayingOverlay`,
> `NowPlayingBackdrop`, `SeekBar`, `VolumeControl`, `OutputPicker`, `QueuePanel`, `EqDrawer`, `EqBand`, `HeartButton` ·
> Part of [Pandora](README.md)

## Engine (`lib/audio.ts`)

`audioEngine` (one `PandoraAudioEngine`) owns an `<audio>` element and `playerStore` (queue, index, track, time,
volume, shuffle / repeat, EQ, lyrics state). Sources come from the local proxy (`/stream/yt/<id>`, `/stream/local`).
The Web Audio graph is built on first play:

```
source → 10 biquads (32 Hz–16 kHz) → preamp → [AutoEq parametric filters] → compressor → analyser → gain → out
```

- EQ: presets in `EQ_PRESETS`; AutoEq `ParametricEQ.txt` import (`lib/parametricEq.ts`) adds peaking / shelf / pass
  filters plus preamp. Normalize turns the compressor on (threshold −20 dB, ratio 5).
- Skip silence: while playing, RMS from the analyser; quiet for > 450 ms jumps ahead.
- Speed (`playbackRate`), sleep timer (minutes or end of track).
- `playTrack(track, queue)` sets the queue, records history, loads lyrics, extracts cover colours, updates the media
  session. Autoplay radio (`autoQueueRelated`) appends `ytm_related` songs when two or fewer remain.
- Shuffle (`playerStore.shuffle`: `off` → `on` → `magic`, `cycleShuffle`): plays every song of the list once per pass
  (`shufflePlayed`), then stops unless repeat all is on. A new list (`noteSource`: its song ids, Magic additions
  excluded) starts a new pass.
- Magic shuffle (`lib/magicShuffle.ts`): after every 2–4 list songs, `next()` plays a recommended song, inserted right
  after the current one. Recommendations come from `ytm_recommend` seeded by the list, a batch at a time (each batch
  skips earlier suggestions); `playerStore.magicIds` marks them with a sparkle in the queue panel. Offline, it simply
  shuffles.
- Windows media controls through `navigator.mediaSession` (when enabled); Ice's notch picks this up via SMTC.
- `applySettings` is called with every settings change (artwork accents, media session).

## Last session (`lib/session.ts`)

Queue, index and position in `localStorage` (`pandora.session.v1`), saved every 5 s while playing, on pause and on
`pagehide`. At launch `restoreSession(history[0])` shows that song paused at its spot without streaming; Play resumes
there. If the saved song is no longer the latest in history, the latest song starts from 0:00.

## Player bar and panels

`PlayerBar`: cover, title, heart, transport, `SeekBar`, `VolumeControl` (wheel 5 %, arrows 2 %), `OutputPicker`
(devices with headphone / speaker icons), queue and EQ toggles. `QueuePanel` (right side): Queue and Recently played;
open state in `localStorage` (`pandora.queuePanelOpen`). `EqDrawer`: right sheet with `EqBand` sliders, response curve,
presets, sound toggles, speed, sleep timer, AutoEq import.

## Seek bar (`SeekBar.svelte`, `lib/wave.ts`)

Follows Ice's notch scrubber: one pointer slider with keyboard steps, a rail that thickens on hover, and with
`wavyProgress` a travelling wave on the played part (`wave.ts`, Twin of `app/ui/src/lib/notch/waves.ts`), animated at
most 30 Hz while playing, with a round accent dot at the play head (always shown with the wave; on hover otherwise).

## Now Playing (`NowPlayingOverlay`, `NowPlayingBackdrop`)

Cover (optionally spinning), title, transport, lyrics column. Backdrop (`NowPlayingBackdrop`): blurred cover + tint;
with Animated Backdrop, art-coloured orbs (`--art-*`) that pause with the song, in one of two motions
(`settings.backdropMotion`):

- **Calm** (default): five orbs on slow circles.
- **Lively** (`LavaLamp.svelte`, after Ice's `NotchBackdrop.svelte`): nine orbs, each looping around its own spot while
  its distance and size change (one `wander` keyframe driven by per-orb CSS variables, periods 26 s × 0.77–1.37), and
  the field swells on bass kicks (`lib/breath.ts`, Twin of Ice's `BreathEnvelope`; kicks are the 55–110 Hz level rising
  over its recent average). The breath samples the analyser at most 30 Hz, only while playing and Now Playing is open;
  not at all with reduced motion.

Spectrum: a 4096-point FFT (~12 Hz bins) read as
log-spaced bands from 55 Hz to 15 kHz (`getSpectrumDb`; bands narrower than two bins interpolate at their centre, so
bass bars don't move in lockstep), shaped by `lib/spectrum.ts` (Twin of Ice's `beat.ts` / `visualizer.ts`: per-band
range, tilt, gate, springs, resting texture; Pandora adds an −8 dB bass trim and caps the lowest bars at 72 % height,
opening to full over the first 30 % of bars), drawn DPR-sharp at 30 Hz while playing, all in the accent colour.
