use std::sync::{Arc, mpsc::Receiver};

use ice_core::device::Flow;
use ice_core::engine::{DeviceSummary, EngineEvent, EngineSnapshot, Envelope, RoleSet, SpatialMode};
use tauri::{AppHandle, Emitter};

use super::{ActivityKind, NotchSnapshot, SharedRuntime, tray};

pub(super) fn project(snapshot: &EngineSnapshot, view: &mut NotchSnapshot) {
    let active = snapshot.playback.iter().find(|device| device.is_default);
    let target = preview_target(snapshot);
    view.active_device = active.map_or_else(|| "No playback device".into(), |device| device.name.clone());
    view.target_device = target.map_or_else(String::new, |device| device.name.clone());
    view.spatial_badge = active.and_then(|device| match device.spatial? {
        SpatialMode::WindowsSonic => Some("Sonic".into()),
        SpatialMode::DolbyAtmosHeadphones | SpatialMode::DolbyAtmosHomeTheater | SpatialMode::DolbyAtmosSpeakers => {
            Some("Atmos".into())
        }
        SpatialMode::DtsHeadphoneX | SpatialMode::DtsXUltra | SpatialMode::DtsXHomeTheater => Some("DTS".into()),
        SpatialMode::Keep | SpatialMode::Off => None,
    });
    view.volume_percent = snapshot.volume.map_or(0, |volume| volume.percent);
    view.muted = snapshot.volume.is_some_and(|volume| volume.muted);
    view.mic_muted = snapshot.mic_muted.unwrap_or(false);
}

pub(super) fn preview_target(snapshot: &EngineSnapshot) -> Option<&DeviceSummary> {
    snapshot.playback.iter().find(|device| !device.is_default)
}

pub(super) fn watch(
    app: AppHandle,
    state: Arc<SharedRuntime>,
    events: Receiver<Envelope>,
    initial_revision: u64,
) -> std::io::Result<()> {
    std::thread::Builder::new().name("ice-engine-events".into()).spawn(move || {
        let mut revision = initial_revision;
        for delta in events {
            if delta.revision <= revision {
                continue;
            }
            revision = delta.revision;
            let app_for_event = app.clone();
            let state = Arc::clone(&state);
            if app.run_on_main_thread(move || apply_event(&app_for_event, &state, delta)).is_err() {
                break;
            }
        }
    })?;
    Ok(())
}

fn apply_event(app: &AppHandle, state: &SharedRuntime, delta: Envelope) {
    let snapshot = state.engine.snapshot();
    let activity = match &delta.event {
        EngineEvent::VolumeChanged { .. } => Some(ActivityKind::Volume),
        EngineEvent::MicMuteChanged { .. } => Some(ActivityKind::MicMute),
        EngineEvent::Switched { device, roles, .. } => {
            tracing::info!(device_id = %device.id, ?roles, "Audio switch confirmed");
            (device.flow == Flow::Render && *roles != RoleSet::Communications).then_some(ActivityKind::Switch)
        }
        EngineEvent::Notice { notice } => {
            tracing::warn!(?notice, "Audio engine notice");
            None
        }
        _ => None,
    };

    state.update_and_emit(app, |view| {
        project(&snapshot, view);
        if let Some(activity) = activity
            && snapshot.revision == delta.revision
        {
            view.activity = activity;
            view.event_seq = view.event_seq.saturating_add(1);
        }
    });
    if matches!(
        delta.event,
        EngineEvent::Switched { .. }
            | EngineEvent::DevicesChanged { .. }
            | EngineEvent::DeviceArrived { .. }
            | EngineEvent::DeviceLeft { .. }
    ) {
        tray::refresh_tooltip(app, &snapshot);
    }
    let _ = app.emit_to("notch", "engine://delta/v1", &delta);
    let _ = app.emit_to("settings", "engine://delta/v1", &delta);
}
