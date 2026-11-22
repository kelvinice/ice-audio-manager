//! Event-driven directory watching for config hot reload. Windows wakes us; we never poll (code-practices §2).

use std::path::Path;
use std::thread::JoinHandle;

use windows::Win32::Foundation::{CloseHandle, E_FAIL, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FindCloseChangeNotification,
    FindFirstChangeNotificationW, FindNextChangeNotification,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects};
use windows::core::{HSTRING, PCWSTR, Result};

/// Owns a change-notification handle; closes it even if the callback panics (review ENG-09).
struct ChangeHandle(HANDLE);

impl Drop for ChangeHandle {
    fn drop(&mut self) {
        // SAFETY: closes a handle from FindFirstChangeNotificationW exactly once.
        let _ = unsafe { FindCloseChangeNotification(self.0) };
    }
}

/// Owns an event handle.
struct EventHandle(HANDLE);

impl Drop for EventHandle {
    fn drop(&mut self) {
        // SAFETY: closes a handle from CreateEventW exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Calls `on_change` (on its own thread) whenever a file in `dir` is written, created, renamed or deleted.
/// Callers debounce and re-read; one save often produces several notifications.
pub struct DirWatch {
    stop: EventHandle,
    join: Option<JoinHandle<()>>,
}

impl DirWatch {
    /// # Errors
    /// Fails if `dir` doesn't exist or can't be watched.
    pub fn start(dir: &Path, on_change: impl Fn() + Send + 'static) -> Result<Self> {
        // SAFETY: plain kernel-object creation; each handle is owned by an RAII guard from here on.
        let change = ChangeHandle(unsafe {
            FindFirstChangeNotificationW(
                &HSTRING::from(dir.as_os_str()),
                false,
                FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_FILE_NAME,
            )?
        });
        // SAFETY: as above.
        let stop = EventHandle(unsafe { CreateEventW(None, true, false, PCWSTR::null())? });
        // HANDLE isn't Send; pass the raw values. `stop` outlives the thread (joined in Drop before closing).
        let (change_raw, stop_raw) = (change.0.0 as isize, stop.0.0 as isize);
        std::mem::forget(change); // ownership moves into the thread's guard below
        let join = std::thread::Builder::new()
            .name("ice-config-watch".into())
            .spawn(move || {
                let change = ChangeHandle(HANDLE(change_raw as *mut _));
                let handles = [change.0, HANDLE(stop_raw as *mut _)];
                loop {
                    // SAFETY: both handles stay valid for the whole loop.
                    let woke = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
                    // SAFETY: re-arms the change handle we own.
                    if woke != WAIT_OBJECT_0 || unsafe { FindNextChangeNotification(change.0) }.is_err() {
                        break;
                    }
                    on_change();
                }
            })
            .map_err(|e| {
                // SAFETY: the thread never started, so the change handle is still ours to close.
                drop(ChangeHandle(HANDLE(change_raw as *mut _)));
                windows::core::Error::new(E_FAIL, e.to_string())
            })?;
        Ok(Self { stop, join: Some(join) })
    }
}

impl Drop for DirWatch {
    fn drop(&mut self) {
        // SAFETY: signals the event we own; the thread exits and closes its change handle.
        unsafe {
            let _ = SetEvent(self.stop.0);
        }
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        // `self.stop` closes after the join.
    }
}
