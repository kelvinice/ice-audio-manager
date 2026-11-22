//! Event-driven directory watching for config hot reload. Windows wakes us; we never poll (code-practices §2).

use std::path::Path;
use std::thread::JoinHandle;

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FindCloseChangeNotification,
    FindFirstChangeNotificationW, FindNextChangeNotification,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects};
use windows::core::{HSTRING, PCWSTR, Result};

/// Calls `on_change` (on its own thread) whenever a file in `dir` is written, created, renamed or deleted.
/// Callers debounce and re-read; one save often produces several notifications.
pub struct DirWatch {
    /// Manual-reset event that stops the thread. Stored as an integer because `HANDLE` isn't `Send`.
    stop: isize,
    join: Option<JoinHandle<()>>,
}

impl DirWatch {
    /// # Errors
    /// Fails if `dir` doesn't exist or can't be watched.
    pub fn start(dir: &Path, on_change: impl Fn() + Send + 'static) -> Result<Self> {
        // SAFETY: plain kernel-object creation; both handles are closed by this type (Drop / thread exit).
        let (change, stop) = unsafe {
            let change = FindFirstChangeNotificationW(
                &HSTRING::from(dir.as_os_str()),
                false,
                FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_FILE_NAME,
            )?;
            let stop = match CreateEventW(None, true, false, PCWSTR::null()) {
                Ok(h) => h,
                Err(e) => {
                    let _ = FindCloseChangeNotification(change);
                    return Err(e);
                }
            };
            (change.0 as isize, stop.0 as isize)
        };
        let join = std::thread::Builder::new()
            .name("ice-config-watch".into())
            .spawn(move || {
                let handles = [HANDLE(change as *mut _), HANDLE(stop as *mut _)];
                loop {
                    // SAFETY: both handles stay valid until this loop ends (stop is closed only after join).
                    let woke = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
                    // SAFETY: re-arms the change handle we own.
                    if woke != WAIT_OBJECT_0 || unsafe { FindNextChangeNotification(handles[0]) }.is_err() {
                        break;
                    }
                    on_change();
                }
                // SAFETY: closes the change handle created above, exactly once.
                let _ = unsafe { FindCloseChangeNotification(handles[0]) };
            })
            .map_err(|e| {
                // SAFETY: the thread never started, so we still own both handles.
                unsafe {
                    let _ = FindCloseChangeNotification(HANDLE(change as *mut _));
                    let _ = CloseHandle(HANDLE(stop as *mut _));
                }
                windows::core::Error::new(windows::Win32::Foundation::E_FAIL, e.to_string())
            })?;
        Ok(Self { stop, join: Some(join) })
    }
}

impl Drop for DirWatch {
    fn drop(&mut self) {
        let stop = HANDLE(self.stop as *mut _);
        // SAFETY: signals and then closes the event we created; the thread has exited before CloseHandle.
        unsafe {
            let _ = SetEvent(stop);
        }
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        // SAFETY: see above.
        let _ = unsafe { CloseHandle(stop) };
    }
}
