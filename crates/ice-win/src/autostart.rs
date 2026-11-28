//! Per-user startup registration for the desktop shell.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::{
    APPMODEL_ERROR_NO_PACKAGE, E_INVALIDARG, ERROR_FILE_NOT_FOUND, ERROR_INSUFFICIENT_BUFFER,
};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::core::{Error, HSTRING, PCWSTR, Result, w};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

/// One app's `HKCU\…\Run` value: its name and the arguments after the quoted executable.
#[derive(Clone, Copy, Debug)]
pub struct StartupEntry {
    pub value: &'static str,
    pub args: &'static str,
}

/// Ice Audio Manager starts quietly in the tray.
pub const ICE_STARTUP: StartupEntry = StartupEntry { value: "IceAudioManager", args: "--start-minimized" };

/// The current package determines which startup API may be used.
#[derive(Clone, Copy)]
pub enum Autostart {
    Unpackaged,
    Packaged,
}

impl Autostart {
    pub fn current() -> Result<Self> {
        // SAFETY: the size query writes one u32 and takes no output string buffer.
        let status = unsafe { GetCurrentPackageFullName(&mut 0, None) };
        match status {
            APPMODEL_ERROR_NO_PACKAGE => Ok(Self::Unpackaged),
            ERROR_INSUFFICIENT_BUFFER => Ok(Self::Packaged),
            status => Err(Error::from_hresult(status.to_hresult())),
        }
    }

    /// MSIX uses a manifest StartupTask, installed with the Phase 6 package.
    pub fn enabled(self) -> Result<Option<bool>> {
        self.enabled_as(ICE_STARTUP)
    }

    pub fn set_enabled(self, enabled: bool, executable: &Path) -> Result<()> {
        self.set_enabled_as(ICE_STARTUP, enabled, executable)
    }

    /// `None` when packaged (the manifest owns startup), else whether `entry`'s Run value exists.
    pub fn enabled_as(self, entry: StartupEntry) -> Result<Option<bool>> {
        if matches!(self, Self::Packaged) {
            return Ok(None);
        }
        let value = HSTRING::from(entry.value);
        // SAFETY: HKCU is a predefined handle, `value` is null-terminated and outlives the call, and a size-only
        // query needs no data buffer.
        let status =
            unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, &value, RRF_RT_REG_SZ, None, None, Some(&mut 0)) };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(Some(false));
        }
        status.ok()?;
        Ok(Some(true))
    }

    pub fn set_enabled_as(self, entry: StartupEntry, enabled: bool, executable: &Path) -> Result<()> {
        if matches!(self, Self::Packaged) {
            return Err(Error::new(E_INVALIDARG, "Packaged app startup requires a StartupTask"));
        }
        let value = HSTRING::from(entry.value);
        if enabled {
            let command = startup_command(executable, entry.args)?;
            // SAFETY: the UTF-16 buffer includes its terminator and lives through the write; `value` is
            // null-terminated.
            let size = (command.len() * size_of::<u16>()) as u32;
            unsafe {
                RegSetKeyValueW(HKEY_CURRENT_USER, RUN_KEY, &value, REG_SZ.0, Some(command.as_ptr().cast()), size)
            }
            .ok()
        } else {
            // SAFETY: delete only this entry's value from HKCU; `value` is null-terminated and outlives the call.
            let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, &value) };
            if status == ERROR_FILE_NOT_FOUND { Ok(()) } else { status.ok() }
        }
    }
}

fn startup_command(executable: &Path, args: &str) -> Result<Vec<u16>> {
    let path: Vec<_> = executable.as_os_str().encode_wide().collect();
    if !executable.is_absolute() || path.iter().any(|&character| character == 0 || character == u16::from(b'"')) {
        return Err(Error::new(E_INVALIDARG, "Startup requires an absolute executable path"));
    }
    let mut command = vec![u16::from(b'"')];
    command.extend(path);
    command.push(u16::from(b'"'));
    if !args.is_empty() {
        command.push(u16::from(b' '));
        command.extend(args.encode_utf16());
    }
    if command.len() > 260 {
        return Err(Error::new(E_INVALIDARG, "Windows startup commands are limited to 260 characters"));
    }
    command.push(0);
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_quotes_the_executable_and_requests_tray_only() {
        let command =
            startup_command(Path::new(r"C:\Program Files\Ice\ice-audio-manager.exe"), ICE_STARTUP.args).unwrap();
        assert_eq!(
            String::from_utf16(&command).unwrap(),
            "\"C:\\Program Files\\Ice\\ice-audio-manager.exe\" --start-minimized\0"
        );
    }

    #[test]
    fn startup_rejects_invalid_and_overlong_paths() {
        assert!(startup_command(Path::new("ice-audio-manager.exe"), "").is_err());
        assert!(startup_command(Path::new(&format!("C:\\{}.exe", "a".repeat(260))), "").is_err());
    }

    #[test]
    fn startup_uses_each_entry_args() {
        let bare = startup_command(Path::new(r"C:\Apps\pandora.exe"), "").unwrap();
        assert_eq!(String::from_utf16(&bare).unwrap(), "\"C:\\Apps\\pandora.exe\"\0");
        let pandora = startup_command(Path::new(r"C:\Apps\pandora.exe"), "--background").unwrap();
        assert_eq!(String::from_utf16(&pandora).unwrap(), "\"C:\\Apps\\pandora.exe\" --background\0");
    }
}
