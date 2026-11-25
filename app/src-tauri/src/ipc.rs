use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use ice_core::appearance::{IdleStyle, NotchStyle, SpringPreset, ThemeKind};
pub use ice_core::placement::NotchAnchor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Idle,
    Volume,
    Switch,
    MicMute,
    /// A new track started (wakes the notch; the UI's scheduler draws the track-change activity).
    Track,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSummary {
    pub index: usize,
    pub id: String,
    /// Persistent display path; `Command::SetPinnedMonitor` takes this, never `id`.
    pub display_path: String,
    /// Monitor's own name (EDID), e.g. "LG FULL HD".
    pub name: String,
    pub is_primary: bool,
    /// Desktop position in physical pixels, for drawing the monitor layout.
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub scale_factor: f64,
    pub refresh_hz: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HitRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotchSnapshot {
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub event_seq: u64,
    pub activity: ActivityKind,
    pub volume_percent: u8,
    pub muted: bool,
    pub mic_muted: bool,
    pub active_device: String,
    pub target_device: String,
    pub spatial_badge: Option<String>,
    pub preset: SpringPreset,
    pub system_reduce_motion: bool,
    pub theme: ThemeKind,
    pub idle_style: IdleStyle,
    pub notch_style: NotchStyle,
    /// D-20: the anchor in use (Edge Nook maps a bottom anchor to the top); a bottom one makes the notch grow upwards.
    pub anchor: NotchAnchor,
    pub expanded: bool,
    pub suspended: bool,
    pub hidden_for_bench: bool,
    pub active_monitor: usize,
    pub monitors: Vec<MonitorSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NotchDelta {
    State { snapshot: NotchSnapshot },
}

#[tauri::command]
pub(super) async fn open_music_service(
    window: tauri::WebviewWindow,
    service: ice_core::media::MusicServiceId,
) -> Result<(), String> {
    if window.label() != "notch" {
        return Err("Only the notch can launch music services".into());
    }
    tauri::async_runtime::spawn_blocking(move || ice_win::music_service::open(service))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| {
            tracing::warn!(%e, "Could not launch music service");
            e.to_string()
        })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ice_core::device::{DeviceState, Flow, Role};
    use ice_core::engine::{
        BindingView, Command, DevicePrefsView, DeviceSummary, EngineEvent, EngineSnapshot, Envelope, MediaAction,
        Notice, PerformanceProfile, RoleSet, SavedDevice, SettingsView, SpatialMode, TogglePairView, VolumeState,
    };
    use ice_core::media::{ArtInfo, NowPlaying, PlaybackStatus, Rgb};

    use super::*;

    #[test]
    fn export_ts_bindings() {
        let cfg = ts_rs::Config::default();
        let body = [
            "// Auto-generated from Rust IPC types via `cargo test -p ice-audio-manager export_ts_bindings`.",
            "// Do not edit manually.",
            "",
            "import type { IdleStyle, NotchAnchor, NotchStyle, SpringPreset, ThemeKind } from './engine';",
            "export type { IdleStyle, NotchAnchor, NotchStyle, SpringPreset, ThemeKind } from './engine';",
            "",
            &format!("export {}", ActivityKind::decl(&cfg)),
            "",
            &format!("export {}", MonitorSummary::decl(&cfg)),
            "",
            &format!("export {}", HitRect::decl(&cfg)),
            "",
            &format!("export {}", NotchSnapshot::decl(&cfg)),
            "",
            &format!("export {}", NotchDelta::decl(&cfg)),
            "",
        ]
        .join("\n");
        write_generated("types.ts", &body);

        let engine = [
            "// Auto-generated from the ice-core engine contract.",
            "// Do not edit manually.",
            "",
            &format!("export {}", SpringPreset::decl(&cfg)),
            "",
            &format!("export {}", ThemeKind::decl(&cfg)),
            "",
            &format!("export {}", IdleStyle::decl(&cfg)),
            "",
            &format!("export {}", NotchStyle::decl(&cfg)),
            "",
            &format!("export {}", ice_core::appearance::NowPlayingPill::decl(&cfg)),
            "",
            &format!("export {}", ice_core::appearance::HibernateLook::decl(&cfg)),
            "",
            &format!("export {}", ice_core::placement::NotchAnchor::decl(&cfg)),
            "",
            &format!("export {}", ice_core::placement::NotchPlacement::decl(&cfg)),
            "",
            &format!("export {}", ice_core::appearance::Appearance::decl(&cfg)),
            "",
            &format!("export {}", Flow::decl(&cfg)),
            "",
            &format!("export {}", Role::decl(&cfg)),
            "",
            &format!("export {}", DeviceState::decl(&cfg)),
            "",
            &format!("export {}", RoleSet::decl(&cfg)),
            "",
            &format!("export {}", SpatialMode::decl(&cfg)),
            "",
            &format!("export {}", DeviceSummary::decl(&cfg)),
            "",
            &format!("export {}", VolumeState::decl(&cfg)),
            "",
            &format!("export {}", ice_core::media::MusicServiceId::decl(&cfg)),
            "",
            &format!("export {}", ice_core::media::MusicServices::decl(&cfg)),
            "",
            &format!("export {}", ice_core::lyrics::LyricsConfig::decl(&cfg)),
            "",
            &format!("export {}", ice_core::lyrics::LyricsSource::decl(&cfg)),
            "",
            &format!("export {}", ice_core::lyrics::LyricsSyncKind::decl(&cfg)),
            "",
            &format!("export {}", ice_core::lyrics::LyricsStatus::decl(&cfg)),
            "",
            &format!("export {}", ice_core::lyrics::LyricWord::decl(&cfg)),
            "",
            &format!("export {}", ice_core::lyrics::LyricLine::decl(&cfg)),
            "",
            &format!("export {}", ice_core::lyrics::LyricsDocument::decl(&cfg)),
            "",
            &format!("export {}", ice_core::lyrics::LyricsState::decl(&cfg)),
            "",
            &format!("export {}", ice_core::lyrics::LyricsStorageStats::decl(&cfg)),
            "",
            &format!("export {}", MediaAction::decl(&cfg)),
            "",
            &format!("export {}", PlaybackStatus::decl(&cfg)),
            "",
            &format!("export {}", Rgb::decl(&cfg)),
            "",
            &format!("export {}", ArtInfo::decl(&cfg)),
            "",
            &format!("export {}", NowPlaying::decl(&cfg)),
            "",
            &format!("export {}", SavedDevice::decl(&cfg)),
            "",
            &format!("export {}", TogglePairView::decl(&cfg)),
            "",
            &format!("export {}", BindingView::decl(&cfg)),
            "",
            &format!("export {}", DevicePrefsView::decl(&cfg)),
            "",
            &format!("export {}", PerformanceProfile::decl(&cfg)),
            "",
            &format!("export {}", SettingsView::decl(&cfg)),
            "",
            &format!("export {}", EngineSnapshot::decl(&cfg)),
            "",
            &format!("export {}", Command::decl(&cfg)),
            "",
            &format!("export {}", Notice::decl(&cfg)),
            "",
            &format!("export {}", EngineEvent::decl(&cfg)),
            "",
            &format!("export {}", Envelope::decl(&cfg)),
            "",
            &format!("export {}", ice_core::meter::MeterFrame::decl(&cfg)),
            "",
        ]
        .join("\n");

        write_generated("engine.ts", &engine);
    }

    fn write_generated(name: &str, source: &str) {
        let body = source.split('\n').map(str::trim_end).collect::<Vec<_>>().join("\n");
        let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/lib/ipc").join(name);
        // Git may check the file out with CRLF. A test should not dirty the worktree.
        let unchanged = std::fs::read_to_string(&out).is_ok_and(|old| old.replace("\r\n", "\n") == body);
        if !unchanged {
            std::fs::write(out, body).expect("write generated IPC bindings");
        }
    }
}
