# Pandora lyrics

> Paths: `pandora/src-tauri/src/lyrics.rs`, `pandora/ui/src/lib/lyricWords.ts`, components `NowPlayingOverlay`
> (Default style), `LyricsFisheye`, `LyricsRain` · Part of [Pandora](README.md)

## Fetching (`lyrics.rs`)

`fetch_lyrics(title, artist, album, duration)` uses `ice-win`'s LRCLIB client and `ice-core`'s LRC parser. Every line
gets word timings: the provider's own when present, otherwise `ice_core::lyrics::estimate_word_timings` (share of the
line by word length, capped at a natural singing span, shared with Ice's notch). Romaji / romanization (Japanese,
Korean, Cyrillic) comes from `ice-core`; the fetch joins a COM STA so kanji readings work. Romaji shows in every style
when Lyrics Romanization is on.

## Styles (`settings.lyricsStyle`)

Chosen from the lyrics icon menu in Now Playing's top bar (Rust `LyricsStyle`, unknown values read as Default).

| Style | Component | Behaviour |
|---|---|---|
| Default | `NowPlayingOverlay` | Scrolling list, current line by colour; follows the song until scrolled, then a Sync button re-centres (auto-follow pauses 3 s after a manual scroll) |
| Fisheye | `LyricsFisheye` | Lens over the current line: neighbours shrink, fade and blur with distance |
| Rain | `LyricsRain` | One line at a time; words drop in letter by letter, finished lines fall away while the next drops in |

## Word timing (`lib/lyricWords.ts`)

- `timedWords(line, nextStart)`: the line's word timings, or an even split by length when it has none.
- `quickenWords(words, lineStart, pace, maxStepMs)`: reveals words sooner. Rain uses pace 0.72 and 340 ms per word,
  because estimated timings lag the voice (they spread words over the whole gap to the next line).
- `joinsWithoutSpaces`: Japanese / Chinese lines join pieces without spaces.

Designs and the plan for more styles: [pandora-lyric-styles.md](../../pandora-lyric-styles.md).
