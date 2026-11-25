//! D-19: one "show the notch" action for the "show notch" hotkey and the native pill's click.
//!
//! - `Instant`/`Balanced`: expand (pin) the notch and wake or create its WebView. The hotkey toggles, so a second
//!   press closes it; a pill click only opens.
//! - `Lite` (no notch WebView): the tray quick-switch menu instead.
//! - A fullscreen app covers the notch's monitor (gaming mode): nothing, so a game never gets a popup.

use std::sync::Arc;

use ice_core::engine::{Command, RoleSet};
use ice_win::native_dot::{NativeDotHandle, PillInput};
use tauri::AppHandle;

use super::{Preview, SharedRuntime, request_preview, tray};

/// Volume change per wheel notch over the native pill. Twin: `onWheel` (`step = 4`) in app/ui/src/lib/notch/controller.svelte.ts.
const WHEEL_VOLUME_STEP: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Open {
    /// The hotkey: open, or close if already open.
    Toggle,
    /// The pill click / hover: open.
    Show,
}

/// What a request does, decided from the placement alone (pure, so it's testable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Nothing,
    QuickMenu,
    Expand(bool),
}

fn decide(allows_webview: bool, gaming: bool, expanded: bool, open: Open) -> Outcome {
    if gaming {
        Outcome::Nothing
    } else if !allows_webview {
        Outcome::QuickMenu
    } else {
        Outcome::Expand(open == Open::Show || !expanded)
    }
}

/// Runs on the main thread (engine bridge and pill input both dispatch there).
pub(super) fn show(app: &AppHandle, state: &Arc<SharedRuntime>, open: Open) {
    let (allows_webview, gaming) = {
        let p = state.placement();
        (p.allows_webview(), p.gaming)
    };
    match decide(allows_webview, gaming, state.snapshot_clone().expanded, open) {
        Outcome::Nothing => {}
        Outcome::QuickMenu => tray::open_quick_menu(app, state),
        Outcome::Expand(expand) => {
            state.update_and_emit(app, |s| {
                s.expanded = expand;
                s.event_seq = s.event_seq.saturating_add(1);
            });
            if expand {
                // Creates the WebView if it's gone, or wakes it if Balanced suspended it; a no-op when visible.
                request_preview(app, state, Preview::Notch);
            }
        }
    }
}

/// D-19 shell wiring: the native pill's click/hover opens the notch (or the quick-switch menu in Lite), and a
/// middle click toggles mute like the notch. Input arrives on the pill's thread and is moved to the main thread.
pub(super) fn wire_native_pill(app: &AppHandle, state: &Arc<SharedRuntime>, dot: &NativeDotHandle) {
    let app = app.clone();
    let state = Arc::clone(state);
    dot.set_input_handler(Arc::new(move |input| {
        match input {
            PillInput::MiddleClick => {
                state.engine.send(Command::ToggleMute);
                return;
            }
            // Like the WebView notch: ±4 % per notch, Shift switches to the next device.
            PillInput::Wheel { notches, shift: true } => {
                if notches != 0
                    && let Some(target) = super::engine_bridge::preview_target(&state.engine.snapshot())
                {
                    state.engine.send(Command::SwitchTo { device_id: target.id.clone(), roles: RoleSet::Default });
                }
                return;
            }
            PillInput::Wheel { notches, shift: false } => {
                let delta = i8::try_from(i32::from(notches).saturating_mul(WHEEL_VOLUME_STEP))
                    .unwrap_or(if notches > 0 { i8::MAX } else { i8::MIN });
                state.engine.send(Command::StepVolume { delta });
                return;
            }
            PillInput::DragStart => {
                super::drag::start(&app, &state, super::drag::Host::Native);
                return;
            }
            PillInput::Click | PillInput::HoverOpen => {}
        }
        let (handle, st) = (app.clone(), Arc::clone(&state));
        let _ = app.run_on_main_thread(move || show(&handle, &st, Open::Show));
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hotkey_toggles_and_the_pill_only_opens() {
        assert_eq!(decide(true, false, false, Open::Toggle), Outcome::Expand(true));
        assert_eq!(decide(true, false, true, Open::Toggle), Outcome::Expand(false));
        assert_eq!(decide(true, false, true, Open::Show), Outcome::Expand(true));
    }

    #[test]
    fn lite_opens_the_quick_menu_and_games_get_nothing() {
        assert_eq!(decide(false, false, false, Open::Show), Outcome::QuickMenu);
        assert_eq!(decide(false, true, false, Open::Toggle), Outcome::Nothing);
        assert_eq!(decide(true, true, false, Open::Toggle), Outcome::Nothing);
    }
}
