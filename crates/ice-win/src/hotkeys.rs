//! Global hotkeys with `RegisterHotKey` on a dedicated message-loop thread.
//!
//! Never a keyboard hook (code-practices §1): Windows only tells us about the exact chords we
//! registered, and nothing else the user types.

use std::collections::VecDeque;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use ice_core::hotkey::Chord;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{HOT_KEY_MODIFIERS, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, WM_APP, WM_HOTKEY, WM_QUIT,
};

/// Posted to the hotkey thread when the request queue has work.
const WM_HOTKEY_REQUEST: u32 = WM_APP + 1;

enum Request {
    Register { id: i32, chord: Chord, repeat: bool, reply: Sender<Result<(), String>> },
    Unregister { id: i32 },
}

/// Owns the hotkey thread. Dropping it unregisters everything and stops the thread.
pub struct HotkeyService {
    thread_id: u32,
    queue: Arc<Mutex<VecDeque<Request>>>,
    join: Option<JoinHandle<()>>,
}

impl HotkeyService {
    /// Starts the thread. Each press of a registered chord sends its `id` on `pressed`.
    ///
    /// # Errors
    /// Fails if the thread can't be spawned.
    pub fn start(pressed: Sender<i32>) -> std::io::Result<Self> {
        let queue: Arc<Mutex<VecDeque<Request>>> = Arc::default();
        let (tid_tx, tid_rx) = channel();
        let q = Arc::clone(&queue);
        let join = std::thread::Builder::new().name("ice-hotkeys".into()).spawn(move || run(&q, &pressed, &tid_tx))?;
        let thread_id = tid_rx.recv().map_err(|_| std::io::Error::other("hotkey thread failed to start"))?;
        Ok(Self { thread_id, queue, join: Some(join) })
    }

    /// Registers `chord` under `id`. `repeat = false` ignores auto-repeat while held.
    ///
    /// # Errors
    /// A readable message, e.g. when another app already owns the chord (`HotkeyConflict` notice).
    pub fn register(&self, id: i32, chord: Chord, repeat: bool) -> Result<(), String> {
        let (reply, rx) = channel();
        self.push(Request::Register { id, chord, repeat, reply });
        rx.recv().unwrap_or_else(|_| Err("hotkey thread stopped".into()))
    }

    pub fn unregister(&self, id: i32) {
        self.push(Request::Unregister { id });
    }

    fn push(&self, r: Request) {
        self.queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push_back(r);
        // SAFETY: posting a plain message to a thread we created; its queue exists before `start` returns.
        let _ = unsafe { PostThreadMessageW(self.thread_id, WM_HOTKEY_REQUEST, WPARAM(0), LPARAM(0)) };
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        // SAFETY: as in `push`. WM_QUIT ends GetMessageW's loop; registrations die with the thread's cleanup below.
        let _ = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn run(queue: &Mutex<VecDeque<Request>>, pressed: &Sender<i32>, tid_tx: &Sender<u32>) {
    let mut msg = MSG::default();
    // SAFETY: PeekMessageW forces creation of this thread's message queue before we publish its ID,
    // so PostThreadMessageW from other threads can't race ahead of it.
    unsafe {
        let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
    }
    // SAFETY: trivial getter.
    let _ = tid_tx.send(unsafe { GetCurrentThreadId() });
    let mut registered = Vec::new();

    // SAFETY: standard message loop on this thread; `msg` is a valid out-pointer.
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        match msg.message {
            WM_HOTKEY => {
                let _ = pressed.send(i32::try_from(msg.wParam.0).unwrap_or(-1));
            }
            WM_HOTKEY_REQUEST => {
                let drained: Vec<Request> =
                    queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner).drain(..).collect();
                for req in drained {
                    match req {
                        Request::Register { id, chord, repeat, reply } => {
                            let mods = HOT_KEY_MODIFIERS(chord.mods.0 | if repeat { 0 } else { MOD_NOREPEAT.0 });
                            // SAFETY: thread-level hotkey (no window) owned by this message-loop thread.
                            let res = unsafe { RegisterHotKey(None, id, mods, chord.vk) };
                            if res.is_ok() {
                                registered.push(id);
                            }
                            let _ = reply.send(res.map_err(|e| format!("{chord} is unavailable ({})", e.message())));
                        }
                        Request::Unregister { id } => {
                            // SAFETY: unregisters a hotkey this thread registered (no-op error otherwise).
                            let _ = unsafe { UnregisterHotKey(None, id) };
                            registered.retain(|r| *r != id);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for id in registered {
        // SAFETY: same thread that registered them.
        let _ = unsafe { UnregisterHotKey(None, id) };
    }
}
