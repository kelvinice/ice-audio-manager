//! COM apartment initialisation.

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

/// Initialises the multithreaded COM apartment for the current thread and uninitialises it on drop.
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
        // SAFETY: plain COM initialisation for the calling thread. Balanced by CoUninitialize in Drop,
        // including the S_FALSE ("already initialised") case, which also requires a matching uninit.
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
        Ok(Self { _not_send: std::marker::PhantomData })
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: matches the successful CoInitializeEx in `init_mta` on this same thread (the guard is !Send).
        unsafe { CoUninitialize() };
    }
}
