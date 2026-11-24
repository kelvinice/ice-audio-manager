use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use ice_core::appearance::NowPlayingPill;
use ice_core::config::ConfigStore;
use ice_core::device::Flow;
use ice_core::engine::{
    Command, DeviceSummary, Engine, EngineEvent, EngineSnapshot, Envelope, Notice, RoleSet, SpatialMode,
};
use ice_core::media::{NowPlaying, PlaybackStatus};
use ice_win::audio_engine::AudioEngine;
use ice_win::native_dot::{NativeDotHandle, NativePillTheme};
use tauri::{AppHandle, Emitter, Manager};

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
    view.theme = snapshot.settings.appearance.theme;
    view.preset = snapshot.settings.appearance.preset;
    view.idle_style = snapshot.settings.appearance.idle_style;
    view.volume_percent = snapshot.volume.map_or(0, |volume| volume.percent);
    view.muted = snapshot.volume.is_some_and(|volume| volume.muted);
    view.mic_muted = snapshot.mic_muted.unwrap_or(false);
}

/// What makes a track "new" for the notch: the app and the track's names (art and position don't count;
/// some apps send the cover a moment after the title).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TrackKey {
    app_id: String,
    title: String,
    artist: String,
    album: String,
}

impl TrackKey {
    pub(super) fn of(m: &NowPlaying) -> Self {
        Self { app_id: m.app_id.clone(), title: m.title.clone(), artist: m.artist.clone(), album: m.album.clone() }
    }
}

/// Mirrors the UI's `isMediaActive`: something is (or is about to be) playing and has a name to show.
fn media_active(m: &NowPlaying) -> bool {
    !matches!(m.status, PlaybackStatus::Closed | PlaybackStatus::Stopped)
        && !(m.title.trim().is_empty() && m.artist.trim().is_empty())
}

/// Remembers `media` in `last` and says whether it is a new, active track worth waking the notch for.
fn is_new_track(last: &mut Option<TrackKey>, media: Option<&NowPlaying>) -> bool {
    let key = media.map(TrackKey::of);
    let changed = *last != key;
    *last = key;
    changed && media.is_some_and(media_active)
}

pub(super) fn preview_target(snapshot: &EngineSnapshot) -> Option<&DeviceSummary> {
    snapshot.playback.iter().find(|device| !device.is_default)
}

type EngineRef = Arc<dyn Engine>;

pub(super) struct EngineSlot(Mutex<Result<EngineRef, String>>);

impl EngineSlot {
    pub(super) fn new(engine: Option<EngineRef>) -> Self {
        Self(Mutex::new(engine.ok_or_else(|| "Starting Windows audio…".into())))
    }

    pub(super) fn current(&self) -> Option<EngineRef> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).as_ref().ok().cloned()
    }

    pub(super) fn art(&self, id: &str) -> Option<Vec<u8>> {
        self.current()?.art(id)
    }

    pub(super) fn unavailable(&self) -> Option<String> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).as_ref().err().cloned()
    }

    pub(super) fn snapshot(&self) -> EngineSnapshot {
        self.current().map_or_else(EngineSnapshot::default, |engine| engine.snapshot())
    }

    pub(super) fn send(&self, command: Command) {
        if let Some(engine) = self.current() {
            engine.send(command);
        }
    }

    fn set(&self, engine: Result<EngineRef, String>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = engine;
    }
}

fn start_audio() -> Result<EngineRef, String> {
    let path = PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA is unavailable")?)
        .join("IceAudioManager")
        .join("config.toml");
    Ok(Arc::new(AudioEngine::start(ConfigStore::new(path))?))
}

fn retry_start<T>(
    mut start: impl FnMut() -> Result<T, String>,
    mut on_failure: impl FnMut(String, Duration) -> bool,
) -> Option<T> {
    let mut delay = Duration::from_secs(1);
    loop {
        match start() {
            Ok(engine) => return Some(engine),
            Err(error) => {
                if !on_failure(error, delay) {
                    return None;
                }
                delay = (delay * 2).min(Duration::from_secs(30));
            }
        }
    }
}

enum Update {
    // Boxed: the snapshot (devices, settings, media) is much larger than a delta.
    Snapshot(Box<EngineSnapshot>),
    Delta(Envelope),
}

fn follow(engine: &dyn Engine, mut publish: impl FnMut(Update) -> bool) {
    loop {
        let events = engine.subscribe();
        let snapshot = engine.snapshot();
        let initial_revision = snapshot.revision;
        if !publish(Update::Snapshot(Box::new(snapshot))) {
            return;
        }
        let mut revision = 0;
        for delta in events {
            if delta.revision <= revision {
                continue;
            }
            revision = delta.revision;
            // The snapshot supersedes older deltas, except for transient prompts.
            if revision <= initial_revision
                && !matches!(
                    delta.event,
                    EngineEvent::Notice {
                        notice: Notice::NotConnected { .. }
                            | Notice::ChooseDevice { .. }
                            | Notice::SettingRejected { .. }
                    }
                )
            {
                continue;
            }
            if !publish(Update::Delta(delta)) {
                return;
            }
        }
        tracing::warn!("Audio event stream ended; resubscribing");
    }
}

pub(super) fn watch(app: AppHandle, state: Arc<SharedRuntime>) -> std::io::Result<()> {
    std::thread::Builder::new().name("ice-engine-events".into()).spawn(move || {
        if state.allow_exit.load(Ordering::Acquire) {
            return;
        }
        let existing = state.engine.current();
        let Some(engine) = existing.clone().or_else(|| {
            retry_start(start_audio, |error, delay| {
                tracing::warn!(%error, retry_after_secs = delay.as_secs(), "Windows audio unavailable");
                let ui_state = Arc::clone(&state);
                if state.allow_exit.load(Ordering::Acquire)
                    || app
                        .run_on_main_thread(move || {
                            ui_state.engine.set(Err(error));
                            tray::refresh_tooltip(&ui_state, &EngineSnapshot::default());
                        })
                        .is_err()
                {
                    return false;
                }
                std::thread::park_timeout(delay);
                !state.allow_exit.load(Ordering::Acquire)
            })
        }) else {
            return;
        };
        let mut install = existing.is_none().then(|| Arc::clone(&engine));
        let generation = Arc::new(AtomicU64::new(0));
        follow(engine.as_ref(), |update| {
            if state.allow_exit.load(Ordering::Acquire) {
                return false;
            }
            let app_for_event = app.clone();
            let state = Arc::clone(&state);
            let ready = install.take();
            let stamp = if matches!(update, Update::Snapshot(_)) {
                generation.fetch_add(1, Ordering::AcqRel) + 1
            } else {
                generation.load(Ordering::Acquire)
            };
            let active_generation = Arc::clone(&generation);
            app.run_on_main_thread(move || {
                if let Some(engine) = ready {
                    state.engine.set(Ok(engine));
                    tracing::info!("Windows audio engine ready");
                }
                // A fresh snapshot replaces any work still queued from the old subscription.
                if active_generation.load(Ordering::Acquire) != stamp {
                    return;
                }
                match update {
                    Update::Snapshot(snapshot) => apply_snapshot(&app_for_event, &state, &snapshot),
                    Update::Delta(delta) => apply_event(&app_for_event, &state, delta),
                }
            })
            .is_ok()
        });
    })?;
    Ok(())
}

/// Feeds the native dot a snapshot plus the saved theme. The shell owns the theme mapping, so Phase 4 user
/// themes (CSS overrides) can replace the built-in palette here.
pub(super) fn native_dot_snapshot(dot: &NativeDotHandle, snapshot: &EngineSnapshot) {
    dot.set_theme(NativePillTheme::builtin(snapshot.settings.appearance.theme));
    dot.apply_snapshot(snapshot);
}

fn apply_snapshot(app: &AppHandle, state: &Arc<SharedRuntime>, snapshot: &EngineSnapshot) {
    super::placement::settings(app, state, &snapshot.settings);
    if let Some(dot) = state.native_dot.lock().unwrap_or_else(PoisonError::into_inner).as_ref() {
        native_dot_snapshot(dot, snapshot);
    }
    state.update_and_emit(app, |view| project(snapshot, view));
    tray::refresh_tooltip(state, snapshot);
    let _ = app.emit_to("notch", "engine://snapshot/v1", snapshot);
    let _ = app.emit_to("settings", "engine://snapshot/v1", snapshot);
}

fn apply_event(app: &AppHandle, state: &Arc<SharedRuntime>, delta: Envelope) {
    tracing::debug!(revision = delta.revision, event = ?delta.event, "Audio engine event");
    if !state.bench_mode
        && let Some(dot) = state.native_dot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref()
    {
        if let EngineEvent::SettingsChanged { settings } = &delta.event {
            dot.set_theme(NativePillTheme::builtin(settings.appearance.theme));
        }
        let _ = dot.apply_envelope(&delta);
    }

    let snapshot = state.engine.snapshot();
    if matches!(delta.event, EngineEvent::SettingsChanged { .. }) {
        super::placement::settings(app, state, &snapshot.settings);
    }
    let activity = match &delta.event {
        EngineEvent::VolumeChanged { .. } | EngineEvent::MicVolumeChanged { .. } => Some(ActivityKind::Volume),
        EngineEvent::MicMuteChanged { .. } => Some(ActivityKind::MicMute),
        EngineEvent::Switched { device, roles, relinked, spatial_applied } => {
            tracing::info!(device_id = %device.id, ?roles, relinked, ?spatial_applied, "Audio switch confirmed");
            (device.flow == Flow::Render && *roles != RoleSet::Communications).then_some(ActivityKind::Switch)
        }
        EngineEvent::DeviceArrived { .. } | EngineEvent::DeviceLeft { .. } => Some(ActivityKind::Switch),
        EngineEvent::Notice { notice } => {
            tracing::warn!(?notice, "Audio engine notice");
            tray::show_notice(state, notice);
            None
        }
        EngineEvent::MediaChanged { media } => {
            let mut last = state.last_track.lock().unwrap_or_else(PoisonError::into_inner);
            // Always remember the track; only wake the notch when the now-playing pill pops up at all.
            let new_track = is_new_track(&mut last, media.as_ref());
            (new_track && snapshot.settings.appearance.now_playing != NowPlayingPill::Off)
                .then_some(ActivityKind::Track)
        }
        EngineEvent::ShowNotch => {
            super::show_notch::show(app, state, super::show_notch::Open::Toggle);
            None
        }
        EngineEvent::DevicesChanged { .. }
        | EngineEvent::NoticesChanged { .. }
        | EngineEvent::SettingsChanged { .. } => None,
    };

    let needs_webview = activity.is_some() && !state.bench_mode && state.placement().allows_webview();
    let waking = needs_webview
        && app.get_webview_window("notch").is_some()
        && (state.want_suspended.load(Ordering::Acquire)
            || state.notch_suspended.load(Ordering::Acquire)
            || !state.notch_visible.load(Ordering::Acquire));

    let _ = app.emit_to("notch", "engine://delta/v1", &delta);
    let _ = app.emit_to("settings", "engine://delta/v1", &delta);

    let view = state.update_and_emit(app, |view| {
        project(&snapshot, view);
        if let Some(activity) = activity.filter(|_| needs_webview) {
            view.activity = activity;
            view.event_seq = view.event_seq.saturating_add(1);
            if waking {
                view.suspended = false;
            }
        }
    });

    if matches!(
        delta.event,
        EngineEvent::Switched { .. }
            | EngineEvent::DevicesChanged { .. }
            | EngineEvent::DeviceArrived { .. }
            | EngineEvent::DeviceLeft { .. }
            | EngineEvent::Notice {
                notice: Notice::NotConnected { .. } | Notice::ChooseDevice { .. } | Notice::SettingRejected { .. }
            }
            | EngineEvent::NoticesChanged { .. }
            | EngineEvent::SettingsChanged { .. }
    ) {
        tray::refresh_tooltip(state, &snapshot);
    }

    if waking {
        super::wake_notch_on_main(app, state, view.event_seq);
    } else if needs_webview && app.get_webview_window("notch").is_none() {
        super::request_preview(app, state, super::Preview::Notch);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{Receiver, channel};

    use super::*;

    #[test]
    fn only_a_new_active_track_wakes_the_notch() {
        let track = |title: &str, status: PlaybackStatus| NowPlaying {
            app_id: "Spotify.exe".into(),
            title: title.into(),
            artist: "Artist".into(),
            album: "Album".into(),
            status,
            position_ms: Some(0),
            duration_ms: Some(1000),
            position_updated_ms: None,
            art: None,
        };
        let mut last = None;
        assert!(is_new_track(&mut last, Some(&track("A", PlaybackStatus::Playing))));
        // Position, status and late art don't count.
        let mut later = track("A", PlaybackStatus::Paused);
        later.position_ms = Some(500);
        later.art =
            Some(ice_core::media::ArtInfo { id: "x".into(), width: 1, height: 1, accent: ice_core::media::ICE_ACCENT });
        assert!(!is_new_track(&mut last, Some(&later)));
        assert!(is_new_track(&mut last, Some(&track("B", PlaybackStatus::Playing))));
        // A stopped session or none at all is not a track to show.
        assert!(!is_new_track(&mut last, Some(&track("C", PlaybackStatus::Stopped))));
        assert!(!is_new_track(&mut last, None));
        assert!(is_new_track(&mut last, Some(&track("C", PlaybackStatus::Playing))));
    }

    struct DroppedSubscriber(AtomicU64);

    impl Engine for DroppedSubscriber {
        fn snapshot(&self) -> EngineSnapshot {
            EngineSnapshot { revision: self.0.load(Ordering::Relaxed), ..Default::default() }
        }

        fn send(&self, _: Command) {}

        fn subscribe(&self) -> Receiver<Envelope> {
            let revision = self.0.fetch_add(10, Ordering::Relaxed) + 10;
            let (tx, rx) = channel();
            tx.send(Envelope { revision, event: EngineEvent::NoticesChanged { notices: vec![] } }).unwrap();
            tx.send(Envelope { revision: revision + 1, event: EngineEvent::NoticesChanged { notices: vec![] } })
                .unwrap();
            rx
        }
    }

    #[test]
    fn reconnect_subscribes_before_snapshot_and_skips_subsumed_deltas() {
        let engine = DroppedSubscriber(AtomicU64::new(0));
        let mut seen = Vec::new();
        follow(&engine, |update| {
            seen.push(match update {
                Update::Snapshot(snapshot) => ("snapshot", snapshot.revision),
                Update::Delta(delta) => ("delta", delta.revision),
            });
            seen.len() < 4
        });
        assert_eq!(seen, [("snapshot", 10), ("delta", 11), ("snapshot", 20), ("delta", 21)]);
    }

    #[test]
    fn startup_retries_with_capped_delays_and_can_stop() {
        let mut attempts = 0;
        let mut delays = Vec::new();
        let ready = retry_start(
            || {
                attempts += 1;
                if attempts == 8 { Ok("ready") } else { Err("audio service is unavailable".into()) }
            },
            |error, delay| {
                assert_eq!(error, "audio service is unavailable");
                delays.push(delay.as_secs());
                true
            },
        );
        assert_eq!(delays, [1, 2, 4, 8, 16, 30, 30]);
        assert_eq!(ready, Some("ready"));
        assert_eq!(retry_start::<()>(|| Err("stopping".into()), |_, _| false), None);
    }
}
