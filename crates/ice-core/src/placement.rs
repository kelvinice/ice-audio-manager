//! D-20 notch placement: where the notch sits on its monitor, and flexible horizontal drag-and-drop maths.
//! One rule for both hosts (the WebView notch window and the native pill).
//!
//! Each position fixes the notch's **centre** within the monitor's work area (the screen minus the taskbar). Both
//! hosts centre their own box on it: the WebView window is the full [`NOTCH_CANVAS_W`] canvas (expanded card
//! included), the native pill is small. So left/right clamping reserves the canvas width from both screen edges, and
//! the widest card never clips outside the monitor. Top anchors sit flush at (Edge Nook) or just below (Floating) the
//! work area's top; bottom anchors sit flush on (Edge Nook) or just above (Floating) the work area's bottom (above the
//! taskbar), growing upwards.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::appearance::NotchStyle;

/// Logical size of the WebView notch canvas. Twin: `CANVAS_W` / `CANVAS_H` in `app/ui/src/lib/notch/geometry.ts`.
pub const NOTCH_CANVAS_W: i32 = 648;
/// Logical height of the WebView notch canvas. Twin: `CANVAS_H` in `app/ui/src/lib/notch/geometry.ts`.
pub const NOTCH_CANVAS_H: i32 = 292;
/// At a Floating bottom anchor the pill sits `BOTTOM_PAD` (24) inside the canvas so its drop shadow has room; the
/// canvas then overhangs the work area by this much, leaving the pill 6 px from the taskbar.
/// Twin: `BOTTOM_PAD - TOP_PAD` in `geometry.ts`.
pub const CANVAS_BOTTOM_OVERHANG: i32 = 18;
/// Logical px magnetic snap radius around Left (0.0), Centre (0.5), and Right (1.0) when dropping without visual marks.
const SNAP_MAGNET_LOGICAL_PX: f64 = 14.0;

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

/// How the notch is moved: dragged freely along the top/bottom edge (default), or only from the tray and Settings (D-20).
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

    /// Both Edge Nook and Floating support all six anchors (top & bottom bezels/taskbars).
    #[must_use]
    pub const fn allowed_for(self, _style: NotchStyle) -> bool {
        true
    }

    /// Anchor actually used with `style`. Both styles support all anchors directly.
    #[must_use]
    pub const fn effective_for(self, _style: NotchStyle) -> Self {
        self
    }

    const fn column(self) -> Column {
        match self {
            Self::TopLeft | Self::BottomLeft => Column::Left,
            Self::TopCenter | Self::BottomCenter => Column::Centre,
            Self::TopRight | Self::BottomRight => Column::Right,
        }
    }

    const fn from_edge_column(bottom: bool, col: Column) -> Self {
        match (bottom, col) {
            (false, Column::Left) => Self::TopLeft,
            (false, Column::Centre) => Self::TopCenter,
            (false, Column::Right) => Self::TopRight,
            (true, Column::Left) => Self::BottomLeft,
            (true, Column::Centre) => Self::BottomCenter,
            (true, Column::Right) => Self::BottomRight,
        }
    }
}

/// How far (logical px) the WebView canvas sits from its anchored edge for `style`: flush at top, flush on bottom for
/// Edge Nook (`0`), and overhanging by drop-shadow room (`-18`) at bottom for Floating.
#[must_use]
pub const fn canvas_edge_offset_for(anchor: NotchAnchor, style: NotchStyle) -> i32 {
    if anchor.is_bottom() && matches!(style, NotchStyle::Floating) { -CANVAS_BOTTOM_OVERHANG } else { 0 }
}

/// Floating-style shorthand kept for existing callers/tests.
#[must_use]
pub const fn canvas_edge_offset(anchor: NotchAnchor) -> i32 {
    canvas_edge_offset_for(anchor, NotchStyle::Floating)
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

/// Like [`anchor_center_x`], but honours an optional custom horizontal `x_ratio` (`0.0..=1.0`) between Left and Right
/// while still clamping strictly within `[area.left + reserve_w / 2, area.left + area.width - reserve_w / 2]`.
#[must_use]
pub fn anchor_center_x_at(anchor: NotchAnchor, x_ratio: Option<f32>, area: Area, reserve_w: i32) -> i32 {
    let Some(raw_ratio) = x_ratio else {
        return anchor_center_x(anchor, area, reserve_w);
    };
    let centre = area.left + area.width / 2;
    if area.width <= reserve_w {
        return centre;
    }
    let ratio = if raw_ratio.is_finite() { raw_ratio.clamp(0.0, 1.0) } else { 0.5 };
    let min_x = area.left + reserve_w / 2;
    let max_x = area.left + area.width - reserve_w / 2;
    min_x + (f64::from(max_x - min_x) * f64::from(ratio)).round() as i32
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

/// Like [`anchor_origin`], supporting an optional continuous horizontal `x_ratio` (`0.0..=1.0`).
#[must_use]
pub fn anchor_origin_at(
    anchor: NotchAnchor,
    x_ratio: Option<f32>,
    area: Area,
    reserve_w: i32,
    box_w: i32,
    box_h: i32,
    edge_offset: i32,
) -> (i32, i32) {
    let x = anchor_center_x_at(anchor, x_ratio, area, reserve_w) - box_w / 2;
    let y = if anchor.is_bottom() { area.top + area.height - edge_offset - box_h } else { area.top + edge_offset };
    (x, y)
}

/// One monitor the notch can be dropped on: its work area and its scale (1.0 = 100 %).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropArea {
    pub area: Area,
    pub scale: f64,
}

/// Resolved drop placement on a monitor: edge/semantic anchor plus optional continuous horizontal ratio (`0.0..=1.0`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropTarget {
    pub monitor: usize,
    pub anchor: NotchAnchor,
    pub x_ratio: Option<f32>,
}

fn dist_sq_to_area(area: Area, point: (i32, i32)) -> i64 {
    let cx = point.0.clamp(area.left, area.left + area.width);
    let cy = point.1.clamp(area.top, area.top + area.height);
    let dx = i64::from(point.0 - cx);
    let dy = i64::from(point.1 - cy);
    dx * dx + dy * dy
}

fn area_contains(area: Area, point: (i32, i32)) -> bool {
    point.0 >= area.left && point.0 < area.left + area.width && point.1 >= area.top && point.1 < area.top + area.height
}

/// Resolves a continuous, markless drag drop at physical coordinate `point` (`centre_x, cursor_y`).
///
/// Selects the monitor containing (or closest to) `point`, picks top vs bottom edge by vertical midpoint, and clamps
/// horizontal centre inside `[left + reserve_w / 2, right - reserve_w / 2]` so the notch never clips off-screen.
/// Drops within a subtle 14 logical px magnet of Left, Centre, or Right snap cleanly to `x_ratio = None`.
#[must_use]
pub fn resolve_drop_target(monitors: &[DropArea], _style: NotchStyle, point: (i32, i32)) -> Option<DropTarget> {
    if monitors.is_empty() {
        return None;
    }
    let monitor = monitors
        .iter()
        .position(|m| area_contains(m.area, point))
        .or_else(|| (0..monitors.len()).min_by_key(|&i| dist_sq_to_area(monitors[i].area, point)))?;
    let m = monitors[monitor];
    let bottom = point.1 >= m.area.top + m.area.height / 2;
    let reserve_w = (f64::from(NOTCH_CANVAS_W) * m.scale).round() as i32;
    if m.area.width <= reserve_w {
        return Some(DropTarget {
            monitor,
            anchor: NotchAnchor::from_edge_column(bottom, Column::Centre),
            x_ratio: None,
        });
    }

    let min_x = m.area.left + reserve_w / 2;
    let max_x = m.area.left + m.area.width - reserve_w / 2;
    let centre_x = m.area.left + m.area.width / 2;
    let clamped_x = point.0.clamp(min_x, max_x);
    let magnet = (SNAP_MAGNET_LOGICAL_PX * m.scale).round() as i32;

    if (clamped_x - min_x).abs() <= magnet {
        return Some(DropTarget {
            monitor,
            anchor: NotchAnchor::from_edge_column(bottom, Column::Left),
            x_ratio: None,
        });
    }
    if (clamped_x - centre_x).abs() <= magnet {
        return Some(DropTarget {
            monitor,
            anchor: NotchAnchor::from_edge_column(bottom, Column::Centre),
            x_ratio: None,
        });
    }
    if (clamped_x - max_x).abs() <= magnet {
        return Some(DropTarget {
            monitor,
            anchor: NotchAnchor::from_edge_column(bottom, Column::Right),
            x_ratio: None,
        });
    }

    let span = (max_x - min_x).max(1);
    let ratio = (f64::from(clamped_x - min_x) / f64::from(span)).clamp(0.0, 1.0) as f32;
    let col = if ratio < 1.0 / 3.0 {
        Column::Left
    } else if ratio > 2.0 / 3.0 {
        Column::Right
    } else {
        Column::Centre
    };
    Some(DropTarget { monitor, anchor: NotchAnchor::from_edge_column(bottom, col), x_ratio: Some(ratio) })
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
    fn bottom_anchors_work_for_both_edge_nook_and_floating() {
        // Floating: canvas overhangs below work area by 18 px for shadow room; pill sits 6 px above taskbar.
        let pill = anchor_origin(NotchAnchor::BottomCenter, FULL_HD, 648, 116, 28, 6);
        assert_eq!(pill, (902, 1040 - 6 - 28));
        let floating_canvas = anchor_origin(
            NotchAnchor::BottomCenter,
            FULL_HD,
            648,
            648,
            292,
            canvas_edge_offset_for(NotchAnchor::BottomCenter, NotchStyle::Floating),
        );
        assert_eq!(floating_canvas.1 + 292, 1040 + CANVAS_BOTTOM_OVERHANG);
        assert_eq!(floating_canvas.1 + 292 - 24, pill.1 + 28);

        // Edge Nook: flush against top of taskbar (edge offset = 0 for both canvas and native pill).
        assert_eq!(canvas_edge_offset_for(NotchAnchor::BottomCenter, NotchStyle::Nook), 0);
        let nook_canvas = anchor_origin(
            NotchAnchor::BottomCenter,
            FULL_HD,
            648,
            648,
            292,
            canvas_edge_offset_for(NotchAnchor::BottomCenter, NotchStyle::Nook),
        );
        assert_eq!(nook_canvas.1 + 292, 1040, "Edge Nook canvas bottom is flush on taskbar");
        let nook_pill = anchor_origin(NotchAnchor::BottomCenter, FULL_HD, 648, 132, 30, 0);
        assert_eq!(nook_pill.1 + 30, 1040, "Edge Nook native pill bottom is flush on taskbar");

        for anchor in NotchAnchor::ALL {
            assert!(anchor.allowed_for(NotchStyle::Nook));
            assert!(anchor.allowed_for(NotchStyle::Floating));
            assert_eq!(anchor.effective_for(NotchStyle::Nook), anchor);
            assert_eq!(anchor.effective_for(NotchStyle::Floating), anchor);
        }
    }

    #[test]
    fn custom_horizontal_ratio_stays_clamped_within_monitor_bounds() {
        for ratio in [-0.5_f32, 0.0, 0.25, 0.5, 0.75, 1.0, 1.5, f32::NAN] {
            let canvas = anchor_origin_at(NotchAnchor::TopCenter, Some(ratio), FULL_HD, 648, 648, 292, 0);
            assert!(canvas.0 >= FULL_HD.left, "ratio {ratio} must not clip left edge");
            assert!(canvas.0 + 648 <= FULL_HD.left + FULL_HD.width, "ratio {ratio} must not clip right edge");
        }
        assert_eq!(anchor_center_x_at(NotchAnchor::TopCenter, Some(0.0), FULL_HD, 648), 324);
        assert_eq!(anchor_center_x_at(NotchAnchor::TopCenter, Some(0.5), FULL_HD, 648), 960);
        assert_eq!(anchor_center_x_at(NotchAnchor::TopCenter, Some(1.0), FULL_HD, 648), 1920 - 324);
        let narrow = Area { left: 0, top: 0, width: 600, height: 800 };
        assert_eq!(anchor_center_x_at(NotchAnchor::TopLeft, Some(0.2), narrow, 648), 300);
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
    fn continuous_drop_clamps_to_edges_and_picks_arbitrary_horizontal_pixels() {
        let monitors = [
            DropArea { area: FULL_HD, scale: 1.0 },
            DropArea { area: Area { left: 1920, top: 0, width: 2560, height: 1392 }, scale: 1.5 },
        ];
        // Dropping far left clamps cleanly to TopLeft without visual marks or clipping.
        let left_drop = resolve_drop_target(&monitors, NotchStyle::Nook, (40, 20)).unwrap();
        assert_eq!(left_drop.monitor, 0);
        assert_eq!(left_drop.anchor, NotchAnchor::TopLeft);
        assert_eq!(left_drop.x_ratio, None);

        // Dropping far right on bottom edge picks BottomRight cleanly for Edge Nook too.
        let right_bottom = resolve_drop_target(&monitors, NotchStyle::Nook, (1890, 990)).unwrap();
        assert_eq!(right_bottom.monitor, 0);
        assert_eq!(right_bottom.anchor, NotchAnchor::BottomRight);
        assert_eq!(right_bottom.x_ratio, None);

        // Gentle silent magnet near centre.
        let near_centre = resolve_drop_target(&monitors, NotchStyle::Floating, (952, 40)).unwrap();
        assert_eq!((near_centre.anchor, near_centre.x_ratio), (NotchAnchor::TopCenter, None));

        // Arbitrary intermediate pixel (e.g. x = 520 between min 324 and max 1596) stores exact continuous ratio.
        let custom = resolve_drop_target(&monitors, NotchStyle::Nook, (520, 80)).unwrap();
        assert_eq!(custom.monitor, 0);
        assert_eq!(custom.anchor, NotchAnchor::TopLeft);
        let ratio = custom.x_ratio.expect("custom ratio preserved between left and centre");
        assert!((anchor_center_x_at(custom.anchor, Some(ratio), FULL_HD, 648) - 520).abs() <= 1);

        // Secondary 150 % DPI monitor re-pins and clamps inside [1920 + 486, 4480 - 486].
        let second_custom = resolve_drop_target(&monitors, NotchStyle::Nook, (2700, 1200)).unwrap();
        assert_eq!(second_custom.monitor, 1);
        assert!(second_custom.anchor.is_bottom());
        let canvas_x =
            anchor_origin_at(second_custom.anchor, second_custom.x_ratio, monitors[1].area, 972, 972, 438, 0).0;
        assert!(canvas_x >= 1920 && canvas_x + 972 <= 1920 + 2560);
        assert_eq!(resolve_drop_target(&[], NotchStyle::Nook, (0, 0)), None);
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
