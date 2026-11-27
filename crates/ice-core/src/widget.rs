//! Experimental notch widget (owner, 2026-11-27): an ordered, user-chosen set of quick buttons that replaces the fixed
//! buttons in the card's side rail (Floating) and the Edge Nook toolbar capsule. Off by default; while off, the notch
//! keeps its built-in buttons.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The most buttons the widget holds: five 24 px buttons fill the Floating card's 140 px side rail.
pub const MAX_BUTTONS: usize = 5;

/// One quick button the widget can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum WidgetButton {
    /// Keep the card open (`toggle_expand`).
    Pin,
    /// Mute or unmute the default microphone.
    Mic,
    /// Show or hide the lyrics drawer (shown only while the song has lyrics on the card).
    Lyrics,
    /// Hibernate the notch (D-27).
    Hibernate,
    /// Turn call mode (D-30) on or off.
    CallMode,
    /// Ask the Pandora companion app to recognise the song playing on this PC (launches Pandora if needed).
    PandoraRecognize,
    /// Show the Pandora companion app (launches it if needed).
    PandoraOpen,
    /// Open Ice's Settings window.
    Settings,
}

impl WidgetButton {
    /// Every button, in the order Settings offers them.
    pub const ALL: [Self; 8] = [
        Self::Pin,
        Self::Mic,
        Self::Lyrics,
        Self::Hibernate,
        Self::CallMode,
        Self::PandoraRecognize,
        Self::PandoraOpen,
        Self::Settings,
    ];
}

/// The widget's switch and its button order (`[widget]` in `config.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct NotchWidget {
    /// Experimental: show `buttons` instead of the built-in rail / toolbar buttons.
    pub enabled: bool,
    /// Left-to-right (toolbar) or top-to-bottom (rail) order; at most [`MAX_BUTTONS`], no repeats.
    pub buttons: Vec<WidgetButton>,
}

impl Default for NotchWidget {
    fn default() -> Self {
        Self {
            enabled: false,
            buttons: vec![WidgetButton::Pin, WidgetButton::Mic, WidgetButton::PandoraRecognize, WidgetButton::Settings],
        }
    }
}

/// Checks a button list a user (or a hand-edited file) asks for.
///
/// # Errors
/// More than [`MAX_BUTTONS`] buttons, or the same button twice.
pub fn validate(buttons: &[WidgetButton]) -> Result<(), String> {
    if buttons.len() > MAX_BUTTONS {
        return Err(format!("The widget holds at most {MAX_BUTTONS} buttons."));
    }
    for (i, b) in buttons.iter().enumerate() {
        if buttons[..i].contains(b) {
            return Err("Each widget button can appear only once.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_off_and_valid() {
        let w = NotchWidget::default();
        assert!(!w.enabled);
        assert_eq!(validate(&w.buttons), Ok(()));
    }

    #[test]
    fn rejects_too_many_and_repeats() {
        assert!(validate(&WidgetButton::ALL[..MAX_BUTTONS + 1]).is_err());
        assert!(validate(&[WidgetButton::Mic, WidgetButton::Pin, WidgetButton::Mic]).is_err());
        assert_eq!(validate(&[]), Ok(()));
    }

    #[test]
    fn buttons_serialise_snake_case() {
        let json = serde_json::to_string(&WidgetButton::PandoraRecognize).unwrap();
        assert_eq!(json, "\"pandora_recognize\"");
    }
}
