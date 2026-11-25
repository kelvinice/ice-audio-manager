# D-28 · Claude's vote

## Scores

*Part A (pick one)*

| Option | Score | One-line reason |
|---|---|---|
| A1 restyle in place | −1 | Fixes the look, keeps the position cue that causes the mix-up |
| A2 song block + output strip | +2 | Moves volume out of the progress slot and groups it with the device it belongs to |
| A3 volume chip, slider on demand | +1 | Removes the confusion completely, at the cost of a click and a new state |
| A4 vertical volume at the edge | −1 | Short travel, and the Nook ear / lyrics split / full Floating rail each need special cases |

*Part B (pick many)*

| Option | Score | One-line reason |
|---|---|---|
| B1 two visual languages | +2 | Needed with any A; cheap (tokens only) |
| B2 volume never jumps up on a click | +2 | A misclick should never be loud; seeking can stay click-to-jump |
| B3 hover-arm (~200 ms) | +1 | Targets the "passing pointer" case directly; needs care not to feel laggy |
| B4 value tooltip | +1 | Shows what is about to change; small cost |
| B5 wheel over progress seeks | −1 | One gesture with two meanings on a 4 px target |

*Part C (pick many)*

| Option | Score | One-line reason |
|---|---|---|
| C1 the volume pop-up names itself | +1 | Keeps the compact states consistent with B1 |
| C2 no change to the music pill | +1 | Its waveform already reads as the song, not a slider |

## Strongest argument against my top choice (A2)

It is the biggest change of the four: both card layouts and their fixed heights move, and the output strip gets crowded
with three or more device chips, which then needs a second rule (collapse to icon + %). A1 + B1 + B2 might solve most of
the reported problem for a tenth of the work, and we would only learn whether position matters by shipping A1 first.

## New option or amendment

**A2 + A3 hybrid:** A2's output strip shows the compact slider while it fits, and falls back to A3's chip (wheel +
click-to-open) when the chips need the room, instead of a separate collapse rule. The same component then serves both
the 408 px and 572 px layouts and Floating.

## Evidence I checked myself

- `app/ui/src/notch/activities/ExpandedVolumeRow.svelte`: `.vol-rail` 5 px (7 px on hover), gradient fill; `onPointerDown`
  sends the clicked value immediately (`send(dragPercent, true)`).
- `app/ui/src/notch/activities/MediaScrubber.svelte`: `.scrubber-rail` 3 px (5 px on hover), solid `--ice-active-accent`.
- `app/ui/src/notch/activities/NookExpandedNotch.svelte`: `.controls` (volume + chips) comes after `NookMediaPanel`, whose
  `.transport` row is last, so the volume rail is the first bar under play/pause.
- `app/ui/src/lib/notch/controller.svelte.ts` `onPointerEnter`: the hover timer opens the card after 120 ms.
- The owner's screenshot (2026-11-25): the 13 % volume bar spans the card under the transport; the progress line is
  shorter and thinner beside the cover.

## Position

The mix-up has two causes: **identity** (both bars look alike, and volume looks more important) and **position** (volume
sits where players put progress). A1 fixes only identity. A2 fixes both with no new interaction to learn, and its
song/output split also gives D-20 a clean unit to flip at bottom anchors. B1 and B2 should ship with whichever A wins:
B2 alone removes the worst outcome (a loud jump from a misclick). I'd ship A2 + B1 + B2 + C1 first and add B3/B4 if the
mix-up still happens.
