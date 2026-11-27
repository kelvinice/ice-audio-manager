//! COM apartment initialisation.

use windows::Win32::System::Com::{
    COINIT, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize,
};

/// Initialises a COM apartment for the current thread and uninitialises it on drop.
///
/// Create exactly one per thread that uses Core Audio or WinRT objects from this crate.
#[must_use = "COM is uninitialised when the guard is dropped"]
pub struct ComGuard {
    _not_send: std::marker::PhantomData<*const ()>,
}

impl ComGuard {
    /// Joins (or creates) the MTA.
    ///
    /// # Errors
    /// Fails if this thread already initialised COM as single-threaded (`RPC_E_CHANGED_MODE`).
    pub fn init_mta() -> windows::core::Result<Self> {
        Self::init(COINIT_MULTITHREADED)
    }

    /// Joins the STA: shell extensions used by ShellExecuteEx, and Japanese Romaji (the Windows phonetic analyzer and
    /// the IME reading need COM on the calling thread; without it only kana can be romanized).
    ///
    /// # Errors
    /// Fails if this thread already initialised COM as multithreaded (`RPC_E_CHANGED_MODE`).
    pub fn init_sta() -> windows::core::Result<Self> {
        Self::init(COINIT_APARTMENTTHREADED)
    }

    fn init(model: COINIT) -> windows::core::Result<Self> {
        // SAFETY: plain COM initialisation for the calling thread. Balanced by CoUninitialize in Drop,
        // including the S_FALSE ("already initialised") case, which also requires a matching uninit.
        unsafe { CoInitializeEx(None, model).ok()? };
        Ok(Self { _not_send: std::marker::PhantomData })
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: matches the successful CoInitializeEx on this same thread (the guard is !Send).
        unsafe { CoUninitialize() };
    }
}
