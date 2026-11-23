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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Appearance {
    pub theme: ThemeKind,
    pub preset: SpringPreset,
    pub idle_style: IdleStyle,
}
