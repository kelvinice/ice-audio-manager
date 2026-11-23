//! F17: a notch document requests frames, but the shell owns the subscription lifetime.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, PoisonError, Weak};

use ice_core::meter::{METER_INTERVAL, MeterFrame, MeterSubscription};
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

use super::SharedRuntime;

#[derive(Default)]
pub(super) struct Visualizer {
    requested: bool,
    // Set only after the shell can observe renderer loss, even during silence.
    monitored: bool,
    stop: Weak<AtomicBool>,
}

fn meter_wanted(requested: bool, visible: bool, suspended: bool, webview: bool, allowed: bool) -> bool {
    requested && visible && !suspended && webview && allowed
}

#[tauri::command]
pub(super) fn set_visualizer(active: bool, app: AppHandle, state: State<'_, Arc<SharedRuntime>>) {
    let mut meter = state.meter.lock().unwrap_or_else(PoisonError::into_inner);
    meter.requested = active;
    meter.reconcile(&app, &state);
}

pub(super) fn reconcile(app: &AppHandle, state: &SharedRuntime) {
    state.meter.lock().unwrap_or_else(PoisonError::into_inner).reconcile(app, state);
}

impl Visualizer {
    fn reconcile(&mut self, app: &AppHandle, state: &SharedRuntime) {
        let wanted = meter_wanted(
            self.requested,
            state.notch_visible.load(Ordering::Acquire),
            state.want_suspended.load(Ordering::Acquire) || state.notch_suspended.load(Ordering::Acquire),
            self.monitored && state.notch_hwnd.load(Ordering::Acquire) != 0,
            state.placement().allows_webview(),
        );
        if !wanted {
            self.stop();
            return;
        }
        if self.stop.upgrade().is_some() {
            return;
        }
        let Some(subscription) = state.engine.current().and_then(|engine| engine.meter()) else { return };
        let stop = Arc::new(AtomicBool::new(false));
        self.stop = Arc::downgrade(&stop);
        let app = app.clone();
        if let Err(error) = std::thread::Builder::new().name("ice-notch-meter".into()).spawn(move || {
            tracing::debug!("Notch visualizer subscribed");
            forward(subscription, &stop, |frame| app.emit_to("notch", "audio://meter/v1", frame));
        }) {
            tracing::warn!(%error, "Could not start notch meter forwarder");
        }
    }

    fn stop(&mut self) {
        // Never join on the UI thread: Tauri may still be dispatching the last emit.
        if let Some(stop) = std::mem::take(&mut self.stop).upgrade() {
            stop.store(true, Ordering::Release);
        }
    }
}

fn forward(subscription: MeterSubscription, stop: &AtomicBool, emit: impl Fn(&MeterFrame) -> tauri::Result<()>) {
    while !stop.load(Ordering::Acquire) {
        match subscription.recv_timeout(METER_INTERVAL) {
            Ok(frame) if !stop.load(Ordering::Acquire) => {
                if let Err(error) = emit(&frame) {
                    tracing::debug!(%error, "Notch meter receiver unavailable");
                    break;
                }
            }
            Ok(_) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

pub(super) fn page_load(win: WebviewWindow, payload: PageLoadPayload<'_>) {
    if matches!(payload.event(), PageLoadEvent::Started) {
        forget_document(&win.state::<Arc<SharedRuntime>>());
    }
}

fn forget_document(state: &SharedRuntime) {
    let mut meter = state.meter.lock().unwrap_or_else(PoisonError::into_inner);
    meter.requested = false;
    meter.stop();
}

pub(super) fn released(state: &SharedRuntime) {
    let mut meter = state.meter.lock().unwrap_or_else(PoisonError::into_inner);
    meter.stop();
    *meter = Visualizer::default();
}

pub(super) fn watch_renderer(win: &WebviewWindow, state: &Arc<SharedRuntime>) -> tauri::Result<()> {
    let state = Arc::clone(state);
    let app = win.app_handle().clone();
    win.with_webview(move |wv| {
        let weak = Arc::downgrade(&state);
        match ice_win::webview2::on_process_exit(&wv.controller(), move || {
            if let Some(state) = weak.upgrade() {
                forget_document(&state);
            }
        }) {
            Ok(()) => {
                let mut meter = state.meter.lock().unwrap_or_else(PoisonError::into_inner);
                meter.monitored = true;
                meter.reconcile(&app, &state);
            }
            Err(error) => tracing::warn!(%error, "Notch renderer monitoring unavailable; visualizer disabled"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_five_gates_must_be_open() {
        for mask in 0_u8..32 {
            let open = |bit: u8| mask & (1 << bit) != 0;
            assert_eq!(meter_wanted(open(0), open(1), !open(2), open(3), open(4)), mask == 31);
        }
    }

    #[test]
    fn stops_during_silence_and_on_engine_disconnect() {
        for disconnect in [false, true] {
            let (mut sink, subscription) = ice_core::meter::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let (frames, received) = std::sync::mpsc::channel();
            let worker = {
                let stop = Arc::clone(&stop);
                std::thread::spawn(move || {
                    forward(subscription, &stop, |frame| {
                        frames.send(frame.clone()).unwrap();
                        Ok(())
                    })
                })
            };
            let frame = MeterFrame::new(0.5, vec![0.5, 0.3]);
            sink.offer(&frame);
            assert_eq!(received.recv_timeout(std::time::Duration::from_secs(2)).unwrap(), frame);
            if disconnect {
                drop(sink);
            } else {
                stop.store(true, Ordering::Release);
            }
            assert_eq!(
                received.recv_timeout(std::time::Duration::from_millis(100)),
                Err(RecvTimeoutError::Disconnected)
            );
            worker.join().unwrap();
        }
    }
}
