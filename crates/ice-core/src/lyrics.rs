//! Synced lyrics domain types, LRC/Enhanced-LRC parser, word cadence estimator, and pure Kana → Hepburn Romaji.
//!
//! Pure Rust (`#![forbid(unsafe_code)]`, zero Windows or network dependencies). See `docs/lyrics.md`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::media::content_id;

/// Default lyrics disk cache ceiling, in MiB.
pub const LYRICS_CACHE_MAX_MB_DEFAULT: u16 = 50;
/// Allowed lyrics disk cache ceiling range, in MiB.
pub const LYRICS_CACHE_MAX_MB: std::ops::RangeInclusive<u16> = 5..=500;
/// Maximum delta (ms) between two consecutive timestamps treated as a paired primary + subtitle line.
const PAIRED_SUBTITLE_DELTA_MS: u64 = 20;
/// Fallback display duration for the last lyric line when total track duration is unknown.
const LAST_LINE_FALLBACK_MS: u64 = 5_000;

/// User configuration for synced lyrics, Privacy opt-in, Romaji subtitles, and Storage cache limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LyricsConfig {
    /// Master toggle for displaying lyrics on the notch (off by default).
    pub enabled: bool,
    /// Option A (LRCLIB `https://lrclib.net`): strictly opt-in from Settings → Privacy (§1.5).
    pub online_lookup: bool,
    /// Attach the compact bottom lyrics extension under the music pill.
    pub show_on_pill: bool,
    /// Attach the 3-line bottom lyrics extension under the expanded card.
    pub show_on_card: bool,
    /// Highlight the active word across each synced line (Enhanced LRC or weighted phrase cadence).
    pub word_highlight: bool,
    /// Generate an offline Romaji subtitle line underneath Japanese lyric lines.
    pub romaji: bool,
    /// Maximum managed lyrics cache size on disk, in MiB ([`LYRICS_CACHE_MAX_MB`]).
    pub cache_max_mb: u16,
}

impl Default for LyricsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            online_lookup: false,
            show_on_pill: true,
            show_on_card: true,
            word_highlight: true,
            romaji: true,
            cache_max_mb: LYRICS_CACHE_MAX_MB_DEFAULT,
        }
    }
}

/// Where a [`LyricsDocument`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub enum LyricsSource {
    /// User-placed `.lrc` file in `%LOCALAPPDATA%\IceAudioManager\lyrics\custom\`.
    Local,
    /// Previously fetched record served from `%LOCALAPPDATA%\IceAudioManager\lyrics\cache\`.
    Cache,
    /// Freshly fetched from `https://lrclib.net`.
    Lrclib,
}

/// Timing granularity of a [`LyricsDocument`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub enum LyricsSyncKind {
    /// True Enhanced LRC word-level timestamps (`<mm:ss.xx>`).
    Word,
    /// Standard line-level LRC timestamps (`[mm:ss.xx]`) with estimated word cadence.
    Line,
    /// Unsynced plain text fallback.
    Plain,
}

/// Current lyrics fetch/display status for the active track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub enum LyricsStatus {
    #[default]
    Disabled,
    Idle,
    Loading,
    Ready,
    NotFound,
}

/// One timed word (or CJK clause token) inside a synced [`LyricLine`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct LyricWord {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub start_ms: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub end_ms: u64,
    pub text: String,
}

/// One timestamped lyric line with optional paired/Romaji subtitle and word-level timings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub start_ms: u64,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub end_ms: Option<u64>,
    pub text: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub words: Vec<LyricWord>,
}

/// Canonical lyrics payload shared across providers, disk cache, engine, and UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct LyricsDocument {
    pub track_key: String,
    pub source: LyricsSource,
    pub sync_kind: LyricsSyncKind,
    #[serde(default)]
    pub instrumental: bool,
    #[serde(default)]
    pub lines: Vec<LyricLine>,
    #[serde(default)]
    pub plain_text: Option<String>,
}

/// Snapshot/event state of lyrics for the currently playing track.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct LyricsState {
    pub status: LyricsStatus,
    #[serde(default)]
    pub track_key: Option<String>,
    #[serde(default)]
    pub document: Option<LyricsDocument>,
}

/// Disk usage summary reported lazily to `Settings → Storage`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct LyricsStorageStats {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub cache_bytes: u64,
    pub cache_files: u32,
    pub custom_files: u32,
    pub cache_max_mb: u16,
    pub custom_dir: String,
}

/// Metadata sent to a [`LyricsProvider`] (never includes user identifiers or secrets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackQuery {
    pub track_key: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: Option<u64>,
}

/// Extensibility seam for lyrics backends (Option A LRCLIB, Option B local `.lrc`, future Option C providers).
pub trait LyricsProvider: Send + Sync {
    /// Looks up lyrics for `query`. `Ok(None)` = clean miss; `Err(_)` = transient failure.
    ///
    /// # Errors
    /// Returns a human-readable error string if the provider failed (e.g. network timeout or I/O error).
    fn fetch(&self, query: &TrackQuery) -> Result<Option<LyricsDocument>, String>;
}

/// Stable track identity string matching the shell/UI track key (`app_id`, `title`, `artist`, `album`).
#[must_use]
pub fn track_key(app_id: &str, title: &str, artist: &str, album: &str) -> String {
    format!("{app_id}\0{title}\0{artist}\0{album}")
}

/// Stable 16-hex-digit cache key derived from normalized track metadata + 2-second duration bucket.
#[must_use]
pub fn track_cache_key(title: &str, artist: &str, album: &str, duration_ms: Option<u64>) -> String {
    let norm = |s: &str| s.trim().to_lowercase();
    let bucket = duration_ms.map_or(0, |ms| (ms + 1_000) / 2_000);
    let key = format!("{}|{}|{}|{bucket}", norm(artist), norm(title), norm(album));
    content_id(key.as_bytes())
}

/// Strips common streaming suffixes like `(feat. ...)`, `- Remastered 2011`, `[Official Video]` for fallback search.
#[must_use]
pub fn normalize_query_title(title: &str) -> String {
    let mut s = title.trim().to_string();
    for (open, close) in [('(', ')'), ('[', ']')] {
        while let Some(start) = s.find(open) {
            let Some(rel_end) = s[start + 1..].find(close) else { break };
            let inner = s[start + 1..start + 1 + rel_end].to_lowercase();
            let noisy = ["feat", "ft.", "remaster", "official", "lyric", "video", "audio", "version", "live", "edit"]
                .iter()
                .any(|kw| inner.contains(kw));
            if !noisy {
                break;
            }
            s.replace_range(start..=start + 1 + rel_end, " ");
        }
    }
    if let Some((head, tail)) = s.split_once(" - ") {
        let lower = tail.to_lowercase();
        if ["remaster", "live", "mono", "stereo", "radio edit", "single", "version", "mix", "instrumental"]
            .iter()
            .any(|kw| lower.contains(kw))
        {
            s = head.to_string();
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Sanitizes a track/artist string into a safe Windows filename stem (for local `.lrc` lookup).
#[must_use]
pub fn sanitize_filename(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    cleaned.trim().trim_end_matches('.').split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parses an LRC or Enhanced LRC (`<mm:ss.xx>`) string into sorted [`LyricLine`]s.
///
/// Supports `[mm:ss.xx]`, `[mm:ss.xxx]`, `[mm:ss]`, repeated timestamps on one line, `[offset:±ms]`,
/// dual-timestamp paired subtitles, Enhanced LRC inline word timings, and weighted word cadence fallback.
#[must_use]
pub fn parse_lrc(lrc: &str, duration_ms: Option<u64>) -> (LyricsSyncKind, Vec<LyricLine>) {
    let mut offset_ms: i64 = 0;
    let mut raw_entries: Vec<(u64, String, Vec<LyricWord>, usize)> = Vec::new();
    let mut any_word_sync = false;

    for (line_idx, raw_line) in lrc.lines().enumerate() {
        let trimmed = raw_line.trim().trim_start_matches('\u{feff}');
        if trimmed.is_empty() {
            continue;
        }
        if let Some(off) = parse_offset_tag(trimmed) {
            offset_ms = off;
            continue;
        }
        let (stamps, body) = extract_line_timestamps(trimmed);
        if stamps.is_empty() {
            continue;
        }
        let (plain_body, inline_words) = parse_enhanced_words(body);
        if !inline_words.is_empty() {
            any_word_sync = true;
        }
        let text = plain_body.trim().to_string();
        for stamp in stamps {
            let shifted = shift_ms(stamp, offset_ms);
            let words = inline_words
                .iter()
                .map(|w| LyricWord {
                    start_ms: shift_ms(w.start_ms, offset_ms),
                    end_ms: shift_ms(w.end_ms, offset_ms),
                    text: w.text.clone(),
                })
                .collect();
            raw_entries.push((shifted, text.clone(), words, line_idx));
        }
    }

    // Stable sort by timestamp, preserving file order for same-timestamp paired subtitles.
    raw_entries.sort_by_key(|e| (e.0, e.3));

    // Group consecutive lines sharing the same timestamp (≤ 20 ms) into primary + subtitle.
    let mut lines: Vec<LyricLine> = Vec::with_capacity(raw_entries.len());
    for (start_ms, text, words, _) in raw_entries {
        if let Some(last) = lines.last_mut()
            && start_ms.saturating_sub(last.start_ms) <= PAIRED_SUBTITLE_DELTA_MS
            && last.subtitle.is_none()
            && !last.text.is_empty()
            && !text.is_empty()
        {
            last.subtitle = Some(text);
            continue;
        }
        lines.push(LyricLine { start_ms, end_ms: None, text, subtitle: None, words });
    }

    // Fill end_ms from the next line (or track duration), finalize word end bounds, and estimate cadence.
    let len = lines.len();
    for i in 0..len {
        let next_start = lines.get(i + 1).map(|l| l.start_ms);
        let fallback_end = duration_ms
            .filter(|&d| d > lines[i].start_ms)
            .unwrap_or_else(|| lines[i].start_ms.saturating_add(LAST_LINE_FALLBACK_MS));
        let end_ms = next_start.unwrap_or(fallback_end).max(lines[i].start_ms.saturating_add(250));
        lines[i].end_ms = Some(end_ms);

        if lines[i].text.is_empty() {
            lines[i].words.clear();
            continue;
        }
        if lines[i].words.is_empty() {
            lines[i].words = estimate_word_timings(&lines[i].text, lines[i].start_ms, end_ms);
        } else {
            finalize_word_ends(&mut lines[i].words, end_ms);
        }
    }

    // Drop trailing empty marker lines that exist only to cap the final lyric line's duration.
    while lines.last().is_some_and(|l| l.text.is_empty()) {
        lines.pop();
    }

    let kind = if any_word_sync { LyricsSyncKind::Word } else { LyricsSyncKind::Line };
    (kind, lines)
}

fn shift_ms(base: u64, offset_ms: i64) -> u64 {
    if offset_ms >= 0 {
        base.saturating_add(offset_ms.unsigned_abs())
    } else {
        base.saturating_sub(offset_ms.unsigned_abs())
    }
}

fn parse_offset_tag(line: &str) -> Option<i64> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let (key, val) = inner.split_once(':')?;
    if !key.trim().eq_ignore_ascii_case("offset") {
        return None;
    }
    val.trim().parse::<i64>().ok()
}

fn parse_timestamp_inner(inner: &str) -> Option<u64> {
    let (min_str, rest) = inner.split_once(':')?;
    if min_str.is_empty() || !min_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mins = min_str.parse::<u64>().ok()?;
    let (sec_str, frac_ms) = if let Some((s, f)) = rest.split_once('.').or_else(|| rest.split_once(':')) {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) || !f.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let frac = match f.len() {
            0 => 0,
            1 => f.parse::<u64>().ok()? * 100,
            2 => f.parse::<u64>().ok()? * 10,
            _ => f[..3].parse::<u64>().ok()?,
        };
        (s, frac)
    } else {
        if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        (rest, 0)
    };
    let secs = sec_str.parse::<u64>().ok()?;
    Some(mins.saturating_mul(60_000).saturating_add(secs.saturating_mul(1_000)).saturating_add(frac_ms))
}

fn extract_line_timestamps(mut s: &str) -> (Vec<u64>, &str) {
    let mut stamps = Vec::new();
    while let Some(rest) = s.strip_prefix('[') {
        let Some(end) = rest.find(']') else { break };
        let Some(ms) = parse_timestamp_inner(&rest[..end]) else { break };
        stamps.push(ms);
        s = &rest[end + 1..];
    }
    (stamps, s)
}

fn parse_enhanced_words(body: &str) -> (String, Vec<LyricWord>) {
    if !body.contains('<') {
        return (body.to_string(), Vec::new());
    }
    let mut words: Vec<LyricWord> = Vec::new();
    let mut plain = String::with_capacity(body.len());
    let mut rest = body;
    let mut active_start: Option<u64> = None;

    while let Some(open) = rest.find('<') {
        let before = &rest[..open];
        if !before.is_empty() {
            plain.push_str(before);
            if let Some(start_ms) = active_start {
                words.push(LyricWord { start_ms, end_ms: start_ms, text: before.to_string() });
                active_start = None;
            }
        }
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('>') else {
            plain.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let tag = &after_open[..close];
        if let Some(ts) = parse_timestamp_inner(tag) {
            active_start = Some(ts);
        } else {
            plain.push('<');
            plain.push_str(tag);
            plain.push('>');
        }
        rest = &after_open[close + 1..];
    }
    if !rest.is_empty() {
        plain.push_str(rest);
        if let Some(start_ms) = active_start {
            words.push(LyricWord { start_ms, end_ms: start_ms, text: rest.to_string() });
        }
    }
    if words.is_empty() { (body.to_string(), Vec::new()) } else { (plain, words) }
}

fn finalize_word_ends(words: &mut [LyricWord], line_end_ms: u64) {
    let len = words.len();
    for i in 0..len {
        let next = words.get(i + 1).map_or(line_end_ms, |w| w.start_ms);
        words[i].end_ms = next.max(words[i].start_ms.saturating_add(80));
    }
}

/// Splits a lyric line into displayable tokens and distributes `[start_ms, end_ms]` weighted by visual length.
#[must_use]
pub fn estimate_word_timings(text: &str, start_ms: u64, end_ms: u64) -> Vec<LyricWord> {
    let tokens = tokenize_lyric_line(text);
    if tokens.is_empty() {
        return Vec::new();
    }
    let total_span = end_ms.saturating_sub(start_ms).max(200);
    let weights: Vec<u64> = tokens.iter().map(|t| token_weight(t)).collect();
    let total_weight: u64 = weights.iter().sum::<u64>().max(1);

    let mut out = Vec::with_capacity(tokens.len());
    let mut acc_weight = 0u64;
    for (idx, (token, weight)) in tokens.into_iter().zip(weights).enumerate() {
        let w_start = start_ms.saturating_add(total_span.saturating_mul(acc_weight) / total_weight);
        acc_weight = acc_weight.saturating_add(weight);
        let w_end = if idx + 1 == out.capacity() {
            end_ms.max(w_start.saturating_add(60))
        } else {
            start_ms.saturating_add(total_span.saturating_mul(acc_weight) / total_weight).max(w_start + 40)
        };
        out.push(LyricWord { start_ms: w_start, end_ms: w_end, text: token });
    }
    out
}

fn tokenize_lyric_line(text: &str) -> Vec<String> {
    if !contains_japanese(text) {
        return text.split_whitespace().map(str::to_string).collect();
    }
    // For Japanese/CJK lines without spaces, group runs into small readable clusters (Latin words intact,
    // CJK/Kana in 2-character rhythmic chunks or separated by spaces/punctuation).
    let mut tokens = Vec::new();
    let mut buf = String::new();
    let mut cjk_count = 0usize;
    let mut in_latin = false;

    for ch in text.chars() {
        if ch.is_whitespace() {
            if !buf.is_empty() {
                tokens.push(std::mem::take(&mut buf));
                cjk_count = 0;
                in_latin = false;
            }
            continue;
        }
        let is_jp = is_japanese_char(ch);
        if is_jp {
            if in_latin && !buf.is_empty() {
                tokens.push(std::mem::take(&mut buf));
                in_latin = false;
            }
            buf.push(ch);
            cjk_count += 1;
            if cjk_count >= 2 && !is_small_kana(ch) {
                tokens.push(std::mem::take(&mut buf));
                cjk_count = 0;
            }
        } else {
            if !in_latin && !buf.is_empty() {
                tokens.push(std::mem::take(&mut buf));
                cjk_count = 0;
            }
            in_latin = true;
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        tokens.push(buf);
    }
    tokens
}

fn token_weight(token: &str) -> u64 {
    let mut w = 0u64;
    for ch in token.chars() {
        if is_japanese_char(ch) {
            w += 2;
        } else if ch.is_alphanumeric() {
            w += 1;
        }
    }
    w.max(1)
}

#[must_use]
pub fn is_hiragana(c: char) -> bool {
    ('\u{3040}'..='\u{309F}').contains(&c)
}

#[must_use]
pub fn is_katakana(c: char) -> bool {
    ('\u{30A0}'..='\u{30FF}').contains(&c)
}

#[must_use]
pub fn is_kanji(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3400}'..='\u{4DBF}').contains(&c)
}

#[must_use]
pub fn is_japanese_char(c: char) -> bool {
    is_hiragana(c) || is_katakana(c) || is_kanji(c)
}

#[must_use]
pub fn contains_japanese(text: &str) -> bool {
    text.chars().any(is_japanese_char)
}

#[must_use]
pub fn contains_kanji(text: &str) -> bool {
    text.chars().any(is_kanji)
}

fn is_small_kana(c: char) -> bool {
    matches!(
        c,
        'ゃ' | 'ゅ'
            | 'ょ'
            | 'ぁ'
            | 'ぃ'
            | 'ぅ'
            | 'ぇ'
            | 'ぉ'
            | 'ャ'
            | 'ュ'
            | 'ョ'
            | 'ァ'
            | 'ィ'
            | 'ゥ'
            | 'ェ'
            | 'ォ'
            | 'ー'
    )
}

/// Converts Hiragana and Katakana into clean Modified Hepburn Romaji (leaving Latin/Kanji intact).
#[must_use]
pub fn kana_to_romaji(input: &str) -> String {
    let chars: Vec<char> = input.chars().map(katakana_to_hiragana).collect();
    let mut out = String::with_capacity(input.len() * 2);
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        // Small tsu (gemination: doubles the following consonant, or "t" before "ch").
        if ch == 'っ' {
            if let Some((rom, _)) = lookup_kana(&chars[i + 1..]) {
                if rom.starts_with("ch") {
                    out.push('t');
                } else if let Some(first) = rom.chars().next()
                    && first.is_ascii_alphabetic()
                {
                    out.push(first);
                }
            }
            i += 1;
            continue;
        }
        // Prolonged sound mark repeats the previous vowel if present.
        if ch == 'ー' {
            if let Some(v) = out.chars().last().filter(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')) {
                out.push(v);
            }
            i += 1;
            continue;
        }
        // Syllabic 'n' uses apostrophe before vowel or 'y' to avoid ambiguity (e.g. shin'ai).
        if ch == 'ん' {
            out.push('n');
            if let Some((next_rom, _)) = lookup_kana(&chars[i + 1..])
                && next_rom.starts_with(['a', 'i', 'u', 'e', 'o', 'y'])
            {
                out.push('\'');
            }
            i += 1;
            continue;
        }
        if let Some((rom, step)) = lookup_kana(&chars[i..]) {
            out.push_str(rom);
            i += step;
            continue;
        }
        match ch {
            '、' | '，' => out.push_str(", "),
            '。' | '．' => out.push_str(". "),
            '「' | '」' | '『' | '』' => out.push('"'),
            '　' => out.push(' '),
            other => out.push(other),
        }
        i += 1;
    }

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn katakana_to_hiragana(c: char) -> char {
    if ('\u{30A1}'..='\u{30F6}').contains(&c) { char::from_u32(c as u32 - 0x60).unwrap_or(c) } else { c }
}

fn lookup_kana(slice: &[char]) -> Option<(&'static str, usize)> {
    if slice.len() >= 2
        && let Some(digraph) = digraph_romaji(slice[0], slice[1])
    {
        return Some((digraph, 2));
    }
    slice.first().and_then(|&c| monograph_romaji(c)).map(|r| (r, 1))
}

fn digraph_romaji(a: char, b: char) -> Option<&'static str> {
    Some(match (a, b) {
        ('き', 'ゃ') => "kya",
        ('き', 'ゅ') => "kyu",
        ('き', 'ょ') => "kyo",
        ('ぎ', 'ゃ') => "gya",
        ('ぎ', 'ゅ') => "gyu",
        ('ぎ', 'ょ') => "gyo",
        ('し', 'ゃ') => "sha",
        ('し', 'ゅ') => "shu",
        ('し', 'ょ') => "sho",
        ('し', 'ぇ') => "she",
        ('じ', 'ゃ') => "ja",
        ('じ', 'ゅ') => "ju",
        ('じ', 'ょ') => "jo",
        ('じ', 'ぇ') => "je",
        ('ち', 'ゃ') => "cha",
        ('ち', 'ゅ') => "chu",
        ('ち', 'ょ') => "cho",
        ('ち', 'ぇ') => "che",
        ('に', 'ゃ') => "nya",
        ('に', 'ゅ') => "nyu",
        ('に', 'ょ') => "nyo",
        ('ひ', 'ゃ') => "hya",
        ('ひ', 'ゅ') => "hyu",
        ('ひ', 'ょ') => "hyo",
        ('び', 'ゃ') => "bya",
        ('び', 'ゅ') => "byu",
        ('び', 'ょ') => "byo",
        ('ぴ', 'ゃ') => "pya",
        ('ぴ', 'ゅ') => "pyu",
        ('ぴ', 'ょ') => "pyo",
        ('み', 'ゃ') => "mya",
        ('み', 'ゅ') => "myu",
        ('み', 'ょ') => "myo",
        ('り', 'ゃ') => "rya",
        ('り', 'ゅ') => "ryu",
        ('り', 'ょ') => "ryo",
        ('ふ', 'ぁ') => "fa",
        ('ふ', 'ぃ') => "fi",
        ('ふ', 'ぇ') => "fe",
        ('ふ', 'ぉ') => "fo",
        ('て', 'ぃ') => "ti",
        ('で', 'ぃ') => "di",
        ('と', 'ぅ') => "tu",
        ('ど', 'ぅ') => "du",
        ('ゔ', 'ぁ') => "va",
        ('ゔ', 'ぃ') => "vi",
        ('ゔ', 'ぇ') => "ve",
        ('ゔ', 'ぉ') => "vo",
        ('う', 'ぃ') => "wi",
        ('う', 'ぇ') => "we",
        ('う', 'ぉ') => "wo",
        _ => return None,
    })
}

fn monograph_romaji(c: char) -> Option<&'static str> {
    Some(match c {
        'あ' | 'ぁ' => "a",
        'い' | 'ぃ' => "i",
        'う' | 'ぅ' => "u",
        'え' | 'ぇ' => "e",
        'お' | 'ぉ' => "o",
        'か' => "ka",
        'き' => "ki",
        'く' => "ku",
        'け' => "ke",
        'こ' => "ko",
        'が' => "ga",
        'ぎ' => "gi",
        'ぐ' => "gu",
        'げ' => "ge",
        'ご' => "go",
        'さ' => "sa",
        'し' => "shi",
        'す' => "su",
        'せ' => "se",
        'そ' => "so",
        'ざ' => "za",
        'じ' => "ji",
        'ず' => "zu",
        'ぜ' => "ze",
        'ぞ' => "zo",
        'た' => "ta",
        'ち' => "chi",
        'つ' => "tsu",
        'て' => "te",
        'と' => "to",
        'だ' => "da",
        'ぢ' => "ji",
        'づ' => "zu",
        'で' => "de",
        'ど' => "do",
        'な' => "na",
        'に' => "ni",
        'ぬ' => "nu",
        'ね' => "ne",
        'の' => "no",
        'は' => "ha",
        'ひ' => "hi",
        'ふ' => "fu",
        'へ' => "he",
        'ほ' => "ho",
        'ば' => "ba",
        'び' => "bi",
        'ぶ' => "bu",
        'べ' => "be",
        'ぼ' => "bo",
        'ぱ' => "pa",
        'ぴ' => "pi",
        'ぷ' => "pu",
        'ぺ' => "pe",
        'ぽ' => "po",
        'ま' => "ma",
        'み' => "mi",
        'む' => "mu",
        'め' => "me",
        'も' => "mo",
        'や' | 'ゃ' => "ya",
        'ゆ' | 'ゅ' => "yu",
        'よ' | 'ょ' => "yo",
        'ら' => "ra",
        'り' => "ri",
        'る' => "ru",
        'れ' => "re",
        'ろ' => "ro",
        'わ' | 'ゎ' => "wa",
        'ゐ' => "wi",
        'ゑ' => "we",
        'を' => "wo",
        'ゔ' => "vu",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_lrc_multi_stamps_offset_and_paired_subtitles() {
        let lrc = "\
[offset:500]
[00:10.00][00:30.00]Chorus line
[00:15.25]無敵の笑顔で荒らすメディア
[00:15.25]Muteki no egao de arasu media
[00:20.00]
";
        let (kind, lines) = parse_lrc(lrc, Some(60_000));
        assert_eq!(kind, LyricsSyncKind::Line);
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].start_ms, 10_500);
        assert_eq!(lines[0].text, "Chorus line");
        assert_eq!(lines[0].words.len(), 2);
        assert_eq!(lines[1].start_ms, 15_750);
        assert_eq!(lines[1].subtitle.as_deref(), Some("Muteki no egao de arasu media"));
        assert_eq!(lines[3].start_ms, 30_500);
        assert_eq!(lines[3].text, "Chorus line");
    }

    #[test]
    fn parses_enhanced_word_synced_lrc() {
        let lrc = "[00:05.00]<00:05.00>Hello <00:05.50>world <00:06.10>today\n[00:08.00]Next";
        let (kind, lines) = parse_lrc(lrc, Some(30_000));
        assert_eq!(kind, LyricsSyncKind::Word);
        assert_eq!(lines[0].text, "Hello world today");
        assert_eq!(lines[0].words.len(), 3);
        assert_eq!(lines[0].words[0].start_ms, 5_000);
        assert_eq!(lines[0].words[0].end_ms, 5_500);
        assert_eq!(lines[0].words[2].start_ms, 6_100);
        assert_eq!(lines[0].words[2].end_ms, 8_000);
    }

    #[test]
    fn converts_kana_to_modified_hepburn_romaji() {
        assert_eq!(kana_to_romaji("むてき の えがお で あらす メディア"), "muteki no egao de arasu media");
        assert_eq!(kana_to_romaji("ちょっと まって コーヒー"), "chotto matte koohii");
        assert_eq!(kana_to_romaji("しんあい"), "shin'ai");
        assert!(contains_japanese("アイドル"));
        assert!(!contains_kanji("アイドル"));
        assert!(contains_kanji("無敵の笑顔"));
    }

    #[test]
    fn normalizes_titles_and_sanitizes_filenames() {
        assert_eq!(normalize_query_title("Instant Crush (feat. Julian Casablancas) - Remastered"), "Instant Crush");
        assert_eq!(normalize_query_title("Idol [Official Video]"), "Idol");
        assert_eq!(sanitize_filename("A/B: C*D?"), "A_B_ C_D_");
    }
}
