//! Audio endpoint domain types.

use serde::{Deserialize, Serialize};

/// Direction of an audio endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Flow {
    /// Playback (speakers, headphones).
    Render,
    /// Recording (microphones).
    Capture,
}

/// Endpoint state as reported by Windows.
///
/// Windows keeps `NotPresent` endpoints forever, often with the same name as a live one,
/// so resolvers must only ever pick `Active` endpoints (D-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    Active,
    Disabled,
    NotPresent,
    Unplugged,
    Unknown,
}

impl DeviceState {
    /// Maps the `DEVICE_STATE_*` bit flags used by the MMDevice API.
    #[must_use]
    pub fn from_bits(bits: u32) -> Self {
        match bits & 0xF {
            0x1 => Self::Active,
            0x2 => Self::Disabled,
            0x4 => Self::NotPresent,
            0x8 => Self::Unplugged,
            _ => Self::Unknown,
        }
    }
}

/// Default-device roles (`ERole`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Console,
    Multimedia,
    Communications,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Console, Role::Multimedia, Role::Communications];
    /// "Default" in the UI means Console + Multimedia (D-05, F02).
    pub const DEFAULT: [Role; 2] = [Role::Console, Role::Multimedia];
}

/// Everything we know about one endpoint at one point in time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointInfo {
    /// MMDevice endpoint ID, e.g. `{0.0.0.00000000}.{guid}`.
    pub id: String,
    pub flow: Flow,
    pub state: DeviceState,
    /// Full name, e.g. "Headphones (Arctis Nova 7)".
    pub friendly_name: Option<String>,
    /// Endpoint part of the name, e.g. "Headphones".
    pub endpoint_name: Option<String>,
    /// Adapter part of the name, e.g. "Arctis Nova 7".
    pub interface_name: Option<String>,
    /// Parent device node, e.g. `{1}.USB\VID_1038&PID_22A1&MI_00\7&2B084728&0&0000`.
    pub devnode: Option<String>,
    /// Model-level hardware key derived from `devnode` (see [`hardware_key_from_devnode`]).
    pub hardware_key: Option<String>,
    pub container_id: Option<String>,
    /// `EndpointFormFactor` value (e.g. 3 = Headphones, 1 = Speakers).
    pub form_factor: Option<u32>,
    /// `PKEY_AudioEndpoint_StableId` (Windows 11 24H2+), if Windows provides it.
    pub stable_id: Option<String>,
    /// Roles for which this endpoint is currently the default.
    pub default_for: Vec<Role>,
}

/// Derives a model-level hardware key from an endpoint's parent devnode string.
///
/// Drops the `{n}.` prefix and the per-instance suffix, keeping bus + hardware IDs:
/// `{1}.USB\VID_1038&PID_22A1&MI_00\7&2B084728&0&0000` → `USB\VID_1038&PID_22A1&MI_00`.
/// Distinct units of the same model share this key; `container_id` tells them apart.
#[must_use]
pub fn hardware_key_from_devnode(devnode: &str) -> Option<String> {
    let s = devnode.trim();
    let s = match s.strip_prefix('{') {
        Some(rest) => rest.split_once("}.").map_or(s, |(_, tail)| tail),
        None => s,
    };
    let mut parts = s.split('\\').filter(|p| !p.is_empty());
    let bus = parts.next()?;
    let hw = parts.next()?;
    Some(format!("{}\\{}", bus.to_ascii_uppercase(), hw.to_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real devnode strings from the owner's machine (2026-11-22 registry scan).
    #[test]
    fn usb_headset_key_drops_instance() {
        assert_eq!(
            hardware_key_from_devnode(r"{1}.USB\VID_1038&PID_22A1&MI_00\7&2B084728&0&0000").as_deref(),
            Some(r"USB\VID_1038&PID_22A1&MI_00")
        );
    }

    #[test]
    fn hdaudio_key_keeps_codec_ids() {
        assert_eq!(
            hardware_key_from_devnode(
                r"{1}.HDAUDIO\FUNC_01&VEN_10EC&DEV_0B00&SUBSYS_18491204&REV_1000\5&7AA6472&0&0001"
            )
            .as_deref(),
            Some(r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0B00&SUBSYS_18491204&REV_1000")
        );
    }

    #[test]
    fn virtual_and_bluetooth_keys() {
        assert_eq!(hardware_key_from_devnode(r"{1}.ROOT\MEDIA\0000").as_deref(), Some(r"ROOT\MEDIA"));
        assert_eq!(
            hardware_key_from_devnode(r"{1}.BTHHFENUM\BTHHFPAUDIO\9&1F0190A0&0&97").as_deref(),
            Some(r"BTHHFENUM\BTHHFPAUDIO")
        );
    }

    #[test]
    fn malformed_devnode_is_none() {
        assert_eq!(hardware_key_from_devnode(""), None);
        assert_eq!(hardware_key_from_devnode("{1}.USB"), None);
    }

    #[test]
    fn state_bits() {
        assert_eq!(DeviceState::from_bits(1), DeviceState::Active);
        assert_eq!(DeviceState::from_bits(4), DeviceState::NotPresent);
        assert_eq!(DeviceState::from_bits(0x10000004), DeviceState::NotPresent);
    }
}
