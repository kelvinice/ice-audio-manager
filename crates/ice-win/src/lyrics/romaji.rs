//! Offline Japanese → Romaji conversion using Windows' built-in MS-IME (`IFELanguage::GetJMorphResult`)
//! with automatic fallback to `ice_core::lyrics::kana_to_romaji` (100 % MIT, zero extra crates).

use std::ffi::c_void;

use ice_core::lyrics::{contains_japanese, contains_kanji, kana_to_romaji};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CLSIDFromProgID, CoCreateInstance, CoTaskMemFree};
use windows::core::{GUID, HRESULT, HSTRING, IUnknown, Interface, PCWSTR};

/// `IFELanguage` COM interface ID (`msime.h`).
const IID_IFE_LANGUAGE: GUID = GUID::from_u128(0x019f_7152_e6db_11d0_83c3_00c0_4fdd_b82e);
/// Request reverse morphological conversion (Kanji → Kana reading).
const FELANG_REQ_REV: u32 = 0x0003_0000;
/// Output Hiragana readings with phrase/clause segmentation.
const FELANG_CMODE_HIRAGANAOUT: u32 = 0x0000_0000;
const FELANG_CMODE_PHRASEPREDICT: u32 = 0x1000_0000;

#[repr(C)]
struct IFeLanguageVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    open: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    close: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    get_j_morph_result:
        unsafe extern "system" fn(*mut c_void, u32, u32, i32, *const u16, *const u32, *mut *mut MorRslt) -> HRESULT,
}

#[repr(C)]
struct IFeLanguageRaw {
    vtbl: *const IFeLanguageVtbl,
}

/// Prefix of Windows `MORRSLT` structure (`msime.h`) containing output reading buffer and word/clause breaks.
#[repr(C)]
struct MorRslt {
    dw_size: u32,
    pwch_output: *mut u16,
    cch_output: u16,
    pwch_read: *mut u16,
    cch_read: u16,
    pch_input_pos: *mut u16,
    pch_output_idx_wdd: *mut u16,
    pch_read_idx_wdd: *mut u16,
    pa_mono_ruby_pos: *mut u16,
    p_wdd: *mut c_void,
    c_wdd: i32,
    p_private: *mut c_void,
}

/// RAII wrapper around Windows' `MSIME.Japan` `IFELanguage` COM object (lives on the `ice-lyrics` MTA thread).
pub struct JapaneseRomaji {
    raw: Option<*mut IFeLanguageRaw>,
}

impl JapaneseRomaji {
    /// Initializes `IFELanguage` if the Windows Japanese IME is installed; degrades cleanly if absent.
    #[must_use]
    pub fn new() -> Self {
        let prog_id = HSTRING::from("MSIME.Japan");
        // SAFETY: `prog_id` is a valid null-terminated UTF-16 string; COM is initialized on the calling thread.
        let raw = unsafe {
            let Ok(clsid) = CLSIDFromProgID(PCWSTR(prog_id.as_ptr())) else {
                tracing::debug!("MSIME.Japan not installed; Romaji will convert Kana-only lines");
                return Self { raw: None };
            };
            let Ok(unknown) = CoCreateInstance::<_, IUnknown>(&clsid, None, CLSCTX_INPROC_SERVER) else {
                tracing::debug!("IFELanguage unavailable");
                return Self { raw: None };
            };
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let hr = unknown.query(&IID_IFE_LANGUAGE, &mut ptr);
            if hr.is_err() || ptr.is_null() {
                tracing::debug!(?hr, "IFELanguage QueryInterface unavailable");
                return Self { raw: None };
            }
            let obj = ptr.cast::<IFeLanguageRaw>();
            let open_hr = ((*(*obj).vtbl).open)(ptr);
            if open_hr.is_err() {
                ((*(*obj).vtbl).release)(ptr);
                tracing::debug!(?open_hr, "IFELanguage::Open failed");
                return Self { raw: None };
            }
            Some(obj)
        };
        Self { raw }
    }

    /// Returns an offline Hepburn Romaji subtitle for `line`, or `None` if not Japanese / unreadable Kanji.
    #[must_use]
    pub fn line_to_romaji(&self, line: &str) -> Option<String> {
        if !contains_japanese(line) {
            return None;
        }
        if !contains_kanji(line) {
            let rom = kana_to_romaji(line);
            return (!rom.is_empty() && rom != line.trim()).then_some(rom);
        }
        let kana = self.kanji_to_spaced_kana(line)?;
        let rom = kana_to_romaji(&kana);
        (!rom.is_empty() && !contains_kanji(&rom)).then_some(rom)
    }

    fn kanji_to_spaced_kana(&self, text: &str) -> Option<String> {
        let obj = self.raw?;
        let utf16: Vec<u16> = text.encode_utf16().collect();
        let len = i32::try_from(utf16.len()).ok()?;
        if len == 0 {
            return None;
        }
        let mut result_ptr: *mut MorRslt = std::ptr::null_mut();
        // SAFETY: `obj` is an opened `IFELanguage` instance; `utf16` stays alive across the call; `result_ptr` is freed via `CoTaskMemFree`.
        unsafe {
            let hr = ((*(*obj).vtbl).get_j_morph_result)(
                obj.cast(),
                FELANG_REQ_REV,
                FELANG_CMODE_HIRAGANAOUT | FELANG_CMODE_PHRASEPREDICT,
                len,
                utf16.as_ptr(),
                std::ptr::null(),
                &mut result_ptr,
            );
            if hr.is_err() || result_ptr.is_null() {
                return None;
            }
            let extracted = extract_spaced_output(&*result_ptr);
            CoTaskMemFree(Some(result_ptr.cast()));
            extracted
        }
    }
}

impl Default for JapaneseRomaji {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for JapaneseRomaji {
    fn drop(&mut self) {
        if let Some(obj) = self.raw.take() {
            // SAFETY: `obj` was opened and owned uniquely by `JapaneseRomaji`.
            unsafe {
                let _ = ((*(*obj).vtbl).close)(obj.cast());
                ((*(*obj).vtbl).release)(obj.cast());
            }
        }
    }
}

/// Reads `pwch_output` (`cch_output` UTF-16 code units), inserting spaces at `pch_output_idx_wdd` word breaks.
///
/// # Safety
/// `res` must be a valid `MORRSLT` populated by `IFELanguage::GetJMorphResult`.
unsafe fn extract_spaced_output(res: &MorRslt) -> Option<String> {
    if res.pwch_output.is_null() || res.cch_output == 0 {
        return None;
    }
    let total = usize::from(res.cch_output);
    // SAFETY: `pwch_output` points to `cch_output` initialized UTF-16 code units owned by `res`.
    let output = unsafe { std::slice::from_raw_parts(res.pwch_output, total) };
    if res.c_wdd > 0 && !res.pch_output_idx_wdd.is_null() {
        let count = usize::try_from(res.c_wdd).ok()?;
        // SAFETY: `pch_output_idx_wdd` holds `c_wdd + 1` monotonically non-decreasing indices into `output`.
        let idxs = unsafe { std::slice::from_raw_parts(res.pch_output_idx_wdd, count + 1) };
        let mut parts = Vec::with_capacity(count);
        for pair in idxs.windows(2) {
            let a = usize::from(pair[0]).min(total);
            let b = usize::from(pair[1]).min(total);
            if b > a {
                let chunk = String::from_utf16_lossy(&output[a..b]);
                let trimmed = chunk.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
        }
        if !parts.is_empty() {
            return Some(parts.join(" "));
        }
    }
    Some(String::from_utf16_lossy(output))
}
