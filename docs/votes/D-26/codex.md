# D-26 · Codex's vote

## Scores

| Option | Score | One-line reason |
|---|---|---|
| A0 · Current beat shaper | −1 | A change of instrument can leave every peak reading identical and the bar pattern frozen. |
| A1 · Travelling wave from peaks | +1 | Delayed envelopes can give the peak-only fallback a lively shape, though two sounds with the same envelope still drive the same wave. |
| A2 · Loopback with 14 IIR bands | +2 | Fourteen fixed display bands can follow a bass note and a cymbal independently, with filter state carried across capture packets. |
| A3 · Loopback with FFT | +1 | Grouping bins gives meaningful movement too. The lowest bands need testing because a short window gives them few bins. |
| B1 · Adaptive range and treble tilt | 0 | Range adaptation can reveal quieter instruments, but boosting and normalizing a nearly empty band can make hiss as prominent as the melody. |
| B2 · Graded springs | +2 | A bass bar that settles slowly and a treble bar that releases quickly can make a kick and hi-hat visually distinct even when they strike together. |
| B3 · Neighbor coupling and motion floor | −1 | A little bleed can soften isolated spikes, but a minimum motion floor can obscure the moment an instrument cuts out. |
| B4a · Centre bass on five bars, ascending on fourteen | +2 | The kick gives the tiny badge a legible centre. The wide scrubber has room to show where a note sits in the spectrum. |
| B4b · Ascending on both | +1 | Consistent ordering makes a frequency sweep easy to follow, though bass-heavy passages put most badge motion on one edge. |

## Strongest argument against my top choice

A2 adds a capture client, format handling, filter tuning and endpoint recovery for a badge often drawn just five bars wide. A1 could be equally satisfying at that size and it already takes the same input as the fallback. I would compare both at actual badge size on a sustained passage and a drum break before making loopback the default. The existing peak-meter CPU figure cannot establish the cost of either loopback design.

## New option or amendment

Keep endpoint peaks in `channels` and put the fourteen frequency levels in a separate optional `bands` field. Frame suppression must compare the bands too. As an acceptance check, change from a steady low tone to a steady high tone at the same peak. The active bars should move across the wide display, then return to rest on silence. On endpoint rebind or fallback, discard old band and filter state so the previous device's spectrum cannot linger.

## Evidence I checked myself

- I generated 375 Hz and 6,000 Hz cosines at 48 kHz and amplitude 0.5. For each tone I took the maximum absolute sample in ten successive 1,600-sample blocks. All twenty peaks were 0.5. This was a synthetic input check, not a capture benchmark.
- At revision `c312d2f`, `crates/ice-core/src/meter.rs:35` defines `channels` in endpoint order. At line 58, `MeterFrame::differs` compares channel count, peak and channel values. Unchanged peaks would suppress the tone transition unless bands participate in this decision.
- `crates/ice-win/src/meter.rs:125` drops closed subscribers before reading the device, and line 134 checks the endpoint generation before binding. Both the loopback stream and filter history belong inside this lifecycle.
- `app/ui/src/lib/notch/visualizer.ts:469` checks visibility and activity on each animation frame and only schedules another while the springs move. A persistent motion floor would need a different stop condition.

## Position

I favor A2 with B2 and B4a. The musical cue I want is a sustained low note holding the centre of the badge while a short high note flicks the edges. A1 is a worthwhile fallback. Gate any treble boost on audible band energy, and preserve a quiet resting shape when playback falls silent.
