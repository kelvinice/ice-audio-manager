//! Saved notch preferences shared by the tray, WebView and native idle indicator.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum SpringPreset {
    Snappy,
    #[default]
    Smooth,
    Bouncy,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    #[default]
    Ice,
    Midnight,
    Fluent,
    Minimal,
}

/// Settled Q-10 idle appearance options (`MiniPill` default, `Dot`, or `Hidden`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum IdleStyle {
    Dot,
    #[default]
    MiniPill,
    Hidden,
}

impl IdleStyle {
    /// Parses `"mini_pill"`, `"dot"`, or `"hidden"` (case-insensitive).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "mini_pill" | "minipill" | "pill" => Some(Self::MiniPill),
            "dot" => Some(Self::Dot),
            "hidden" | "hide" => Some(Self::Hidden),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Appearance {
    pub theme: ThemeKind,
    pub preset: SpringPreset,
    pub idle_style: IdleStyle,
}
