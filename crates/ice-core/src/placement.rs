//! D-20 notch placement: where the notch sits on its monitor. One rule for both hosts (the WebView notch window and the
//! native pill), replacing the two copies of the top-centre maths they had.
//!
//! Each anchor fixes the notch's **centre**. Both hosts centre their own box on it: the WebView window is the full
//! [`NOTCH_CANVAS_W`] canvas (the expanded card included), the native pill is small. So the side anchors reserve the
//! canvas width from the screen edge, and the widest card never runs off screen.
//!
//! Only the top edge for now. Bottom anchors (Floating only, D-20 A2) need the notch to grow upwards and come later.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Logical size of the WebView notch canvas. Twin: `CANVAS_W` / `CANVAS_H` in `app/ui/src/lib/notch/geometry.ts`.
pub const NOTCH_CANVAS_W: i32 = 648;
/// Logical height of the WebView notch canvas. Twin: `CANVAS_H` in `app/ui/src/lib/notch/geometry.ts`.
pub const NOTCH_CANVAS_H: i32 = 292;

/// Where on its monitor's top edge the notch sits (D-20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum NotchAnchor {
    TopLeft,
    #[default]
    TopCenter,
    TopRight,
}

impl NotchAnchor {
    /// Every anchor, in the order the tray and Settings list them.
    pub const ALL: [Self; 3] = [Self::TopLeft, Self::TopCenter, Self::TopRight];

    /// Short label for menus.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TopLeft => "Top left",
            Self::TopCenter => "Top centre",
            Self::TopRight => "Top right",
        }
    }
}

/// A rectangle in physical pixels (a monitor's work area: the screen minus the taskbar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

/// The notch's centre x for `anchor` in `area`, keeping `reserve_w` (physical px, the widest the notch gets) inside it.
/// An area narrower than `reserve_w` centres the notch whatever the anchor.
#[must_use]
pub const fn anchor_center_x(anchor: NotchAnchor, area: Area, reserve_w: i32) -> i32 {
    let centre = area.left + area.width / 2;
    if area.width <= reserve_w {
        return centre;
    }
    match anchor {
        NotchAnchor::TopLeft => area.left + reserve_w / 2,
        NotchAnchor::TopCenter => centre,
        NotchAnchor::TopRight => area.left + area.width - reserve_w / 2,
    }
}

/// Top-left corner (physical px) of a `box_w`-wide box centred on the anchor, `top_offset` below the area's top edge.
#[must_use]
pub const fn anchor_origin(anchor: NotchAnchor, area: Area, reserve_w: i32, box_w: i32, top_offset: i32) -> (i32, i32) {
    (anchor_center_x(anchor, area, reserve_w) - box_w / 2, area.top + top_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_HD: Area = Area { left: 0, top: 0, width: 1920, height: 1040 };

    #[test]
    fn centre_is_the_old_top_centre() {
        // Bottom taskbar (the usual case): the work area starts at the top, so nothing moves for existing users.
        assert_eq!(anchor_origin(NotchAnchor::TopCenter, FULL_HD, 648, 648, 0), (636, 0));
        assert_eq!(anchor_origin(NotchAnchor::TopCenter, FULL_HD, 648, 116, 6), (902, 6));
    }

    #[test]
    fn side_anchors_keep_the_whole_canvas_on_screen_and_both_hosts_share_the_centre() {
        let canvas = anchor_origin(NotchAnchor::TopLeft, FULL_HD, 648, 648, 0);
        assert_eq!(canvas, (0, 0), "the canvas touches the left edge, never past it");
        let pill = anchor_origin(NotchAnchor::TopLeft, FULL_HD, 648, 116, 0);
        assert_eq!(pill.0 + 116 / 2, canvas.0 + 648 / 2, "native pill and WebView notch share one centre");
        let right = anchor_origin(NotchAnchor::TopRight, FULL_HD, 648, 648, 0);
        assert_eq!(right.0 + 648, 1920, "flush with the right edge");
    }

    #[test]
    fn taskbars_and_monitors_are_respected() {
        // Taskbar at the top: the notch sits below it instead of covering it.
        let top_bar = Area { left: 0, top: 48, width: 1920, height: 1032 };
        assert_eq!(anchor_origin(NotchAnchor::TopCenter, top_bar, 648, 648, 0).1, 48);
        // Taskbar on the left: the left anchor starts after it.
        let left_bar = Area { left: 62, top: 0, width: 1858, height: 1080 };
        assert_eq!(anchor_origin(NotchAnchor::TopLeft, left_bar, 648, 648, 0).0, 62);
        // A second monitor to the right, at 150 % (the canvas reserve scales with it).
        let second = Area { left: 1920, top: 0, width: 2560, height: 1392 };
        let reserve = 972;
        assert_eq!(anchor_origin(NotchAnchor::TopRight, second, reserve, reserve, 0).0 + reserve, 1920 + 2560);
        assert_eq!(anchor_center_x(NotchAnchor::TopCenter, second, reserve), 1920 + 1280);
    }

    #[test]
    fn a_narrow_screen_centres_the_notch() {
        let narrow = Area { left: 0, top: 0, width: 600, height: 800 };
        for anchor in NotchAnchor::ALL {
            assert_eq!(anchor_center_x(anchor, narrow, 648), 300);
        }
    }
}
