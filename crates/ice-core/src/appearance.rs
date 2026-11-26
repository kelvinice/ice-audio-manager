//! Saved notch preferences shared by the tray, WebView and native idle indicator.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum SpringPreset {
    #[default]
    Bouncy,
    Smooth,
    Snappy,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    #[default]
    Ice,
    IceLight,
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

/// Visual attachment style of the notch overlay (floating capsule vs flush screen-edge nook).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum NotchStyle {
    /// Floating capsule below the top screen bezel (original Ice pill).
    Floating,
    /// Docked flush to the top screen edge with concave ears and a side-by-side expanded card.
    #[default]
    Nook,
}

/// How the notch shows a new track (owner, 2026-11-24). Only the WebView notch has the large now-playing pill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum NowPlayingPill {
    /// Pops up on a new track (and on play/pause while it shows), then shrinks back to the idle pill.
    Brief,
    /// Stays as the now-playing pill for as long as something is playing; pausing shrinks it after the brief time.
    #[default]
    WhilePlaying,
    /// No pop-up: the idle pill just shows the new title.
    Off,
}

/// What the notch looks like while hibernating (D-27, owner 2026-11-25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum HibernateLook {
    /// A plain black notch (a hardware-notch look) with only a red dot while the mic is muted.
    #[default]
    Plain,
    /// No notch at all until hibernate is turned off (tray or Settings).
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Appearance {
    pub theme: ThemeKind,
    pub preset: SpringPreset,
    pub idle_style: IdleStyle,
    pub notch_style: NotchStyle,
    pub now_playing: NowPlayingPill,
    /// D-25: slow ambient drift of the art wash on the expanded card while playing.
    pub moving_backdrop: bool,
    /// D-27 hibernate: the notch shows no track, art, lyrics or pop-ups and is click-through, for meetings and screen
    /// sharing. Session state: every start begins awake (`ConfigStore::load`), and a hot reload keeps the live value.
    pub hibernate: bool,
    /// D-27: what the notch looks like while hibernating (plain notch, or nothing at all).
    pub hibernate_look: HibernateLook,
    /// D-27: while hibernating, leave the notch windows out of screen captures (`WDA_EXCLUDEFROMCAPTURE`).
    pub hibernate_hide_from_capture: bool,
    /// Owner, 2026-11-26: extra drag & drop effects on the WebView notch (jelly landing in Settings → Notch; the rest
    /// Experimental). All off by default.
    pub drag_effects: DragEffects,
    /// Owner, 2026-11-26 (Settings → Appearance): the played part of the card's seek bar is a slowly travelling wave
    /// while music plays (flat when paused). Off by default.
    pub wavy_progress: bool,
}

/// Drag & drop effects (WebView notch only; the native pill keeps the plain lift and rim flash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct DragEffects {
    /// Experimental: accent rings rise off the notch's edge and spread out where it lands.
    pub landing_ripple: bool,
    /// A softer, wobblier landing (two rebounds instead of one).
    pub jelly_landing: bool,
    /// Experimental: the carried pill glows in the accent colour.
    pub carry_glow: bool,
}

impl Appearance {
    /// Whether the notch windows should be left out of screen captures right now (D-27).
    #[must_use]
    pub const fn exclude_from_capture(&self) -> bool {
        self.hibernate && self.hibernate_hide_from_capture
    }

    /// The idle style the notch uses right now: [`IdleStyle::Hidden`] while hibernating with
    /// [`HibernateLook::Hidden`], so the WebView and the native pill hide through their usual idle-hidden path.
    #[must_use]
    pub const fn effective_idle_style(&self) -> IdleStyle {
        if self.hibernate && matches!(self.hibernate_look, HibernateLook::Hidden) {
            IdleStyle::Hidden
        } else {
            self.idle_style
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: ThemeKind::default(),
            preset: SpringPreset::default(),
            idle_style: IdleStyle::default(),
            notch_style: NotchStyle::default(),
            now_playing: NowPlayingPill::default(),
            moving_backdrop: true,
            hibernate: false,
            hibernate_look: HibernateLook::default(),
            hibernate_hide_from_capture: true,
            drag_effects: DragEffects::default(),
            wavy_progress: false,
        }
    }
}
