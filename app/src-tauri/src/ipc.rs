use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SpringPreset {
    Snappy,
    Smooth,
    Bouncy,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    Ice,
    Midnight,
    Fluent,
    Minimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Idle,
    Volume,
    Switch,
    MicMute,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSummary {
    pub index: usize,
    pub id: String,
    pub is_primary: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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
    pub expanded: bool,
    pub hidden_for_bench: bool,
    pub active_monitor: usize,
    pub monitors: Vec<MonitorSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NotchDelta {
    State { snapshot: NotchSnapshot },
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ice_core::device::{DeviceState, Flow, Role};
    use ice_core::engine::{
        Command, DeviceSummary, EngineEvent, EngineSnapshot, Envelope, Notice, RoleSet, SpatialMode, VolumeState,
    };

    use super::*;

    #[test]
    fn export_ts_bindings() {
        let cfg = ts_rs::Config::default();
        let body = [
            "// Auto-generated from Rust IPC types via `cargo test -p ice-audio-manager export_ts_bindings`.",
            "// Do not edit manually.",
            "",
            &format!("export {}", SpringPreset::decl(&cfg)),
            "",
            &format!("export {}", ThemeKind::decl(&cfg)),
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
