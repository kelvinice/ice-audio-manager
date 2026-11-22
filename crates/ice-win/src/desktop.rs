//! Windows shell lifetime and notification-area menu integration.

use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Shell::{
    NIF_GUID, NIF_INFO, NIIF_RESPECT_QUIET_TIME, NIIF_WARNING, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_NULL};
use windows::core::{GUID, Result, w};

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

/// Send a notification from a registered tray icon without opening or activating a window.
///
/// [MSDN: NOTIFYICONDATAW](https://learn.microsoft.com/windows/win32/api/shellapi/ns-shellapi-notifyicondataw)
#[must_use]
pub fn notify_tray(owner: isize, guid: u128, title: &str, message: &str) -> bool {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(owner as _),
        uFlags: NIF_GUID | NIF_INFO,
        guidItem: GUID::from_u128(guid),
        dwInfoFlags: NIIF_WARNING | NIIF_RESPECT_QUIET_TIME,
        ..Default::default()
    };
    copy_tray_text(&mut data.szInfoTitle, title);
    copy_tray_text(&mut data.szInfo, message);
    // SAFETY: the shell reads a fully initialized structure with terminated UTF-16 buffers.
    unsafe { Shell_NotifyIconW(NIM_MODIFY, &data).as_bool() }
}

fn copy_tray_text(destination: &mut [u16], text: &str) {
    let mut offset = 0;
    for character in text.chars() {
        let next = offset + character.len_utf16();
        if next >= destination.len() || character == '\0' {
            break;
        }
        character.encode_utf16(&mut destination[offset..next]);
        offset = next;
    }
}
