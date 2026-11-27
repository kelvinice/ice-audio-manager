//! Synced lyrics + estimated word karaoke + offline Japanese Romaji reusing `ice-core` and `ice-win`.

use ice_core::lyrics::{
    LyricsDocument, LyricsSource, LyricsSyncKind, TrackQuery, estimate_word_timings, parse_lrc, track_key,
};
use ice_win::lyrics::lrclib::LrcLibProvider;
use ice_win::lyrics::romaji::JapaneseRomaji;

use crate::models::{PandoraLyricLine, PandoraLyricWord, PandoraLyrics};

pub fn fetch_lyrics(
    title: &str,
    artist: &str,
    album: &str,
    duration_ms: Option<u64>,
    lrc_override: Option<&str>,
) -> PandoraLyrics {
    let key = track_key("pandora", title, artist, album);
    if let Some(raw_lrc) = lrc_override {
        let (sync_kind, mut lines) = parse_lrc(raw_lrc, duration_ms);
        let romaji = JapaneseRomaji::new();
        for line in &mut lines {
            if line.words.is_empty() {
                let end = line.end_ms.unwrap_or(line.start_ms + 3_500);
                line.words = estimate_word_timings(&line.text, line.start_ms, end);
            }
            if line.subtitle.is_none() && line.romaji.is_none() {
                line.romaji = romanize_line(&romaji, &line.text);
            }
        }
        let doc = LyricsDocument {
            track_key: key,
            source: LyricsSource::Local,
            sync_kind,
            instrumental: false,
            lines,
            plain_text: None,
        };
        return doc_to_pandora(doc, Some("Demo LRC"));
    }

    let query = TrackQuery {
        track_key: key.clone(),
        title: title.to_string(),
        artist: artist.to_string(),
        album: album.to_string(),
        duration_ms,
    };
    let provider = LrcLibProvider::new();
    match provider.fetch_cancellable(&query, &|| false) {
        Ok(Some(mut doc)) => {
            if doc.sync_kind != LyricsSyncKind::Plain {
                let romaji = JapaneseRomaji::new();
                for line in &mut doc.lines {
                    if line.words.is_empty() {
                        let end = line.end_ms.unwrap_or(line.start_ms + 3_500);
                        line.words = estimate_word_timings(&line.text, line.start_ms, end);
                    }
                    if line.subtitle.is_none() && line.romaji.is_none() {
                        line.romaji = romanize_line(&romaji, &line.text);
                    }
                }
            }
            doc_to_pandora(doc, None)
        }
        Ok(None) => PandoraLyrics {
            track_key: key,
            status: "not_found".into(),
            source: None,
            lines: Vec::new(),
            plain_text: None,
            error: None,
        },
        Err(err) => PandoraLyrics {
            track_key: key,
            status: "error".into(),
            source: None,
            lines: Vec::new(),
            plain_text: None,
            error: Some(err),
        },
    }
}

fn doc_to_pandora(doc: ice_core::lyrics::LyricsDocument, source_override: Option<&str>) -> PandoraLyrics {
    let status = if doc.instrumental {
        "instrumental"
    } else if !doc.lines.is_empty() {
        "synced"
    } else if doc.plain_text.is_some() {
        "plain"
    } else {
        "not_found"
    };
    let source = source_override.map(str::to_string).unwrap_or_else(|| match doc.source {
        LyricsSource::Local => "Local .lrc".into(),
        LyricsSource::Cache => "Cache".into(),
        LyricsSource::Lrclib => "LRCLIB".into(),
    });
    let lines = doc
        .lines
        .into_iter()
        .map(|l| PandoraLyricLine {
            time_ms: l.start_ms,
            end_ms: l.end_ms,
            text: l.text,
            subtitle: l.subtitle,
            romaji: l.romaji,
            words: l
                .words
                .into_iter()
                .map(|w| PandoraLyricWord {
                    start_ms: w.start_ms,
                    end_ms: w.end_ms,
                    text: w.text,
                })
                .collect(),
        })
        .collect();
    PandoraLyrics {
        track_key: doc.track_key,
        status: status.into(),
        source: Some(source),
        lines,
        plain_text: doc.plain_text,
        error: None,
    }
}

fn romanize_line(jp: &JapaneseRomaji, text: &str) -> Option<String> {
    if let Some(r) = jp.line_to_romaji(text) {
        return Some(r);
    }
    romanize_korean_or_cyrillic(text)
}

/// Offline transliteration for Korean Hangul (Revised Romanization) and Cyrillic scripts.
pub fn romanize_korean_or_cyrillic(text: &str) -> Option<String> {
    const INITIALS: [&str; 19] = [
        "g", "kk", "n", "d", "tt", "r", "m", "b", "pp", "s", "ss", "", "j", "jj", "ch", "k", "t", "p", "h",
    ];
    const MEDIALS: [&str; 21] = [
        "a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo", "u", "wo", "we", "wi",
        "yu", "eu", "ui", "i",
    ];
    const FINALS: [&str; 28] = [
        "", "k", "k", "k", "n", "n", "n", "t", "l", "l", "l", "l", "l", "l", "l", "l", "m", "p", "p", "t",
        "t", "ng", "t", "t", "k", "t", "p", "t",
    ];

    let mut out = String::with_capacity(text.len() * 2);
    let mut converted = false;

    for ch in text.chars() {
        let cp = ch as u32;
        if (0xAC00..=0xD7A3).contains(&cp) {
            let idx = (cp - 0xAC00) as usize;
            let ini = idx / (21 * 28);
            let med = (idx % (21 * 28)) / 28;
            let fin = idx % 28;
            out.push_str(INITIALS[ini]);
            out.push_str(MEDIALS[med]);
            out.push_str(FINALS[fin]);
            converted = true;
        } else if let Some(lat) = cyrillic_to_latin(ch) {
            out.push_str(lat);
            converted = true;
        } else {
            out.push(ch);
        }
    }

    if converted { Some(out) } else { None }
}

fn cyrillic_to_latin(ch: char) -> Option<&'static str> {
    Some(match ch {
        'А' => "A",
        'а' => "a",
        'Б' => "B",
        'б' => "b",
        'В' => "V",
        'в' => "v",
        'Г' => "G",
        'г' => "g",
        'Ґ' => "G",
        'ґ' => "g",
        'Д' => "D",
        'д' => "d",
        'Е' => "E",
        'е' => "e",
        'Ё' => "Yo",
        'ё' => "yo",
        'Є' => "Ye",
        'є' => "ye",
        'Ж' => "Zh",
        'ж' => "zh",
        'З' => "Z",
        'з' => "z",
        'И' => "I",
        'и' => "i",
        'І' => "I",
        'і' => "i",
        'Ї' => "Yi",
        'ї' => "yi",
        'Й' => "Y",
        'й' => "y",
        'К' => "K",
        'к' => "k",
        'Л' => "L",
        'л' => "l",
        'М' => "M",
        'м' => "m",
        'Н' => "N",
        'н' => "n",
        'О' => "O",
        'о' => "o",
        'П' => "P",
        'п' => "p",
        'Р' => "R",
        'р' => "r",
        'С' => "S",
        'с' => "s",
        'Т' => "T",
        'т' => "t",
        'У' => "U",
        'у' => "u",
        'Ф' => "F",
        'ф' => "f",
        'Х' => "Kh",
        'х' => "kh",
        'Ц' => "Ts",
        'ц' => "ts",
        'Ч' => "Ch",
        'ч' => "ch",
        'Ш' => "Sh",
        'ш' => "sh",
        'Щ' => "Shch",
        'щ' => "shch",
        'Ъ' | 'ъ' | 'Ь' | 'ь' => "",
        'Ы' => "Y",
        'ы' => "y",
        'Э' => "E",
        'э' => "e",
        'Ю' => "Yu",
        'ю' => "yu",
        'Я' => "Ya",
        'я' => "ya",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn romanizes_korean_and_cyrillic_leaving_latin_none() {
        assert_eq!(romanize_korean_or_cyrillic("사랑해").as_deref(), Some("saranghae"));
        assert_eq!(romanize_korean_or_cyrillic("Привет").as_deref(), Some("Privet"));
        assert_eq!(romanize_korean_or_cyrillic("Hello world"), None);
    }
}
