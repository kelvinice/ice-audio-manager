//! Windows shell lifetime and notification-area menu integration.

use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_NULL};
use windows::core::{Result, w};

/// Holding the handle keeps the session-local mutex alive until the shell exits.
pub struct InstanceGuard(HANDLE);

impl InstanceGuard {
    pub fn acquire() -> Result<Option<Self>> {
        // SAFETY: use the default security descriptor and a static name in the session namespace.
        // The handle keeps the mutex alive without taking ownership of the mutex itself.
        let handle = unsafe { CreateMutexW(None, false, w!("Local\\IceAudioManager.Desktop")) }?;
        // SAFETY: capture the status from CreateMutexW before calling another Windows API.
        let already_running = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let guard = Self(handle);
        Ok((!already_running).then_some(guard))
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // SAFETY: this object is the sole owner of the handle returned by CreateMutexW.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Windows notification-area menus need this posted after the menu closes.
pub fn finish_tray_menu(owner: isize) -> Result<()> {
    // SAFETY: this sends a benign message with no pointer payload to the shell's menu owner.
    unsafe { PostMessageW(Some(HWND(owner as _)), WM_NULL, WPARAM(0), LPARAM(0)) }
}
