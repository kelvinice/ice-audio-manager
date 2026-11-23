//! Device list, volume and microphone state: enumeration, summaries and the snapshot's device fields.

use super::*;

impl Worker {
    /// A summary without any WinRT calls (spatial unknown).
    pub(super) fn summary_basic(&self, e: &EndpointInfo) -> DeviceSummary {
        DeviceSummary {
            id: e.id.clone(),
            name: self
                .config
                .prefs_for(e)
                .and_then(|p| p.alias.clone())
                .unwrap_or_else(|| normalize_name(e.friendly_name.as_deref().unwrap_or(&e.id))),
            flow: e.flow,
            state: e.state,
            is_default: e.default_for.contains(&Role::Multimedia),
            is_default_communications: e.default_for.contains(&Role::Communications),
            form_factor: e.form_factor,
            is_virtual: e.hardware_key.as_deref().is_some_and(|k| k.starts_with("ROOT\\") || k.starts_with("SWD\\")),
            spatial: None,
        }
    }

    /// Full summary: one fast spatial read for playback devices (review ENG-02).
    pub(super) fn summary(&self, e: &EndpointInfo) -> DeviceSummary {
        let spatial = if e.flow == Flow::Render { spatial::active(&e.id).ok().and_then(mode_from) } else { None };
        DeviceSummary { spatial, ..self.summary_basic(e) }
    }

    /// Re-enumerates, updates the snapshot and (when `notify`) emits arrivals, departures and `DevicesChanged`.
    pub(super) fn refresh(&mut self, notify: bool) {
        let fresh = match self.eps.list(false) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(error = %e, "endpoint enumeration failed; keeping the previous device list");
                return;
            }
        };
        let old = std::mem::replace(&mut self.endpoints, fresh);
        let current = |list: &[EndpointInfo]| {
            list.iter()
                .find(|e| e.flow == Flow::Render && e.default_for.contains(&Role::Multimedia))
                .map(|e| e.id.clone())
        };
        let (old_default, new_default) = (current(&old), current(&self.endpoints));

        // One summary per endpoint per refresh (review ENG-02).
        let summaries: Vec<DeviceSummary> = self.endpoints.iter().map(|e| self.summary(e)).collect();
        let visible = |flow: Flow| -> Vec<DeviceSummary> {
            self.endpoints
                .iter()
                .zip(&summaries)
                .filter(|(e, _)| e.flow == flow && !self.config.prefs_for(e).is_some_and(|p| p.hidden))
                .map(|(_, s)| s.clone())
                .collect()
        };
        let (playback, recording) = (visible(Flow::Render), visible(Flow::Capture));

        // Re-create both watches on every refresh: a same-ID reconnect can leave the old interfaces stale (ENG-07).
        self.render_watch = None;
        self.mic_watch = None;
        lock(&self.shared).meter.rebind();
        let render_dev = self.eps.default_device(Flow::Render, Role::Multimedia).ok();
        let mic_dev = self.mic_device();
        self.render_watch = render_dev.as_ref().and_then(|d| {
            volume::VolumeWatch::new(d, audio_sink(&self.engine_tx))
                .inspect_err(|e| tracing::warn!(error = %e, "playback volume watch failed"))
                .ok()
        });
        self.mic_watch = mic_dev.as_ref().and_then(|d| {
            volume::VolumeWatch::new(d, mic_sink(&self.engine_tx))
                .inspect_err(|e| tracing::warn!(error = %e, "microphone volume watch failed"))
                .ok()
        });
        let volume = render_dev
            .as_ref()
            .and_then(|d| volume::get(d).ok())
            .map(|(scalar, muted)| VolumeState { percent: percent(scalar), muted });
        let mic = mic_dev.as_ref().and_then(|d| volume::get(d).ok());

        let lists_changed = {
            let mut sh = lock(&self.shared);
            let changed = sh.snapshot.playback != playback || sh.snapshot.recording != recording;
            sh.snapshot.playback.clone_from(&playback);
            sh.snapshot.recording.clone_from(&recording);
            sh.snapshot.volume = volume;
            sh.snapshot.mic_muted = mic.map(|(_, m)| m);
            sh.snapshot.mic_volume = mic.map(|(s, _)| percent(s));
            changed
        };
        self.publish_settings(notify);
        if !notify {
            return;
        }
        let arrived: Vec<DeviceSummary> = self
            .endpoints
            .iter()
            .zip(&summaries)
            .filter(|(e, _)| !old.iter().any(|o| o.id == e.id))
            .map(|(_, s)| s.clone())
            .collect();
        // Departed devices: no WinRT calls on endpoints that are gone.
        let left: Vec<DeviceSummary> = old
            .iter()
            .filter(|o| !self.endpoints.iter().any(|e| e.id == o.id))
            .map(|o| DeviceSummary {
                state: DeviceState::NotPresent,
                is_default: false,
                is_default_communications: false,
                ..self.summary_basic(o)
            })
            .collect();
        for device in arrived {
            self.emit(EngineEvent::DeviceArrived { device });
        }
        for device in left {
            self.emit(EngineEvent::DeviceLeft { device });
        }
        if lists_changed {
            self.emit(EngineEvent::DevicesChanged { playback, recording });
        }
        if old_default != new_default
            && let Some(volume) = volume
        {
            self.emit(EngineEvent::VolumeChanged { volume });
        }
    }

    pub(super) fn default_id(&self, flow: Flow, role: Role) -> Option<String> {
        self.endpoints.iter().find(|e| e.flow == flow && e.default_for.contains(&role)).map(|e| e.id.clone())
    }

    pub(super) fn mic_device(&self) -> Option<IMMDevice> {
        self.eps.default_device(Flow::Capture, Role::Communications).ok()
    }

    pub(super) fn audio(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::Volume { scalar, muted } => {
                let volume = VolumeState { percent: percent(scalar), muted };
                lock(&self.shared).snapshot.volume = Some(volume);
                self.emit(EngineEvent::VolumeChanged { volume });
            }
            // Very chatty and rarely user-visible; a real change also arrives as state/default events.
            AudioEvent::PropertyChanged { .. } => {}
            _ => {
                // Sliding quiet window, capped so a long burst still refreshes (review ENG-09).
                let now = Instant::now();
                let first = *self.refresh_first.get_or_insert(now);
                self.refresh_at = Some((now + REFRESH_DEBOUNCE).min(first + REFRESH_MAX_WAIT));
            }
        }
    }

    /// Microphone volume/mute from the mic watch or our own changes; emits only real changes.
    pub(super) fn on_mic(&mut self, scalar: f32, muted: bool) {
        let p = percent(scalar);
        let (muted_changed, volume_changed) = {
            let mut sh = lock(&self.shared);
            let changes = (sh.snapshot.mic_muted != Some(muted), sh.snapshot.mic_volume != Some(p));
            sh.snapshot.mic_muted = Some(muted);
            sh.snapshot.mic_volume = Some(p);
            changes
        };
        if muted_changed {
            self.emit(EngineEvent::MicMuteChanged { muted });
        }
        if volume_changed {
            self.emit(EngineEvent::MicVolumeChanged { percent: p });
        }
    }
}

fn percent(scalar: f32) -> u8 {
    // Clamped to 0–100 first, so the cast can't truncate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let p = (scalar.clamp(0.0, 1.0) * 100.0).round() as u8;
    p
}

/// Active spatial state → UI mode. `None` = an unknown provider.
fn mode_from(active: Option<(Option<SpatialFormat>, String)>) -> Option<SpatialMode> {
    match active {
        None => Some(SpatialMode::Off),
        Some((Some(f), _)) => Some(match f {
            SpatialFormat::WindowsSonic => SpatialMode::WindowsSonic,
            SpatialFormat::DolbyAtmosHeadphones => SpatialMode::DolbyAtmosHeadphones,
            SpatialFormat::DolbyAtmosHomeTheater => SpatialMode::DolbyAtmosHomeTheater,
            SpatialFormat::DolbyAtmosSpeakers => SpatialMode::DolbyAtmosSpeakers,
            SpatialFormat::DtsHeadphoneX => SpatialMode::DtsHeadphoneX,
            SpatialFormat::DtsXUltra => SpatialMode::DtsXUltra,
            SpatialFormat::DtsXHomeTheater => SpatialMode::DtsXHomeTheater,
        }),
        Some((None, _)) => None,
    }
}
