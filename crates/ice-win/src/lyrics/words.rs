//! Japanese word breaks from Windows' own ICU (`icuuc.dll`, in Windows 10 1703+ and every Windows 11).
//!
//! ICU's word break iterator uses its built-in CJK dictionary for runs of hiragana, katakana and kanji, so it splits
//! kana into words without the Japanese language data the readers in [`super::romaji`] need. It gives word breaks
//! only, never readings: it spaces kana-only Romaji ("ittai kono mama itsu made" instead of "ittaikonomamaitsumade")
//! but cannot read kanji.

use windows::Win32::Globalization::{
    U_ZERO_ERROR, UBRK_WORD, UBreakIterator, UErrorCode, ubrk_close, ubrk_next, ubrk_open,
};
use windows::core::s;

/// `UBRK_DONE` (`ubrk.h`): `ubrk_next` past the last break. Not exported by the `windows` crate.
const UBRK_DONE: i32 = -1;

/// Closes the iterator however the caller leaves.
struct BreakIter(*mut UBreakIterator);

impl Drop for BreakIter {
    fn drop(&mut self) {
        // SAFETY: `self.0` came from a successful `ubrk_open` and is closed exactly once, here.
        unsafe { ubrk_close(self.0) };
    }
}

/// The words of `text` in order, dropping whitespace-only pieces, or `None` if ICU can't run here.
#[must_use]
pub fn japanese_words(text: &str) -> Option<Vec<String>> {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let len = i32::try_from(utf16.len()).ok()?;
    if len == 0 {
        return Some(Vec::new());
    }
    let mut status: UErrorCode = U_ZERO_ERROR;
    // SAFETY: `utf16` outlives the iterator (dropped at the end of this function, before `utf16`); `s!` is a
    // null-terminated locale string; `status` is a valid out pointer.
    let iter = unsafe { ubrk_open(UBRK_WORD, s!("ja"), utf16.as_ptr(), len, &raw mut status) };
    if iter.is_null() || status.0 > U_ZERO_ERROR.0 {
        return None;
    }
    let iter = BreakIter(iter);
    let mut words = Vec::new();
    let mut start = 0usize;
    loop {
        // SAFETY: `iter.0` is open over `utf16`, which is still alive.
        let next = unsafe { ubrk_next(iter.0) };
        if next == UBRK_DONE {
            break;
        }
        let end = usize::try_from(next).ok()?.min(utf16.len());
        if end <= start {
            break;
        }
        let word = String::from_utf16_lossy(&utf16[start..end]);
        if !word.trim().is_empty() {
            words.push(word.trim().to_string());
        }
        start = end;
    }
    Some(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_kana_into_words_without_japanese_language_data() {
        // ICU ships with Windows 11 (D-08), so this runs on every supported PC.
        let words = japanese_words("いったいこのままいつまで").expect("icuuc.dll");
        assert!(words.len() >= 3, "kana split into words: {words:?}");
        assert_eq!(words.concat(), "いったいこのままいつまで", "nothing lost or reordered");
        assert_eq!(japanese_words("").expect("icuuc.dll"), Vec::<String>::new());
        let spaced = japanese_words("ダンボール　を 抱えて").expect("icuuc.dll");
        assert!(spaced.iter().all(|w| !w.trim().is_empty()), "no whitespace words: {spaced:?}");
    }

    /// By hand: `cargo test -p ice-win icu_words_on_this_pc -- --ignored --nocapture`.
    #[test]
    #[ignore = "prints ICU's word breaks for a few lyric lines"]
    fn icu_words_on_this_pc() {
        for line in [
            "いったいこのままいつまで",
            "1人でいるつもりだろう",
            "きみのなまえは",
            "わたしはがっこうへいく",
            "ダンボールをかかえて",
            "ながれるほしをみていた",
            "ずっとそばにいてほしい",
        ] {
            println!("{line} -> {:?}", japanese_words(line));
        }
    }
}
