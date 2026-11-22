//! Safe wrappers over WebView2 COM APIs for suspension and process-tree attribution (D-15).

use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PROCESS_KIND, COREWEBVIEW2_PROCESS_KIND_BROWSER, COREWEBVIEW2_PROCESS_KIND_GPU,
    COREWEBVIEW2_PROCESS_KIND_PPAPI_BROKER, COREWEBVIEW2_PROCESS_KIND_PPAPI_PLUGIN, COREWEBVIEW2_PROCESS_KIND_RENDERER,
    COREWEBVIEW2_PROCESS_KIND_SANDBOX_HELPER, COREWEBVIEW2_PROCESS_KIND_UTILITY, ICoreWebView2_3,
    ICoreWebView2Controller, ICoreWebView2Environment, ICoreWebView2Environment8,
};
use webview2_com::TrySuspendCompletedHandler;
use windows::core::{Interface, Result};

/// Suspends WebView2's renderer asynchronously and invokes `on_done` on the UI thread when complete.
///
/// Sets `IsVisible(false)` first because `ICoreWebView2_3::TrySuspend` requires the controller to be
/// hidden. Does not block the UI thread.
///
/// # Errors
/// Fails if `ICoreWebView2_3` is unavailable or initiating `TrySuspend` fails synchronously.
pub fn try_suspend(controller: &ICoreWebView2Controller, on_done: impl FnOnce(Result<bool>) + 'static) -> Result<()> {
    // SAFETY: `controller` is a valid COM object on the UI thread.
    let wv2 = unsafe { controller.CoreWebView2()? };
    let wv3: ICoreWebView2_3 = wv2.cast()?;
    // SAFETY: hiding the controller before `TrySuspend` is required by WebView2's contract.
    unsafe { controller.SetIsVisible(false)? };

    let mut cb = Some(on_done);
    let handler = TrySuspendCompletedHandler::create(Box::new(move |hr_res, is_successful| {
        if let Some(f) = cb.take() {
            f(hr_res.map(|()| is_successful));
        }
        Ok(())
    }));
    // SAFETY: `handler` implements `ICoreWebView2TrySuspendCompletedHandler` and lives until invoked.
    unsafe { wv3.TrySuspend(&handler) }
}

/// Resumes a previously suspended WebView2 instance and restores controller visibility.
///
/// # Errors
/// Fails if `ICoreWebView2_3` is unavailable or `Resume`/`SetIsVisible` returns an error.
pub fn resume(controller: &ICoreWebView2Controller) -> Result<()> {
    // SAFETY: `controller` is valid on the UI thread.
    let wv2 = unsafe { controller.CoreWebView2()? };
    let wv3: ICoreWebView2_3 = wv2.cast()?;
    // SAFETY: safe WebView2 visibility + resume sequence.
    unsafe {
        controller.SetIsVisible(true)?;
        wv3.Resume()?;
    }
    Ok(())
}

/// Returns `(pid, process_kind_label)` for every app-owned WebView2 process in `environment`
/// via `ICoreWebView2Environment8::GetProcessInfos` (excludes Crashpad, scoped to user data folder).
///
/// # Errors
/// Fails if `ICoreWebView2Environment8` is unavailable or `GetProcessInfos` fails.
pub fn process_infos(environment: &ICoreWebView2Environment) -> Result<Vec<(u32, String)>> {
    let env8: ICoreWebView2Environment8 = environment.cast()?;
    // SAFETY: `env8` is valid; collection is reference-counted.
    let coll = unsafe { env8.GetProcessInfos()? };
    let mut count = 0u32;
    // SAFETY: stack `u32` pointer is valid.
    unsafe { coll.Count(&raw mut count)? };

    let mut out = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
    for i in 0..count {
        // SAFETY: `i < count` is bounds-checked by the loop.
        let info = unsafe { coll.GetValueAtIndex(i)? };
        let mut pid = 0i32;
        let mut kind = COREWEBVIEW2_PROCESS_KIND(0);
        // SAFETY: stack pointers are valid for the synchronous COM getters.
        unsafe {
            info.ProcessId(&raw mut pid)?;
            info.Kind(&raw mut kind)?;
        }
        if let Ok(pid_u32) = u32::try_from(pid)
            && pid_u32 > 0
        {
            out.push((pid_u32, process_kind_label(kind).to_string()));
        }
    }
    Ok(out)
}

const fn process_kind_label(kind: COREWEBVIEW2_PROCESS_KIND) -> &'static str {
    match kind {
        COREWEBVIEW2_PROCESS_KIND_BROWSER => "wv2-browser",
        COREWEBVIEW2_PROCESS_KIND_RENDERER => "wv2-renderer",
        COREWEBVIEW2_PROCESS_KIND_UTILITY => "wv2-utility",
        COREWEBVIEW2_PROCESS_KIND_SANDBOX_HELPER => "wv2-sandbox",
        COREWEBVIEW2_PROCESS_KIND_GPU => "wv2-gpu",
        COREWEBVIEW2_PROCESS_KIND_PPAPI_PLUGIN => "wv2-plugin",
        COREWEBVIEW2_PROCESS_KIND_PPAPI_BROKER => "wv2-broker",
        _ => "wv2-other",
    }
}
