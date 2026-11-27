# Pandora lyric styles

> Status: Default, Fisheye and Rain are in (2026-11-27). Branches and Pulse reveal are planned below.
> Owner request with an outside "lyric styles" image as inspiration only: the designs and code here are our own
> (code-practices §1.8, reference-never-copy).

The style is a Pandora setting (`PandoraSettings.lyricsStyle`, Rust `LyricsStyle`; unknown values read as `default`),
picked from the segmented control in Now Playing's lyrics header. Every style shares one input: the synced lines
from `playerStore.lyrics`, the active line index the overlay already computes, the playback time, and word timings
from `lib/lyricWords.ts` (`timedWords`: the provider's word times, or the line's time shared out by word length; lines
without spaces are timed per character). Plain or missing lyrics always fall back to Default.

## Shipped

| Style | Component | What it does | Motion |
|---|---|---|---|
| Default | `NowPlayingOverlay` | Scrolling list, current line by colour, follow + Sync | smooth scroll on line change |
| Fisheye | `LyricsFisheye.svelte` | Lens over the current line; up to 3 neighbours each side shrink (0.6 / 0.44 / 0.34), fade and blur with distance; click a line to seek | `font-size` / opacity / blur transitions on line change only |
| Rain | `LyricsRain.svelte` | One line, centred; each word's letters drop in as sung; the finished line's letters fall away with their own drift, spin, speed and delay; next line hinted at the bottom | CSS keyframes per letter, no script loop; reduced motion = fade |

Neither new style runs a timer: everything is driven by `activeLineIdx` / `currentTimeMs`, which already update.

## Planned: Branches

**Idea.** The song as a branch: a thin vertical trunk on the left with a node for every line. Each line hangs off its
node on a short twig. The trunk fills down to the current node like a progress line, the current node lights up, and
romaji / translations sit on a smaller sub-twig under their line.

**Layout.** Reuse Default's scroll container, follow mode and Sync button (extract them into a `LyricsScroller`
first, so Default and Branches share them). Each line is a button with a `::before` node (8 px ring) and a twig
(`::after`, 14 px). The trunk is one absolutely positioned 2 px element behind the nodes; the filled part is a second
element whose height is set to the active node's `offsetTop` on line change, animated with `transform: scaleY`
(measured once per change, not per frame).

**Repeats.** Lines whose text repeats (choruses) get the same twig colour, from a small map of normalised text →
palette slot (`--art-primary/secondary/tertiary`), so the song's structure is visible at a glance.

**Motion.** On a new line: the node pops (scale 1.4 → 1, 300 ms), its twig draws in (`clip-path: inset(0 100% 0 0)`
→ `inset(0)`), the trunk fill grows to it. Nothing animates between line changes. Reduced motion: no pop or draw,
colour only.

**Interaction.** Click a line or node to seek; hovering a node shows its time stamp. Keyboard: lines stay buttons.

**Tests.** A pure `repeatGroups(lines)` helper (text → slot) with a Vitest-style unit test; visual check at 100 % and
150 % scaling in Midnight and Daylight.

## Planned: Pulse reveal

**Idea.** The current line, large and centred, revealed word by word: a word appears as it starts, the word being
sung is in the accent colour and gently pulses with the music's low end, sung words settle to full white, and the
previous line slides up small and fades.

**Beat source.** No new analyser: read the bass bands from `audioEngine.getSpectrumBars` (the log-spaced bands added
for the visualizer) and run them through a small envelope follower (fast attack ~30 ms, slow release ~180 ms) in a
pure `bassPulse(levels, prev, dtMs)` helper. One `requestAnimationFrame` loop capped at 30 Hz writes a single CSS
variable (`--pulse`, 0..1) on the line element. The loop runs only while the style is shown, the song is playing
and the window is visible (`document.hidden` stops it), and stops otherwise.

**Rendering.** Words are inline-block spans with three states (upcoming: hidden; current: accent, `scale(1 +
0.06 * var(--pulse))`; sung: text colour). The line enters with a short rise and leaves as a small faded copy above,
kept until the next change, all transform / opacity.

**Reduced motion.** No pulse loop and no rise: words still reveal and the current word is still accent-coloured.

**Tests.** Unit tests for `bassPulse` (attack and release shape, stays in 0..1, silence decays to 0); visual check
while playing and paused (the loop must stop when paused).

## Order of work

1. Extract `LyricsScroller` (Default's scroll, follow and Sync) so Branches can reuse it.
2. Branches (no loop; lowest risk).
3. `bassPulse` helper with tests, then Pulse reveal.
4. Add both to `LyricsStyle` (Rust + TS) and the picker; document them in [modules/pandora/lyrics](modules/pandora/lyrics.md).
