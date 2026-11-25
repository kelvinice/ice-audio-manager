//! config.toml: hot reload, re-link persistence, safe saves and the settings commands (code-practices §7).

use super::*;

impl Worker {
    /// Applies an edited config.toml. Invalid files are reported once and the last valid config stays in effect.
    pub(super) fn reload_config(&mut self) {
        let text = match self.store.read_text() {
            Ok(Some(t)) => t,
            Ok(None) => return, // deleted: keep running; the next save recreates it
            Err(e) => return self.config_problem(format!("config I/O error: {e}")),
        };
        if Some(&text) == self.disk_text.as_ref() {
            // Our own save, or the user put back the last good file: nothing to apply, but a reported problem is over.
            if self.config_invalid.take().is_some() {
                self.publish_notices();
            }
            return;
        }
        match Config::from_toml(&text) {
            Ok(cfg) => {
                self.disk_text = Some(text);
                self.config_invalid = None;
                self.config_replaced = None;
                self.adopt_disk_config(cfg);
            }
            Err(e) => self.config_problem(e.to_string()),
        }
    }

    pub(super) fn config_problem(&mut self, error: String) {
        let notice = Notice::ConfigInvalid { error };
        if self.config_invalid.as_ref() != Some(&notice) {
            self.config_invalid = Some(notice.clone());
            self.publish_notices();
            self.notice(notice);
        }
    }

    /// Applies a config re-read from disk (hot reload, or adopting an external edit before a save). Hibernate is
    /// session state, so the live value stays whatever the file says (D-27).
    fn adopt_disk_config(&mut self, mut cfg: Config) {
        cfg.keep_session_state(&self.config);
        self.set_config(cfg);
    }

    pub(super) fn set_config(&mut self, cfg: Config) {
        if cfg != self.config {
            let lyrics_changed = cfg.lyrics != self.config.lyrics;
            let spectrum_changed = cfg.visualizer_spectrum != self.config.visualizer_spectrum;
            let call_mode_changed = cfg.call_mode != self.config.call_mode;
            self.config = cfg;
            if call_mode_changed {
                self.sync_call_mode();
            }
            if lyrics_changed && let Some(lyrics) = &self.lyrics {
                lyrics.update_config(self.config.lyrics);
            }
            if spectrum_changed {
                lock(&self.shared).meter.set_spectrum_enabled(self.config.visualizer_spectrum);
            }
            self.pending.clear(); // pickers refer to the old bindings (review ENG-03)
            self.hotkeys = None; // dropping the service unregisters every chord
            self.actions.clear();
            self.register_hotkeys();
            self.refresh(true);
        }
        self.publish_notices();
    }

    /// D-30: the `ice-call` thread runs only while call mode is on. Turning it off drops the service, which puts the
    /// music back at its own level before the thread ends.
    pub(super) fn sync_call_mode(&mut self) {
        let config = &self.config.call_mode;
        match (&self.call_mode, config.enabled) {
            (Some(service), true) => service.set_config(config.clone()),
            (Some(_), false) => self.call_mode = None,
            (None, true) => {
                let now_playing = lock(&self.shared).snapshot.media.as_ref().map(|m| m.app_id.clone());
                self.call_mode = CallModeService::start(config.clone(), now_playing)
                    .inspect_err(|e| tracing::warn!(error = %e, "call mode thread failed to start"))
                    .ok();
            }
            (None, false) => {}
        }
    }

    /// After a confirmed switch to a re-created endpoint, points every reference to the old endpoint at the new
    /// one and saves. If the file changed on disk since we last read it, the user's version is adopted first;
    /// a half-edited (invalid) file is never overwritten (code-practices §7, review ENG-04/ENG-10).
    pub(super) fn persist_relink(&mut self, old: &DeviceRef, target: &EndpointInfo) {
        if !self.sync_with_disk("re-link") {
            return;
        }
        if self.config.relink_all(&old.endpoint_id, target) == 0 {
            return;
        }
        self.write_config();
        self.publish_settings(true);
    }

    /// Before any automatic save: if config.toml changed on disk since we last read or wrote it, adopt the
    /// user's version first. Returns `false` (and reports it) when the file is mid-edit and invalid, so it is
    /// never overwritten (code-practices §7).
    pub(super) fn sync_with_disk(&mut self, what: &str) -> bool {
        match self.store.read_text() {
            Ok(text) if text != self.disk_text => {
                if let Some(t) = text {
                    match Config::from_toml(&t) {
                        Ok(cfg) => {
                            self.disk_text = Some(t);
                            self.adopt_disk_config(cfg);
                        }
                        Err(_) => {
                            self.reload_at = Some(Instant::now() + RELOAD_DEBOUNCE);
                            self.save_problem(format!("config.toml is being edited and is invalid; {what} not saved"));
                            return false;
                        }
                    }
                }
                true
            }
            Ok(_) => true,
            Err(e) => {
                self.save_problem(e.to_string());
                false
            }
        }
    }

    /// Saves the in-memory config; a failure is reported and the change lasts until restart.
    pub(super) fn write_config(&mut self) {
        match self.store.save(&self.config) {
            Ok(text) => {
                self.disk_text = Some(text);
                if self.save_failed.take().is_some() {
                    self.publish_notices();
                }
            }
            Err(e) => self.save_problem(e.to_string()),
        }
    }

    /// A settings command from the tray/settings UI: validate against the live devices, save, apply.
    pub(super) fn edit_settings(&mut self, command: &Command) {
        if !self.sync_with_disk("setting") {
            return;
        }
        let mut cfg = self.config.clone();
        match cfg.apply_setting(command, &self.endpoints) {
            Ok(false) => {}
            Ok(true) => {
                let previous = std::mem::replace(&mut self.config, cfg.clone());
                self.write_config();
                // set_config compares against the current config, so put the old one back for it to diff.
                self.config = previous;
                self.set_config(cfg);
                self.publish_settings(true);
            }
            Err(error) => self.notice(Notice::SettingRejected { error }),
        }
    }

    /// Recomputes the settings view (it depends on which saved devices are connected); emits
    /// `SettingsChanged` only when it changed.
    pub(super) fn publish_settings(&mut self, emit: bool) {
        let view = self.config.view(&self.endpoints);
        let changed = {
            let mut sh = lock(&self.shared);
            let changed = sh.snapshot.settings != view;
            if changed {
                sh.snapshot.settings = view.clone();
            }
            changed
        };
        if changed && emit {
            self.emit(EngineEvent::SettingsChanged { settings: view });
        }
    }

    pub(super) fn save_problem(&mut self, error: String) {
        tracing::warn!(%error, "config save failed");
        let notice = Notice::ConfigSaveFailed { error };
        self.save_failed = Some(notice.clone());
        self.publish_notices();
        self.notice(notice);
    }
}
