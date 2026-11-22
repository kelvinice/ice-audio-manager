//! Default-device switching through the undocumented `IPolicyConfig` interface.
//!
//! Every audio switcher (AudioSwitcher, EarTrumpet, Windows' own Sound settings) uses this
//! interface; there is no documented alternative. It's isolated here and failures are reported,
//! never hidden (code-practices §1.6).

// Method names mirror the COM interface (the #[interface] macro rejects attributes on the trait itself).
#![allow(non_snake_case)]

use core::ffi::c_void;

use ice_core::device::Role;
use windows::Win32::Media::Audio::ERole;
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::core::{GUID, HRESULT, HSTRING, IUnknown, IUnknown_Vtbl, PCWSTR, Result, interface};

use crate::endpoints::to_erole;

/// `CLSID_PolicyConfigClient`.
const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

// Vtable order matters: every method before SetDefaultEndpoint must be declared even though we
// only call SetDefaultEndpoint. Pointer arguments we never use are typed as `*mut c_void`.
#[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    unsafe fn GetMixFormat(&self, id: PCWSTR, format: *mut *mut c_void) -> HRESULT;
    unsafe fn GetDeviceFormat(&self, id: PCWSTR, default: i32, format: *mut *mut c_void) -> HRESULT;
    unsafe fn ResetDeviceFormat(&self, id: PCWSTR) -> HRESULT;
    unsafe fn SetDeviceFormat(&self, id: PCWSTR, endpoint: *mut c_void, mix: *mut c_void) -> HRESULT;
    unsafe fn GetProcessingPeriod(&self, id: PCWSTR, default: i32, def: *mut i64, min: *mut i64) -> HRESULT;
    unsafe fn SetProcessingPeriod(&self, id: PCWSTR, period: *mut i64) -> HRESULT;
    unsafe fn GetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
    unsafe fn SetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
    unsafe fn GetPropertyValue(&self, id: PCWSTR, key: *const c_void, value: *mut c_void) -> HRESULT;
    unsafe fn SetPropertyValue(&self, id: PCWSTR, key: *const c_void, value: *mut c_void) -> HRESULT;
    unsafe fn SetDefaultEndpoint(&self, id: PCWSTR, role: ERole) -> HRESULT;
    unsafe fn SetEndpointVisibility(&self, id: PCWSTR, visible: i32) -> HRESULT;
}

/// Sets `endpoint_id` as the default for each role and reports the result per role.
///
/// Callers must re-read the defaults afterwards to confirm what actually changed (D-05, F02).
///
/// # Errors
/// Fails only if the policy-config object can't be created; per-role failures are in the returned list.
pub fn set_default(endpoint_id: &str, roles: &[Role]) -> Result<Vec<(Role, Result<()>)>> {
    // SAFETY: COM activation of the PolicyConfigClient class; the returned interface is reference-counted.
    let pc: IPolicyConfig = unsafe { CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL)? };
    let id = HSTRING::from(endpoint_id);
    Ok(roles
        .iter()
        .map(|&role| {
            // SAFETY: `id` outlives the call; the vtable layout above matches IPolicyConfig on Windows 7+.
            let hr = unsafe { pc.SetDefaultEndpoint(PCWSTR(id.as_ptr()), to_erole(role)) };
            (role, hr.ok())
        })
        .collect())
}
