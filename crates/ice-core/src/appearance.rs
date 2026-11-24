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

/// How the notch shows a new track (owner, 2026-11-24). Only the WebView notch has the large now-playing pill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum NowPlayingPill {
    /// Pops up on a new track (and on play/pause while it shows), then shrinks back to the idle pill.
    #[default]
    Brief,
    /// Stays as the now-playing pill for as long as something is playing; pausing shrinks it after the brief time.
    WhilePlaying,
    /// No pop-up: the idle pill just shows the new title.
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Appearance {
    pub theme: ThemeKind,
    pub preset: SpringPreset,
    pub idle_style: IdleStyle,
    pub now_playing: NowPlayingPill,
}
