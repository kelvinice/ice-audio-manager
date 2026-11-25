//! D-20 notch placement: where the notch sits on its monitor, and the drag-and-snap maths. One rule for both hosts (the
//! WebView notch window and the native pill).
//!
//! Each anchor fixes the notch's **centre** within the monitor's work area (the screen minus the taskbar). Both hosts
//! centre their own box on it: the WebView window is the full [`NOTCH_CANVAS_W`] canvas (the expanded card included),
//! the native pill is small. So the side anchors reserve the canvas width from the screen edge, and the widest card
//! never runs off screen. Top anchors measure down from the work area's top; bottom anchors up from its bottom (above
//! the taskbar), where the notch grows upwards.
//!
//! Edge Nook is flush with the top edge by design, so it only has top anchors: [`NotchAnchor::effective_for`] maps a
//! bottom anchor to its top counterpart while Edge Nook is the style.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::appearance::NotchStyle;

/// Logical size of the WebView notch canvas. Twin: `CANVAS_W` / `CANVAS_H` in `app/ui/src/lib/notch/geometry.ts`.
pub const NOTCH_CANVAS_W: i32 = 648;
/// Logical height of the WebView notch canvas. Twin: `CANVAS_H` in `app/ui/src/lib/notch/geometry.ts`.
pub const NOTCH_CANVAS_H: i32 = 292;
/// At a bottom anchor the pill sits `BOTTOM_PAD` (24) inside the canvas so its drop shadow has room; the canvas then
/// overhangs the work area by this much, which leaves the pill the same 6 px from the taskbar as from the top bezel.
/// Twin: `BOTTOM_PAD - TOP_PAD` in `geometry.ts`.
pub const CANVAS_BOTTOM_OVERHANG: i32 = 18;
/// The idle mini pill (logical), used for the drag slots. Twin: `idleGeometry` mini pill in `geometry.ts`.
pub const SLOT_W: i32 = 116;
pub const SLOT_H: i32 = 28;

/// Where on its monitor the notch sits (D-20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum NotchAnchor {
    TopLeft,
    #[default]
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// How the notch is moved: dragged and snapped to an anchor (default), or only from the tray and Settings (D-20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum NotchPlacement {
    #[default]
    DragSnap,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Left,
    Centre,
    Right,
}

impl NotchAnchor {
    /// Every anchor, in the order the tray and Settings list them.
    pub const ALL: [Self; 6] =
        [Self::TopLeft, Self::TopCenter, Self::TopRight, Self::BottomLeft, Self::BottomCenter, Self::BottomRight];

    /// Short label for menus.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TopLeft => "Top left",
            Self::TopCenter => "Top centre",
            Self::TopRight => "Top right",
            Self::BottomLeft => "Bottom left",
            Self::BottomCenter => "Bottom centre",
            Self::BottomRight => "Bottom right",
        }
    }

    /// Whether the notch sits on the bottom edge (above the taskbar) and grows upwards.
    #[must_use]
    pub const fn is_bottom(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomCenter | Self::BottomRight)
    }

    /// Whether `style` can use this anchor: Edge Nook is top-only.
    #[must_use]
    pub const fn allowed_for(self, style: NotchStyle) -> bool {
        !(self.is_bottom() && matches!(style, NotchStyle::Nook))
    }

    /// The anchor actually used with `style`: a bottom anchor becomes its top counterpart under Edge Nook, so switching
    /// styles never puts a flush notch on the wrong edge, and switching back restores the saved bottom anchor.
    #[must_use]
    pub const fn effective_for(self, style: NotchStyle) -> Self {
        if self.allowed_for(style) {
            return self;
        }
        match self.column() {
            Column::Left => Self::TopLeft,
            Column::Centre => Self::TopCenter,
            Column::Right => Self::TopRight,
        }
    }

    const fn column(self) -> Column {
        match self {
            Self::TopLeft | Self::BottomLeft => Column::Left,
            Self::TopCenter | Self::BottomCenter => Column::Centre,
            Self::TopRight | Self::BottomRight => Column::Right,
        }
    }
}

/// How far (logical px) the WebView canvas sits from its anchored edge: flush at the top, overhanging at the bottom.
#[must_use]
pub const fn canvas_edge_offset(anchor: NotchAnchor) -> i32 {
    if anchor.is_bottom() { -CANVAS_BOTTOM_OVERHANG } else { 0 }
}

/// How far (logical px) the idle pill sits from its anchored edge: flush for Edge Nook, a small gap for Floating.
/// Twin: `frame_metrics` top offset in `crates/ice-win/src/native_dot.rs` and `TOP_PAD` in `geometry.ts`.
#[must_use]
pub const fn pill_edge_offset(style: NotchStyle) -> i32 {
    match style {
        NotchStyle::Nook => 0,
        NotchStyle::Floating => 6,
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
    match anchor.column() {
        Column::Left => area.left + reserve_w / 2,
        Column::Centre => centre,
        Column::Right => area.left + area.width - reserve_w / 2,
    }
}

/// Top-left corner (physical px) of a `box_w` × `box_h` box centred on the anchor, `edge_offset` in from the anchored
/// edge (the work area's top, or its bottom for a bottom anchor; negative = past the edge).
#[must_use]
pub const fn anchor_origin(
    anchor: NotchAnchor,
    area: Area,
    reserve_w: i32,
    box_w: i32,
    box_h: i32,
    edge_offset: i32,
) -> (i32, i32) {
    let x = anchor_center_x(anchor, area, reserve_w) - box_w / 2;
    let y = if anchor.is_bottom() { area.top + area.height - edge_offset - box_h } else { area.top + edge_offset };
    (x, y)
}

/// One monitor the notch can be dropped on: its work area and its scale (1.0 = 100 %).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropArea {
    pub area: Area,
    pub scale: f64,
}

/// Where a dragged notch can land: a monitor (index into the [`DropArea`] list) and an anchor, with the centre and
/// size of the idle pill there (physical px), which is what the drag shows as a ghost slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropSlot {
    pub monitor: usize,
    pub anchor: NotchAnchor,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl DropSlot {
    #[must_use]
    pub const fn centre(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

/// Every slot on every monitor for `style` (Edge Nook: the top three; Floating: all six).
#[must_use]
pub fn drop_slots(monitors: &[DropArea], style: NotchStyle) -> Vec<DropSlot> {
    let px = |logical: i32, scale: f64| (f64::from(logical) * scale).round() as i32;
    let mut slots = Vec::new();
    for (monitor, m) in monitors.iter().enumerate() {
        for anchor in NotchAnchor::ALL.into_iter().filter(|a| a.allowed_for(style)) {
            let (w, h) = (px(SLOT_W, m.scale), px(SLOT_H, m.scale));
            let (x, y) =
                anchor_origin(anchor, m.area, px(NOTCH_CANVAS_W, m.scale), w, h, px(pill_edge_offset(style), m.scale));
            slots.push(DropSlot { monitor, anchor, x, y, w, h });
        }
    }
    slots
}

/// The slot closest to `point` (physical px, the cursor at release). `None` only for an empty list.
#[must_use]
pub fn nearest_slot(slots: &[DropSlot], point: (i32, i32)) -> Option<DropSlot> {
    let dist = |s: &DropSlot| {
        let (cx, cy) = s.centre();
        let (dx, dy) = (i64::from(cx - point.0), i64::from(cy - point.1));
        dx * dx + dy * dy
    };
    slots.iter().copied().min_by_key(dist)
}

/// Window positions for the release animation from `from` to `to`: a critically damped spring sampled every
/// `frame_ms`, ending exactly on `to`. Empty when they're the same point; just `[to]` with reduce motion.
#[must_use]
pub fn snap_path(from: (i32, i32), to: (i32, i32), frame_ms: u32, reduce_motion: bool) -> Vec<(i32, i32)> {
    if from == to {
        return Vec::new();
    }
    if reduce_motion || frame_ms == 0 {
        return vec![to];
    }
    // ω = 22 rad/s settles within ~0.3 s without overshoot (critically damped: x(t) = (1 + ωt)·e^(−ωt)).
    const OMEGA: f64 = 22.0;
    let dt = f64::from(frame_ms) / 1000.0;
    let mut path = Vec::new();
    for i in 1.. {
        let t = dt * f64::from(i);
        let remaining = (1.0 + OMEGA * t) * (-OMEGA * t).exp();
        let lerp = |a: i32, b: i32| (f64::from(b) - f64::from(b - a) * remaining).round() as i32;
        let p = (lerp(from.0, to.0), lerp(from.1, to.1));
        if remaining < 0.01 || p == to {
            break;
        }
        if path.last() != Some(&p) {
            path.push(p);
        }
    }
    path.push(to);
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_HD: Area = Area { left: 0, top: 0, width: 1920, height: 1040 };

    #[test]
    fn centre_is_the_old_top_centre() {
        // Bottom taskbar (the usual case): the work area starts at the top, so nothing moves for existing users.
        assert_eq!(anchor_origin(NotchAnchor::TopCenter, FULL_HD, 648, 648, 292, 0), (636, 0));
        assert_eq!(anchor_origin(NotchAnchor::TopCenter, FULL_HD, 648, 116, 28, 6), (902, 6));
    }

    #[test]
    fn side_anchors_keep_the_whole_canvas_on_screen_and_both_hosts_share_the_centre() {
        let canvas = anchor_origin(NotchAnchor::TopLeft, FULL_HD, 648, 648, 292, 0);
        assert_eq!(canvas, (0, 0), "the canvas touches the left edge, never past it");
        let pill = anchor_origin(NotchAnchor::TopLeft, FULL_HD, 648, 116, 28, 0);
        assert_eq!(pill.0 + 116 / 2, canvas.0 + 648 / 2, "native pill and WebView notch share one centre");
        let right = anchor_origin(NotchAnchor::TopRight, FULL_HD, 648, 648, 292, 0);
        assert_eq!(right.0 + 648, 1920, "flush with the right edge");
    }

    #[test]
    fn bottom_anchors_sit_above_the_taskbar_and_both_hosts_line_up() {
        // Work area 1040 tall: the taskbar takes the bottom 40 px of 1080.
        let pill = anchor_origin(NotchAnchor::BottomCenter, FULL_HD, 648, 116, 28, 6);
        assert_eq!(pill, (902, 1040 - 6 - 28), "6 px above the taskbar, like 6 px below the top bezel");
        let canvas = anchor_origin(
            NotchAnchor::BottomCenter,
            FULL_HD,
            648,
            648,
            292,
            canvas_edge_offset(NotchAnchor::BottomCenter),
        );
        assert_eq!(
            canvas.1 + 292,
            1040 + CANVAS_BOTTOM_OVERHANG,
            "the canvas overhangs the taskbar by the shadow room"
        );
        // The WebView pill's bottom (canvas bottom − BOTTOM_PAD 24) meets the native pill's bottom.
        assert_eq!(canvas.1 + 292 - 24, pill.1 + 28);
        assert_eq!(anchor_origin(NotchAnchor::BottomRight, FULL_HD, 648, 648, 292, 0).0 + 648, 1920);
    }

    #[test]
    fn edge_nook_is_top_only() {
        assert_eq!(NotchAnchor::BottomLeft.effective_for(NotchStyle::Nook), NotchAnchor::TopLeft);
        assert_eq!(NotchAnchor::BottomCenter.effective_for(NotchStyle::Nook), NotchAnchor::TopCenter);
        assert_eq!(NotchAnchor::BottomRight.effective_for(NotchStyle::Floating), NotchAnchor::BottomRight);
        assert_eq!(NotchAnchor::TopRight.effective_for(NotchStyle::Nook), NotchAnchor::TopRight);
        assert_eq!(drop_slots(&[DropArea { area: FULL_HD, scale: 1.0 }], NotchStyle::Nook).len(), 3);
        assert_eq!(drop_slots(&[DropArea { area: FULL_HD, scale: 1.0 }], NotchStyle::Floating).len(), 6);
    }

    #[test]
    fn taskbars_and_monitors_are_respected() {
        let top_bar = Area { left: 0, top: 48, width: 1920, height: 1032 };
        assert_eq!(anchor_origin(NotchAnchor::TopCenter, top_bar, 648, 648, 292, 0).1, 48, "below a top taskbar");
        let left_bar = Area { left: 62, top: 0, width: 1858, height: 1080 };
        assert_eq!(anchor_origin(NotchAnchor::TopLeft, left_bar, 648, 648, 292, 0).0, 62, "after a left taskbar");
        let second = Area { left: 1920, top: 0, width: 2560, height: 1392 };
        let reserve = 972;
        assert_eq!(anchor_origin(NotchAnchor::TopRight, second, reserve, reserve, 438, 0).0 + reserve, 1920 + 2560);
        assert_eq!(anchor_center_x(NotchAnchor::BottomCenter, second, reserve), 1920 + 1280);
    }

    #[test]
    fn a_narrow_screen_centres_the_notch() {
        let narrow = Area { left: 0, top: 0, width: 600, height: 800 };
        for anchor in NotchAnchor::ALL {
            assert_eq!(anchor_center_x(anchor, narrow, 648), 300);
        }
    }

    #[test]
    fn a_drop_snaps_to_the_nearest_slot_on_any_monitor() {
        let monitors = [
            DropArea { area: FULL_HD, scale: 1.0 },
            DropArea { area: Area { left: 1920, top: 0, width: 2560, height: 1392 }, scale: 1.5 },
        ];
        let slots = drop_slots(&monitors, NotchStyle::Floating);
        assert_eq!(slots.len(), 12);
        let near = |p| nearest_slot(&slots, p).map(|s| (s.monitor, s.anchor));
        assert_eq!(near((300, 40)), Some((0, NotchAnchor::TopLeft)));
        assert_eq!(near((960, 1000)), Some((0, NotchAnchor::BottomCenter)));
        assert_eq!(near((4300, 1300)), Some((1, NotchAnchor::BottomRight)), "dropping on the other monitor re-pins");
        assert_eq!(near((2100, 30)), Some((1, NotchAnchor::TopLeft)));
        // Edge Nook: a drop near the bottom goes to the nearest top slot.
        let nook = drop_slots(&monitors, NotchStyle::Nook);
        assert_eq!(nearest_slot(&nook, (960, 1000)).map(|s| s.anchor), Some(NotchAnchor::TopCenter));
        assert_eq!(nearest_slot(&[], (0, 0)), None);
    }

    #[test]
    fn the_release_springs_into_place_without_overshoot() {
        let path = snap_path((100, 500), (636, 0), 8, false);
        assert_eq!(path.last(), Some(&(636, 0)), "ends exactly on the slot");
        assert!(path.len() > 4 && path.len() < 60, "a short animation: {} frames", path.len());
        assert!(path.windows(2).all(|w| w[1].0 >= w[0].0 && w[1].1 <= w[0].1), "monotonic: no overshoot");
        assert_eq!(snap_path((5, 5), (636, 0), 8, true), vec![(636, 0)], "reduce motion: instant");
        assert!(snap_path((636, 0), (636, 0), 8, false).is_empty());
    }
}
