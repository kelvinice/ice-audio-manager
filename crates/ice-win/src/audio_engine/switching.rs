//! Commands, saved-reference resolution (D-16), confirmed switches and spatial sound (D-17).

use super::*;

impl Worker {
    pub(super) fn command(&mut self, command: Command) {
        match command {
            Command::SwitchTo { device_id, roles } => {
                match self.endpoints.iter().find(|e| e.id.eq_ignore_ascii_case(&device_id)).cloned() {
                    Some(target) => self.switch(&target, roles, None),
                    None => self.notice(Notice::NotConnected { name: device_id }),
                }
            }
            Command::SwitchBinding { binding_id } => {
                let Some(b) = self.config.bindings.iter().find(|b| b.id == binding_id) else { return };
                let (dref, roles) = (b.device.clone(), b.roles.unwrap_or(self.config.default_roles));
                let res = Owned::from(resolve::resolve(&dref, &self.endpoints));
                self.follow(&dref, res, roles);
            }
            Command::TogglePair { pair_id } => {
                let Some(p) = self.config.toggle_pairs.iter().find(|p| p.id == pair_id) else { return };
                let roles = p.roles.unwrap_or(self.config.default_roles);
                let pair = [p.primary.clone(), p.secondary.clone()];
                let current = self.default_id(pair[0].flow, Role::Multimedia);
                let (side, res) = match resolve::toggle([&pair[0], &pair[1]], current.as_deref(), &self.endpoints) {
                    ToggleAction::SwitchTo { side, resolution } | ToggleAction::Unavailable { side, resolution } => {
                        (side, Owned::from(resolution))
                    }
                };
                self.follow(&pair[side], res, roles);
            }
            Command::CycleFavourites { flow, step } => self.cycle(flow, i32::from(step)),
            Command::SetVolume { percent } => self.with_default_volume(|dev, _, _| {
                let _ = volume::set_scalar(dev, f32::from(percent) / 100.0);
            }),
            Command::StepVolume { delta } => self.with_default_volume(|dev, scalar, _| {
                let _ = volume::set_scalar(dev, scalar + f32::from(delta) / 100.0);
            }),
            Command::ToggleMute => self.with_default_volume(|dev, _, muted| {
                let _ = volume::set_mute(dev, !muted);
            }),
            Command::ToggleMicMute => {
                if let Some(dev) = self.mic_device()
                    && let Ok((scalar, muted)) = volume::get(&dev)
                    && volume::set_mute(&dev, !muted).is_ok()
                {
                    self.on_mic(scalar, !muted);
                }
            }
            Command::SetSpatial { device_id, mode } => {
                if let Some(target) = self.endpoints.iter().find(|e| e.id.eq_ignore_ascii_case(&device_id)).cloned() {
                    self.apply_spatial(&target, mode);
                    self.refresh(true);
                }
            }
            Command::ResolveChoice { request_id, device_id } => {
                let Some(p) = self.pending.remove(&request_id) else { return };
                let target =
                    device_id.and_then(|id| self.endpoints.iter().find(|e| e.id.eq_ignore_ascii_case(&id)).cloned());
                if let Some(target) = target {
                    // The user confirmed this device, so the re-link is persisted after the switch.
                    self.switch(&target, p.roles, Some(p.dref));
                }
            }
            Command::Refresh => self.refresh(true),
            Command::Media { action } => match &self.media {
                Some(m) => m.send(action),
                None => tracing::debug!(?action, "media command ignored: now playing is unavailable"),
            },
            Command::SetTogglePair { .. }
            | Command::RemoveTogglePair { .. }
            | Command::SetBinding { .. }
            | Command::RemoveBinding { .. }
            | Command::SetFavourites { .. }
            | Command::SetDevicePrefs { .. }
            | Command::SetPerformanceProfile { .. }
            | Command::SetPinnedMonitor { .. }
            | Command::SetKnobGestures { .. }
            | Command::SetTheme { .. }
            | Command::SetPreset { .. }
            | Command::SetIdleStyle { .. }
            | Command::SetNotchStyle { .. }
            | Command::SetNowPlayingPill { .. }
            | Command::SetMovingBackdrop { .. }
            | Command::SetHibernate { .. }
            | Command::SetHibernateHideFromCapture { .. }
            | Command::SetShowDisconnected { .. }
            | Command::SetIdleLinger { .. }
            | Command::SetMusicService { .. }
            | Command::SetLyricsEnabled { .. }
            | Command::SetLyricsOnlineLookup { .. }
            | Command::SetLyricsDisplay { .. }
            | Command::SetLyricsCacheMaxMb { .. }
            | Command::DisableAllNetworkFeatures
            | Command::SetVisualizerSpectrum { .. }
            | Command::SetShowNotchHotkey { .. }
            | Command::SetDefaultRoles { .. } => self.edit_settings(&command),
        }
    }

    pub(super) fn with_default_volume(&self, f: impl FnOnce(&IMMDevice, f32, bool)) {
        if let Ok(dev) = self.eps.default_device(Flow::Render, Role::Multimedia)
            && let Ok((scalar, muted)) = volume::get(&dev)
        {
            f(&dev, scalar, muted);
        }
    }

    /// Acts on a resolution of a saved reference.
    pub(super) fn follow(&mut self, dref: &DeviceRef, res: Owned, roles: RoleSet) {
        match res {
            Owned::Exact(t) => self.switch(&t, roles, None),
            Owned::Relinked(t) => self.switch(&t, roles, Some(dref.clone())),
            Owned::Choose => {
                // Same rules as the resolver (review ENG-06).
                let candidates: Vec<DeviceSummary> =
                    resolve::candidates(dref, &self.endpoints).into_iter().map(|e| self.summary_basic(e)).collect();
                let request_id = self.next_request;
                self.next_request += 1;
                self.pending.insert(request_id, PendingChoice { dref: dref.clone(), roles });
                while self.pending.len() > MAX_PENDING_CHOICES {
                    self.pending.pop_first();
                }
                self.notice(Notice::ChooseDevice { request_id, name: dref.display_name.clone(), candidates });
            }
            Owned::NotConnected => self.notice(Notice::NotConnected { name: dref.display_name.clone() }),
        }
    }

    pub(super) fn cycle(&mut self, flow: Flow, steps: i32) {
        let refs = match flow {
            Flow::Render => &self.config.favourites.playback,
            Flow::Capture => &self.config.favourites.recording,
        };
        let mut targets: Vec<EndpointInfo> =
            refs.iter().filter_map(|r| resolve::resolve(r, &self.endpoints).target().cloned()).collect();
        if targets.is_empty() {
            targets = self.endpoints.iter().filter(|e| e.flow == flow).cloned().collect();
        }
        targets.dedup_by(|a, b| a.id == b.id);
        if targets.is_empty() {
            return;
        }
        let n = i32::try_from(targets.len()).unwrap_or(1);
        let current = self.default_id(flow, Role::Multimedia);
        let cur =
            targets.iter().position(|t| Some(&t.id) == current.as_ref()).map_or(-1, |i| i32::try_from(i).unwrap_or(0));
        let next = usize::try_from((cur + steps).rem_euclid(n)).unwrap_or(0);
        let target = targets[next].clone();
        if Some(&target.id) == current.as_ref() {
            return; // already there (e.g. only one favourite connected)
        }
        self.switch(&target, self.config.default_roles, None);
    }

    /// Switches, confirms by re-reading the defaults, persists a re-link only after confirmation (D-16),
    /// then applies the device's spatial preference (D-17).
    /// `relink_from`: the saved reference that followed the device to `target`, if this is a re-link.
    pub(super) fn switch(&mut self, target: &EndpointInfo, roles: RoleSet, relink_from: Option<DeviceRef>) {
        let name = self.summary_basic(target).name;
        let wanted = roles.roles();
        if let Err(e) = policy::set_default(&target.id, wanted) {
            tracing::warn!(error = %e, device = %name, "IPolicyConfig unavailable");
            self.notice(Notice::SwitchFailed { name, failed_roles: wanted.to_vec() });
            return;
        }
        let defaults = self.eps.defaults();
        let failed: Vec<Role> = wanted
            .iter()
            .copied()
            .filter(|r| !defaults.iter().any(|(f, role, id)| *f == target.flow && role == r && *id == target.id))
            .collect();
        if !failed.is_empty() {
            let all_failed = failed.len() == wanted.len();
            self.notice(Notice::SwitchFailed { name: name.clone(), failed_roles: failed });
            if all_failed {
                // Binding preserved: nothing is re-linked to a device we couldn't switch to.
                self.refresh(true);
                return;
            }
        }
        if let Some(old) = &relink_from {
            self.persist_relink(old, target);
        }
        let spatial_applied = self
            .config
            .prefs_for(target)
            .map(|p| p.spatial)
            .filter(|m| *m != SpatialMode::Keep)
            .and_then(|mode| self.apply_spatial(target, mode));
        self.refresh(true);
        let device = {
            let sh = lock(&self.shared);
            sh.snapshot.playback.iter().chain(&sh.snapshot.recording).find(|d| d.id == target.id).cloned()
        }
        .unwrap_or_else(|| self.summary_basic(target));
        self.emit(EngineEvent::Switched { device, roles, relinked: relink_from.is_some(), spatial_applied });
    }

    /// Read-before-write, bounded write, read back. Returns the mode only when Windows confirms it (D-17).
    pub(super) fn apply_spatial(&mut self, target: &EndpointInfo, mode: SpatialMode) -> Option<SpatialMode> {
        let wanted = format_of(mode)?;
        if target.flow != Flow::Render {
            return None;
        }
        let selected = |id: &str| spatial::selected(id).ok().map(|s| s.and_then(|(f, _)| f));
        let is_wanted = |sel: Option<Option<SpatialFormat>>| sel == Some(wanted);
        if is_wanted(selected(&target.id)) {
            return Some(mode);
        }
        let status = spatial::set_default_within(&target.id, wanted, SPATIAL_WRITE_LIMIT);
        if is_wanted(selected(&target.id)) {
            return Some(mode);
        }
        let reason = match status {
            Ok(s) => s.to_string(),
            Err(e) => e,
        };
        let name = self.summary_basic(target).name;
        self.notice(Notice::SpatialUnavailable { name, mode, reason });
        None
    }
}

/// `None` = leave Windows alone; `Some(None)` = off; `Some(Some(f))` = that format.
fn format_of(mode: SpatialMode) -> Option<Option<SpatialFormat>> {
    Some(match mode {
        SpatialMode::Keep => return None,
        SpatialMode::Off => None,
        SpatialMode::WindowsSonic => Some(SpatialFormat::WindowsSonic),
        SpatialMode::DolbyAtmosHeadphones => Some(SpatialFormat::DolbyAtmosHeadphones),
        SpatialMode::DolbyAtmosHomeTheater => Some(SpatialFormat::DolbyAtmosHomeTheater),
        SpatialMode::DolbyAtmosSpeakers => Some(SpatialFormat::DolbyAtmosSpeakers),
        SpatialMode::DtsHeadphoneX => Some(SpatialFormat::DtsHeadphoneX),
        SpatialMode::DtsXUltra => Some(SpatialFormat::DtsXUltra),
        SpatialMode::DtsXHomeTheater => Some(SpatialFormat::DtsXHomeTheater),
    })
}
