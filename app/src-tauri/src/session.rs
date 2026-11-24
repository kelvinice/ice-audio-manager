//! D-21.1 L3: the notch's session pin ("keep open until unpinned") clears when the session locks or the PC sleeps,
//! so a pinned notch never greets the user expanded after they come back. Event-driven via
//! `ice_win::session_watch`; gaming mode already resets the pin in `placement`.

use std::sync::{Arc, Mutex, PoisonError};

use ice_win::session_watch::{SessionEvent, SessionWatcher};
use tauri::{AppHandle, Manager};

use super::SharedRuntime;

// Kept alive until Exit, then dropped there so the watcher thread ends before the runtime does.
type Watcher = Mutex<Option<SessionWatcher>>;

pub(super) fn watch(app: &AppHandle, state: &Arc<SharedRuntime>) {
    let handle = app.clone();
    let shared = Arc::clone(state);
    match SessionWatcher::start(move |event| {
        if !clears_pin(event) {
            return;
        }
        let app = handle.clone();
        let state = Arc::clone(&shared);
        let _ = handle.run_on_main_thread(move || unpin(&app, &state));
    }) {
        Ok(watcher) => {
            app.manage(Watcher::new(Some(watcher)));
        }
        Err(error) => tracing::warn!(%error, "Session lock/sleep watcher unavailable; the notch pin won't auto-clear"),
    }
}

pub(super) fn stop(app: &AppHandle) {
    if let Some(watcher) = app.try_state::<Watcher>() {
        watcher.lock().unwrap_or_else(PoisonError::into_inner).take();
    }
}

fn clears_pin(event: SessionEvent) -> bool {
    matches!(event, SessionEvent::Locked | SessionEvent::Suspending)
}

fn unpin(app: &AppHandle, state: &Arc<SharedRuntime>) {
    if !state.snapshot_clone().expanded {
        return;
    }
    tracing::debug!("Clearing the notch pin: session locked or going to sleep");
    state.update_and_emit(app, |s| {
        s.expanded = false;
        s.event_seq = s.event_seq.saturating_add(1);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_lock_and_sleep_clear_the_pin() {
        assert!(clears_pin(SessionEvent::Locked));
        assert!(clears_pin(SessionEvent::Suspending));
        assert!(!clears_pin(SessionEvent::Unlocked));
        assert!(!clears_pin(SessionEvent::Resumed));
    }
}
