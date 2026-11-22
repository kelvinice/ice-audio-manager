//! Property-store helpers and the property keys we read from audio endpoints.

use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Com::StructuredStorage::{
    PROPVARIANT, PropVariantClear, PropVariantToStringAlloc, PropVariantToUInt32,
};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::core::GUID;

const fn pkey(fmtid: u128, pid: u32) -> PROPERTYKEY {
    PROPERTYKEY { fmtid: GUID::from_u128(fmtid), pid }
}

/// `PKEY_Device_FriendlyName`, e.g. "Headphones (Arctis Nova 7)".
pub const FRIENDLY_NAME: PROPERTYKEY = pkey(0xa45c254e_df1c_4efd_8020_67d146a850e0, 14);
/// `PKEY_Device_DeviceDesc`, e.g. "Headphones".
pub const DEVICE_DESC: PROPERTYKEY = pkey(0xa45c254e_df1c_4efd_8020_67d146a850e0, 2);
/// `PKEY_DeviceInterface_FriendlyName`, e.g. "Arctis Nova 7".
pub const INTERFACE_FRIENDLY_NAME: PROPERTYKEY = pkey(0x026e516e_b814_414b_83cd_856d6fef4822, 2);
/// `PKEY_Device_ContainerId`.
pub const CONTAINER_ID: PROPERTYKEY = pkey(0x8c7ed206_3f8a_4827_b3ab_ae9e1faefc6c, 2);
/// Parent devnode of the endpoint, e.g. `{1}.USB\VID_1038&PID_22A1&MI_00\…` (MMDevAPI endpoint property).
pub const ENDPOINT_DEVNODE: PROPERTYKEY = pkey(0xb3f8fa53_0004_438e_9003_51a46e139bfc, 2);
/// `PKEY_AudioEndpoint_FormFactor`.
pub const FORM_FACTOR: PROPERTYKEY = pkey(0x1da5d803_d492_4edd_8c23_e0c0ffee7f0e, 0);
/// `PKEY_AudioEndpoint_StableId` (Windows 11 24H2+). Not in the windows crate or SDK 10.0.26100 headers;
/// found by the Phase 0.3 probe: the only string property that `GetDevice` resolves back to the same
/// endpoint on every device. Value looks like `{0.0.0.00000000}.{001.{GUID}}`.
pub const STABLE_ID: PROPERTYKEY = pkey(0x1da5d803_d492_4edd_8c23_e0c0ffee7f0e, 12);

/// Owns a `PROPVARIANT` and clears it on drop.
struct Prop(PROPVARIANT);

impl Drop for Prop {
    fn drop(&mut self) {
        // SAFETY: self.0 is a PROPVARIANT returned by IPropertyStore::GetValue that we own.
        // Clearing an already-empty variant is a no-op.
        unsafe {
            let _ = PropVariantClear(&mut self.0);
        }
    }
}

fn read(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<Prop> {
    // SAFETY: `key` is a valid PROPERTYKEY for the duration of the call; the returned variant is owned by `Prop`.
    unsafe { store.GetValue(key) }.ok().map(Prop)
}

fn to_string(pv: &PROPVARIANT) -> Option<String> {
    // SAFETY: `pv` is a valid, initialised PROPVARIANT. The returned buffer is CoTaskMemAlloc'ed;
    // we copy it into a Rust String and free it immediately.
    unsafe {
        let p = PropVariantToStringAlloc(pv).ok()?;
        let s = String::from_utf16_lossy(p.as_wide());
        CoTaskMemFree(Some(p.0 as *const core::ffi::c_void));
        Some(s).filter(|s| !s.is_empty())
    }
}

/// Reads a property as a string (most variant types convert). `None` if missing or empty.
#[must_use]
pub fn get_string(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<String> {
    read(store, key).and_then(|p| to_string(&p.0))
}

/// Reads a property as `u32`.
#[must_use]
pub fn get_u32(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<u32> {
    let p = read(store, key)?;
    // SAFETY: `p.0` is a valid PROPVARIANT owned by `p`, alive for this call.
    unsafe { PropVariantToUInt32(&p.0) }.ok()
}

/// Formats a key as `{FMTID},PID`.
#[must_use]
pub fn key_to_string(key: &PROPERTYKEY) -> String {
    format!("{{{:?}}},{}", key.fmtid, key.pid)
}

/// Every property in a store, with its value converted to a string where possible.
#[must_use]
pub fn dump(store: &IPropertyStore) -> Vec<(PROPERTYKEY, Option<String>)> {
    // SAFETY: plain property-store reads; failures are skipped.
    let count = unsafe { store.GetCount() }.unwrap_or(0);
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        let mut key = PROPERTYKEY::default();
        // SAFETY: `key` is a valid out-pointer for this call.
        if unsafe { store.GetAt(i, &mut key) }.is_err() {
            continue;
        }
        let value = read(store, &key).and_then(|p| to_string(&p.0));
        out.push((key, value));
    }
    out
}
