//! Windows shell activation for installed music services and their website fallbacks.
use crate::com::ComGuard;
use ice_core::media::MusicServiceId;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    ASSOCF_IS_PROTOCOL, ASSOCSTR_FRIENDLYAPPNAME, AssocQueryStringW, BHID_EnumItems, FOLDERID_AppsFolder,
    IEnumShellItems, IShellItem, IShellItem2, KF_FLAG_DEFAULT, SEE_MASK_FLAG_NO_UI, SEE_MASK_INVOKEIDLIST,
    SEE_MASK_NOASYNC, SHCreateItemInKnownFolder, SHELLEXECUTEINFOW, SHGetIDListFromObject, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{GUID, Interface, PCWSTR, Result, w};

fn matches_app(service: MusicServiceId, id: &str) -> bool {
    let pwa = match service {
        MusicServiceId::Spotify if id.eq_ignore_ascii_case("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify") => {
            return true;
        }
        MusicServiceId::Spotify => "_crx_pjibgclleladliembfgfagdaldikeohf",
        MusicServiceId::YoutubeMusic => "_crx_cinhimbnkkaeohfgghhklpknlkffjgod",
        MusicServiceId::Pandora => return false,
    };
    // Browser prefix, app identity, then optional profile suffix.
    id.split('.').nth(1).is_some_and(|part| part.eq_ignore_ascii_case(pwa))
}

/// Discover and open the installed app on user request, with the official website as fallback.
/// Pandora is not a shell app: the app shell opens it through its companion bridge, so it is refused here.
pub fn open(service: MusicServiceId) -> Result<()> {
    if service == MusicServiceId::Pandora {
        return Err(windows::core::Error::from(windows::Win32::Foundation::E_INVALIDARG));
    }
    let _com = ComGuard::init_sta()?;
    if service == MusicServiceId::Spotify && spotify_protocol_available() && open_uri(w!("spotify:")).is_ok() {
        return Ok(());
    }
    if open_registered_app(service).unwrap_or(false) {
        return Ok(());
    }
    let url = match service {
        MusicServiceId::Spotify => w!("https://open.spotify.com/"),
        MusicServiceId::YoutubeMusic => w!("https://music.youtube.com/"),
        MusicServiceId::Pandora => return Err(windows::core::Error::from(windows::Win32::Foundation::E_INVALIDARG)),
    };
    open_uri(url)
}

fn open_registered_app(service: MusicServiceId) -> Result<bool> {
    // SAFETY: the caller owns a COM apartment. The enumerator and its items stay on this thread.
    let items: IEnumShellItems = unsafe {
        let folder: IShellItem = SHCreateItemInKnownFolder(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None)?;
        folder.BindToHandler(None, &BHID_EnumItems)?
    };
    loop {
        let mut slot = [None];
        // SAFETY: Next writes at most one owned interface to this slice.
        unsafe { items.Next(&mut slot, None)? };
        let Some(item) = slot[0].take() else { return Ok(false) };
        if app_id(&item).is_some_and(|id| matches_app(service, &id)) {
            // SAFETY: use this shell item's own absolute PIDL and free it after synchronous dispatch.
            unsafe {
                let pidl = SHGetIDListFromObject(&item)?;
                let mut info = execute_info();
                info.lpIDList = pidl.cast();
                info.fMask |= SEE_MASK_INVOKEIDLIST;
                let opened = ShellExecuteExW(&mut info);
                CoTaskMemFree(Some(pidl.cast()));
                if opened.is_ok() {
                    return Ok(true);
                }
            }
        }
    }
}
fn spotify_protocol_available() -> bool {
    let mut length = 0;
    // SAFETY: size-only query of the fixed protocol association.
    let handler = unsafe {
        AssocQueryStringW(ASSOCF_IS_PROTOCOL, ASSOCSTR_FRIENDLYAPPNAME, w!("spotify"), None, None, &mut length)
    };
    handler.is_ok() && length > 1
}

fn app_id(item: &IShellItem) -> Option<String> {
    // System.AppUserModel.ID
    const APP_ID: PROPERTYKEY = PROPERTYKEY { fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3), pid: 5 };
    let item: IShellItem2 = item.cast().ok()?;
    // SAFETY: GetString returns a task-allocated terminated string. Copy and free on this thread.
    unsafe {
        let text = item.GetString(&APP_ID).ok()?;
        let id = String::from_utf16_lossy(text.as_wide());
        CoTaskMemFree(Some(text.0.cast()));
        Some(id)
    }
}

fn execute_info() -> SHELLEXECUTEINFOW {
    SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    }
}

fn open_uri(uri: PCWSTR) -> Result<()> {
    let mut info = execute_info();
    info.lpFile = uri;
    // SAFETY: callers supply only static terminated URI constants, with no arguments.
    unsafe { ShellExecuteExW(&mut info) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_the_installed_music_service_without_matching_other_apps() {
        let yt = MusicServiceId::YoutubeMusic;
        assert!(matches_app(yt, "Chrome._crx_cinhimbnkkaeohfgghhklpknlkffjgod"));
        assert!(matches_app(yt, "MSEdge._crx_cinhimbnkkaeohfgghhklpknlkffjgod.Profile_1"));
        assert!(!matches_app(yt, "Chrome._crx_cinhimbnkkaeohfgghhklpknlkffjgodx"));
        assert!(!matches_app(yt, "YouTube Music"));
        assert!(matches_app(MusicServiceId::Spotify, "Chrome._crx_pjibgclleladliembfgfagdaldikeohf"));
        assert!(matches_app(MusicServiceId::Spotify, "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"));
    }
}
