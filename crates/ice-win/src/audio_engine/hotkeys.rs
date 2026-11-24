//! `RegisterHotKey` bindings, toggle pairs, the favourites cycle and the F31 knob gestures.

use super::*;

impl Worker {
    pub(super) fn register_hotkeys(&mut self) {
        self.hotkey_conflicts.clear();
        let mut wanted: Vec<(Chord, HotkeyAction, String, bool)> = Vec::new();
        for b in &self.config.bindings {
            if let Some(c) = b.hotkey {
                wanted.push((c, HotkeyAction::Binding(b.id.clone()), b.id.clone(), false));
            }
        }
        for p in &self.config.toggle_pairs {
            if let Some(c) = p.hotkey {
                wanted.push((c, HotkeyAction::Pair(p.id.clone()), p.id.clone(), false));
            }
        }
        if let Some(c) = self.config.favourites.cycle_hotkey {
            wanted.push((c, HotkeyAction::CycleFavourites, "favourites".into(), false));
        }
        if let Some(c) = self.config.show_notch_hotkey {
            wanted.push((c, HotkeyAction::ShowNotch, "show-notch".into(), false));
        }
        wanted.extend(knob_chords(&self.config.knob));
        if wanted.is_empty() {
            return;
        }
        let tx = self.engine_tx.clone();
        let svc = match HotkeyService::start(move |id| {
            let _ = tx.send(Msg::Hotkey(id));
        }) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "hotkey service failed to start");
                return;
            }
        };
        for (id, (chord, action, name, repeat)) in (1..).zip(wanted) {
            match svc.register(id, chord, repeat) {
                Ok(()) => {
                    self.actions.insert(id, action);
                }
                Err(e) => {
                    tracing::warn!(%chord, error = %e, "hotkey unavailable");
                    let notice = Notice::HotkeyConflict { binding_id: name, keys: chord.to_string() };
                    self.hotkey_conflicts.push(notice.clone());
                    self.notice(notice);
                }
            }
        }
        self.hotkeys = Some(svc);
    }

    pub(super) fn hotkey(&mut self, id: i32) {
        let Some(action) = self.actions.get(&id).cloned() else { return };
        match action {
            HotkeyAction::Binding(binding_id) => self.command(Command::SwitchBinding { binding_id }),
            HotkeyAction::Pair(pair_id) => self.command(Command::TogglePair { pair_id }),
            HotkeyAction::CycleFavourites => self.cycle(Flow::Render, 1),
            HotkeyAction::ShowNotch => self.emit(EngineEvent::ShowNotch),
            HotkeyAction::KnobCycle(step) => {
                // Alt+Tab style: one switch per gesture, after the knob rests (F31; fast spins = 13–40 ms/detent).
                if self.knob_at.is_none() {
                    self.knob_first = step.signum();
                }
                self.knob_steps += step;
                self.knob_at =
                    Some(Instant::now() + Duration::from_millis(u64::from(self.config.knob.commit_after_ms)));
            }
            HotkeyAction::KnobFine(delta) => self.command(Command::StepVolume { delta }),
            HotkeyAction::KnobMic(delta) => {
                if let Some(dev) = self.mic_device()
                    && let Ok((scalar, muted)) = volume::get(&dev)
                {
                    let target = (scalar + delta).clamp(0.0, 1.0);
                    if volume::set_scalar(&dev, target).is_ok() {
                        self.on_mic(target, muted); // immediate feedback; the watch callback dedupes
                    }
                }
            }
        }
    }
}

/// F31 chords: `<modifier>+VolumeUp/Down` for each enabled gesture.
fn knob_chords(k: &KnobGestures) -> Vec<(Chord, HotkeyAction, String, bool)> {
    if !k.enabled {
        return Vec::new();
    }
    let fine = i8::try_from(k.fine_step_percent).unwrap_or(1);
    let gestures: [(&Option<String>, HotkeyAction, HotkeyAction); 3] = [
        (&k.cycle_device_modifier, HotkeyAction::KnobCycle(1), HotkeyAction::KnobCycle(-1)),
        (&k.fine_volume_modifier, HotkeyAction::KnobFine(fine), HotkeyAction::KnobFine(-fine)),
        (&k.mic_volume_modifier, HotkeyAction::KnobMic(MIC_STEP), HotkeyAction::KnobMic(-MIC_STEP)),
    ];
    let mut out = Vec::new();
    for (modifier, up, down) in gestures {
        let Some(m) = modifier else { continue };
        for (key, action) in [("VolumeUp", up), ("VolumeDown", down)] {
            if let Ok(chord) = format!("{m}+{key}").parse::<Chord>() {
                out.push((chord, action, format!("knob {m}+{key}"), true));
            }
        }
    }
    out
}
