//! Spatial sound format per playback endpoint (D-17).
//!
//! Uses the documented WinRT API `Windows.Media.Audio.SpatialAudioDeviceConfiguration`
//! (Windows 10 1903+). Phase 0.3 must confirm that it works un-elevated from an unpackaged app.

use windows::Media::Audio::{
    SetDefaultSpatialAudioFormatStatus, SpatialAudioDeviceConfiguration, SpatialAudioFormatSubtype,
};
use windows::core::{HSTRING, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialFormat {
    WindowsSonic,
    DolbyAtmosHeadphones,
    DolbyAtmosHomeTheater,
    DolbyAtmosSpeakers,
    DtsHeadphoneX,
    DtsXUltra,
    DtsXHomeTheater,
}

impl SpatialFormat {
    pub const ALL: [SpatialFormat; 7] = [
        Self::WindowsSonic,
        Self::DolbyAtmosHeadphones,
        Self::DolbyAtmosHomeTheater,
        Self::DolbyAtmosSpeakers,
        Self::DtsHeadphoneX,
        Self::DtsXUltra,
        Self::DtsXHomeTheater,
    ];

    /// The format's subtype string as defined by Windows.
    ///
    /// # Errors
    /// Fails if the WinRT activation factory is unavailable.
    pub fn subtype(self) -> Result<HSTRING> {
        match self {
            Self::WindowsSonic => SpatialAudioFormatSubtype::WindowsSonic(),
            Self::DolbyAtmosHeadphones => SpatialAudioFormatSubtype::DolbyAtmosForHeadphones(),
            Self::DolbyAtmosHomeTheater => SpatialAudioFormatSubtype::DolbyAtmosForHomeTheater(),
            Self::DolbyAtmosSpeakers => SpatialAudioFormatSubtype::DolbyAtmosForSpeakers(),
            Self::DtsHeadphoneX => SpatialAudioFormatSubtype::DTSHeadphoneX(),
            Self::DtsXUltra => SpatialAudioFormatSubtype::DTSXUltra(),
            Self::DtsXHomeTheater => SpatialAudioFormatSubtype::DTSXForHomeTheater(),
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::WindowsSonic => "Windows Sonic for Headphones",
            Self::DolbyAtmosHeadphones => "Dolby Atmos for Headphones",
            Self::DolbyAtmosHomeTheater => "Dolby Atmos for home theater",
            Self::DolbyAtmosSpeakers => "Dolby Atmos for built-in speakers",
            Self::DtsHeadphoneX => "DTS Headphone:X",
            Self::DtsXUltra => "DTS:X Ultra",
            Self::DtsXHomeTheater => "DTS:X for home theater",
        }
    }

    /// Parses short CLI names: `sonic`, `atmos`, `atmos-ht`, `atmos-spk`, `dts-hx`, `dtsx-ultra`, `dtsx-ht`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "sonic" => Self::WindowsSonic,
            "atmos" => Self::DolbyAtmosHeadphones,
            "atmos-ht" => Self::DolbyAtmosHomeTheater,
            "atmos-spk" => Self::DolbyAtmosSpeakers,
            "dts-hx" => Self::DtsHeadphoneX,
            "dtsx-ultra" => Self::DtsXUltra,
            "dtsx-ht" => Self::DtsXHomeTheater,
            _ => return None,
        })
    }

    fn from_subtype(subtype: &HSTRING) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|f| f.subtype().is_ok_and(|s| s.to_string().eq_ignore_ascii_case(&subtype.to_string())))
    }
}

/// WinRT device interface path for a render endpoint (`KSCATEGORY_AUDIO` render interface class).
#[must_use]
pub fn winrt_render_id(endpoint_id: &str) -> HSTRING {
    HSTRING::from(format!(r"\\?\SWD#MMDEVAPI#{endpoint_id}#{{e6327cad-dcec-4949-ae8a-991e976a79d2}}"))
}

#[derive(Debug, Clone)]
pub struct SpatialStatus {
    pub supported: bool,
    /// What is actually active right now (`None` = off). Raw subtype kept for unknown providers.
    pub active: Option<(Option<SpatialFormat>, String)>,
    /// What the user selected in Windows settings (`None` = off).
    pub default: Option<(Option<SpatialFormat>, String)>,
    pub available: Vec<SpatialFormat>,
}

/// Windows reports "off" as an empty string or the null GUID.
fn classify(raw: HSTRING) -> Option<(Option<SpatialFormat>, String)> {
    let s = raw.to_string();
    let off = s.is_empty() || s.trim_matches(['{', '}']).chars().all(|c| c == '0' || c == '-');
    (!off).then(|| (SpatialFormat::from_subtype(&raw), s))
}

fn config(endpoint_id: &str) -> Result<SpatialAudioDeviceConfiguration> {
    SpatialAudioDeviceConfiguration::GetForDeviceId(&winrt_render_id(endpoint_id))
}

/// Reads the spatial configuration of a render endpoint.
///
/// # Errors
/// Fails if the endpoint isn't a render endpoint or the WinRT API is unavailable.
pub fn status(endpoint_id: &str) -> Result<SpatialStatus> {
    let cfg = config(endpoint_id)?;
    let available = SpatialFormat::ALL
        .into_iter()
        .filter(|f| f.subtype().and_then(|s| cfg.IsSpatialAudioFormatSupported(&s)).unwrap_or(false))
        .collect();
    Ok(SpatialStatus {
        supported: cfg.IsSpatialAudioSupported()?,
        active: classify(cfg.ActiveSpatialAudioFormat()?),
        default: classify(cfg.DefaultSpatialAudioFormat()?),
        available,
    })
}

/// Sets the default spatial format (`None` = off) and returns Windows' status, e.g. `Succeeded`,
/// `AccessDenied`, `LicenseNotValidForAudioEndpoint`.
///
/// Callers should read first and skip the write when it already matches (Gemini, D-17): a write can
/// briefly re-initialise active audio streams.
///
/// # Errors
/// Fails if the async operation itself fails.
pub fn set_default(endpoint_id: &str, format: Option<SpatialFormat>) -> Result<&'static str> {
    let cfg = config(endpoint_id)?;
    // "Off" is the null GUID: an empty string is rejected with AccessDenied (measured 2026-11-22).
    let subtype = match format {
        Some(f) => f.subtype()?,
        None => HSTRING::from("{00000000-0000-0000-0000-000000000000}"),
    };
    let status = cfg.SetDefaultSpatialAudioFormatAsync(&subtype)?.join()?.Status()?;
    Ok(match status {
        s if s == SetDefaultSpatialAudioFormatStatus::Succeeded => "Succeeded",
        s if s == SetDefaultSpatialAudioFormatStatus::AccessDenied => "AccessDenied",
        s if s == SetDefaultSpatialAudioFormatStatus::LicenseExpired => "LicenseExpired",
        s if s == SetDefaultSpatialAudioFormatStatus::LicenseNotValidForAudioEndpoint => {
            "LicenseNotValidForAudioEndpoint"
        }
        s if s == SetDefaultSpatialAudioFormatStatus::NotSupportedOnAudioEndpoint => "NotSupportedOnAudioEndpoint",
        _ => "UnknownError",
    })
}
