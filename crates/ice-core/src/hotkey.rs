//! Hotkey chords: parsing and display, independent of Windows.
//!
//! Hotkeys are registered with `RegisterHotKey` (never a keyboard hook, code-practices §1). Modified
//! volume keys are allowed, so a keyboard knob can drive gestures (F31) while plain turns stay Windows' own.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Modifier bits, matching Win32 `MOD_*` values so `ice-win` can pass them straight through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Mods(pub u32);

impl Mods {
    pub const ALT: u32 = 0x1;
    pub const CTRL: u32 = 0x2;
    pub const SHIFT: u32 = 0x4;
    pub const WIN: u32 = 0x8;

    #[must_use]
    pub fn has(self, bit: u32) -> bool {
        self.0 & bit != 0
    }
}

/// A modifier set plus one virtual key (Win32 `VK_*` code).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Chord {
    pub mods: Mods,
    pub vk: u32,
}

const NAMED: &[(&str, u32)] = &[
    ("VolumeMute", 0xAD),
    ("VolumeDown", 0xAE),
    ("VolumeUp", 0xAF),
    ("MediaNext", 0xB0),
    ("MediaPrev", 0xB1),
    ("MediaStop", 0xB2),
    ("MediaPlayPause", 0xB3),
    ("Space", 0x20),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("End", 0x23),
    ("Home", 0x24),
    ("Left", 0x25),
    ("Up", 0x26),
    ("Right", 0x27),
    ("Down", 0x28),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("Pause", 0x13),
    ("ScrollLock", 0x91),
];

fn key_from_name(name: &str) -> Option<u32> {
    if let Some((_, vk)) = NAMED.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
        return Some(*vk);
    }
    let up = name.to_ascii_uppercase();
    let mut chars = up.chars();
    match (chars.next(), chars.next()) {
        (Some(c @ ('A'..='Z' | '0'..='9')), None) => Some(c as u32),
        (Some('F'), Some(_)) => match up[1..].parse::<u32>() {
            Ok(n @ 1..=24) => Some(0x6F + n), // F1 = 0x70
            _ => None,
        },
        _ => None,
    }
}

fn key_name(vk: u32) -> String {
    if let Some((n, _)) = NAMED.iter().find(|(_, v)| *v == vk) {
        return (*n).to_string();
    }
    match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(vk).map_or_else(|| format!("0x{vk:02X}"), |c| c.to_string()),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        _ => format!("0x{vk:02X}"),
    }
}

/// Why a chord string was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChordError {
    Empty,
    UnknownKey(String),
    MissingKey,
    TwoKeys,
    /// Plain volume/media keys must stay Windows' own (D-07 A): they need at least one modifier.
    NeedsModifier(String),
}

impl fmt::Display for ChordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty hotkey"),
            Self::UnknownKey(k) => write!(f, "unknown key `{k}`"),
            Self::MissingKey => write!(f, "hotkey has modifiers but no key"),
            Self::TwoKeys => write!(f, "hotkey can only have one non-modifier key"),
            Self::NeedsModifier(k) => write!(f, "`{k}` needs a modifier (plain media keys stay with Windows)"),
        }
    }
}

impl std::error::Error for ChordError {}

impl std::str::FromStr for Chord {
    type Err = ChordError;

    /// Parses `"Ctrl+Alt+F12"`, `"ctrl + volumeup"`, `"Win+Shift+A"`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Err(ChordError::Empty);
        }
        let mut mods = 0;
        let mut vk = None;
        for part in s.split('+').map(str::trim) {
            let bit = match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => Some(Mods::CTRL),
                "alt" => Some(Mods::ALT),
                "shift" => Some(Mods::SHIFT),
                "win" | "super" | "meta" => Some(Mods::WIN),
                _ => None,
            };
            match bit {
                Some(b) => mods |= b,
                None if vk.is_some() => return Err(ChordError::TwoKeys),
                None => vk = Some(key_from_name(part).ok_or_else(|| ChordError::UnknownKey(part.to_string()))?),
            }
        }
        let vk = vk.ok_or(ChordError::MissingKey)?;
        if mods == 0 && (0xAD..=0xB3).contains(&vk) {
            return Err(ChordError::NeedsModifier(key_name(vk)));
        }
        Ok(Self { mods: Mods(mods), vk })
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (bit, name) in [(Mods::CTRL, "Ctrl"), (Mods::ALT, "Alt"), (Mods::SHIFT, "Shift"), (Mods::WIN, "Win")] {
            if self.mods.has(bit) {
                write!(f, "{name}+")?;
            }
        }
        write!(f, "{}", key_name(self.vk))
    }
}

impl TryFrom<String> for Chord {
    type Error = ChordError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Chord> for String {
    fn from(c: Chord) -> Self {
        c.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes() {
        let c: Chord = "ctrl + alt + f12".parse().unwrap();
        assert_eq!(c, Chord { mods: Mods(Mods::CTRL | Mods::ALT), vk: 0x7B });
        assert_eq!(c.to_string(), "Ctrl+Alt+F12");
        assert_eq!("Shift+Ctrl+VolumeUp".parse::<Chord>().unwrap().to_string(), "Ctrl+Shift+VolumeUp");
        assert_eq!("Win+a".parse::<Chord>().unwrap().to_string(), "Win+A");
        assert_eq!("F9".parse::<Chord>().unwrap().vk, 0x78);
    }

    #[test]
    fn rejects_bad_chords() {
        assert_eq!("".parse::<Chord>(), Err(ChordError::Empty));
        assert_eq!("Ctrl+Alt".parse::<Chord>(), Err(ChordError::MissingKey));
        assert_eq!("Ctrl+A+B".parse::<Chord>(), Err(ChordError::TwoKeys));
        assert_eq!("Ctrl+Banana".parse::<Chord>(), Err(ChordError::UnknownKey("Banana".into())));
        assert_eq!("F25".parse::<Chord>(), Err(ChordError::UnknownKey("F25".into())));
        // Plain knob/volume keys stay Windows' own.
        assert_eq!("VolumeUp".parse::<Chord>(), Err(ChordError::NeedsModifier("VolumeUp".into())));
        assert!("Ctrl+VolumeUp".parse::<Chord>().is_ok());
    }
}
