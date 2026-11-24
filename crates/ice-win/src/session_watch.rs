//! Session lock/unlock and sleep/resume notifications, event-driven on a hidden top-level window
//! (`WM_POWERBROADCAST` only reaches top-level windows, and `WTSRegisterSessionNotification` needs a window).
//! Used to clear the notch's session pin on lock or sleep (D-21.1 L3). No polling.

use core::ffi::c_void;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, MSG, PBT_APMRESUMEAUTOMATIC,
    PBT_APMSUSPEND, PostMessageW, PostQuitMessage, RegisterClassExW, TranslateMessage, WM_CLOSE, WM_DESTROY,
    WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
    WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
};
use windows::core::{Error, Result, w};

/// What happened to the user's session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    Locked,
    Unlocked,
    Suspending,
    Resumed,
}

type Sink = Box<dyn Fn(SessionEvent)>;

thread_local! {
    /// The sink for the window on this thread (the window procedure runs on the thread that created it).
    static SINK: RefCell<Option<Sink>> = const { RefCell::new(None) };
}

/// Running watcher; dropping it closes the window and joins the thread.
pub struct SessionWatcher {
    hwnd_raw: Arc<AtomicIsize>,
    join: Option<JoinHandle<()>>,
}

impl SessionWatcher {
    /// Starts the watcher thread. `sink` runs on it and must return quickly (e.g. dispatch to the main thread).
    ///
    /// # Errors
    /// Fails if the thread can't be spawned or the hidden window can't be created. A failed session
    /// registration is logged and leaves sleep/resume working.
    pub fn start(sink: impl Fn(SessionEvent) + Send + 'static) -> Result<Self> {
        let hwnd_raw = Arc::new(AtomicIsize::new(0));
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<()>>(1);
        let hwnd_thread = Arc::clone(&hwnd_raw);
        let join = std::thread::Builder::new()
            .name("ice-session-watch".into())
            .spawn(move || run(&hwnd_thread, &ready_tx, Box::new(sink)))
            .map_err(|_| Error::from_thread())?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { hwnd_raw, join: Some(join) }),
            Ok(Err(err)) => {
                let _ = join.join();
                Err(err)
            }
            Err(_) => {
                let _ = join.join();
                Err(Error::from_thread())
            }
        }
    }
}

impl Drop for SessionWatcher {
    fn drop(&mut self) {
        let raw = self.hwnd_raw.swap(0, Ordering::AcqRel);
        if raw != 0 {
            let hwnd = HWND(raw as *mut c_void);
            // SAFETY: `hwnd` belongs to `ice-session-watch` and stays valid until destroyed there.
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Maps a window message to a session event (pure, so it's testable).
fn classify(umsg: u32, wparam: usize) -> Option<SessionEvent> {
    match umsg {
        WM_WTSSESSION_CHANGE if wparam == WTS_SESSION_LOCK as usize => Some(SessionEvent::Locked),
        WM_WTSSESSION_CHANGE if wparam == WTS_SESSION_UNLOCK as usize => Some(SessionEvent::Unlocked),
        WM_POWERBROADCAST if wparam == PBT_APMSUSPEND as usize => Some(SessionEvent::Suspending),
        WM_POWERBROADCAST if wparam == PBT_APMRESUMEAUTOMATIC as usize => Some(SessionEvent::Resumed),
        _ => None,
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if let Some(event) = classify(umsg, wparam.0) {
        SINK.with_borrow(|sink| {
            if let Some(sink) = sink {
                sink(event);
            }
        });
    }
    match umsg {
        WM_CLOSE => {
            // SAFETY: unregisters and destroys only our own hidden window, on its owning thread.
            unsafe {
                let _ = WTSUnRegisterSessionNotification(hwnd);
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: standard default processing (also answers WM_POWERBROADCAST with TRUE).
        _ => unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) },
    }
}

fn create_window() -> Result<HWND> {
    let class_name = w!("IceSessionWatcherClass");
    // SAFETY: `GetModuleHandleW(None)` returns the current executable's HINSTANCE.
    let hinstance = unsafe { GetModuleHandleW(None) }?;
    let wc = WNDCLASSEXW {
        cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(0),
        lpfnWndProc: Some(wnd_proc),
        hInstance: hinstance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    // SAFETY: registering an immutable static class name; repeated registration is benign.
    unsafe { RegisterClassExW(&raw const wc) };
    // Top-level, never shown (not HWND_MESSAGE): power broadcasts only reach top-level windows.
    // SAFETY: creates a private 0×0 tool window owned by this thread.
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name,
            w!("IceSessionWatcher"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
    }
}

fn run(hwnd_raw: &AtomicIsize, ready_tx: &mpsc::SyncSender<Result<()>>, sink: Sink) {
    SINK.set(Some(sink));
    let hwnd = match create_window() {
        Ok(h) => h,
        Err(err) => {
            let _ = ready_tx.send(Err(err));
            return;
        }
    };
    // SAFETY: registers our own window for this session's lock/unlock notifications.
    if let Err(error) = unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) } {
        tracing::warn!(%error, "session lock notifications unavailable; sleep/resume still watched");
    }
    hwnd_raw.store(hwnd.0 as isize, Ordering::Release);
    let _ = ready_tx.send(Ok(()));

    let mut msg = MSG::default();
    // SAFETY: standard message loop on this thread; `msg` is a valid out-pointer. 0 = WM_QUIT, -1 = error.
    while unsafe { GetMessageW(&raw mut msg, None, 0, 0) }.0 > 0 {
        // SAFETY: `msg` came from GetMessageW on this thread.
        unsafe {
            let _ = TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);
        }
    }
    SINK.set(None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_lock_and_sleep_messages_only() {
        assert_eq!(classify(WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK as usize), Some(SessionEvent::Locked));
        assert_eq!(classify(WM_WTSSESSION_CHANGE, WTS_SESSION_UNLOCK as usize), Some(SessionEvent::Unlocked));
        assert_eq!(classify(WM_POWERBROADCAST, PBT_APMSUSPEND as usize), Some(SessionEvent::Suspending));
        assert_eq!(classify(WM_POWERBROADCAST, PBT_APMRESUMEAUTOMATIC as usize), Some(SessionEvent::Resumed));
        assert_eq!(classify(WM_CLOSE, 0), None);
    }

    #[test]
    fn watcher_starts_and_drops_cleanly() {
        let watcher = SessionWatcher::start(|_| {}).expect("session watcher");
        drop(watcher);
    }
}
