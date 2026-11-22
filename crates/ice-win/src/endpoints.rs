//! Endpoint enumeration and identity (MMDevice API).

use ice_core::device::{DeviceState, EndpointInfo, Flow, Role, hardware_key_from_devnode};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    DEVICE_STATE, DEVICE_STATE_ACTIVE, DEVICE_STATEMASK_ALL, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator,
    IMMEndpoint, MMDeviceEnumerator, eCapture, eCommunications, eConsole, eMultimedia, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, STGM_READ};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::core::{HSTRING, Interface, Result};

use crate::props;

/// Wraps `IMMDeviceEnumerator`. Must be used on a thread holding a [`crate::com::ComGuard`].
pub struct Endpoints {
    en: IMMDeviceEnumerator,
}

impl Endpoints {
    /// # Errors
    /// Fails if the MMDevice enumerator can't be created (COM not initialised, audio service down).
    pub fn new() -> Result<Self> {
        // SAFETY: standard COM activation of the documented MMDeviceEnumerator class.
        let en = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        Ok(Self { en })
    }

    #[must_use]
    pub fn raw(&self) -> &IMMDeviceEnumerator {
        &self.en
    }

    /// Lists endpoints of both flows. `include_inactive` adds disabled, unplugged and not-present ghosts.
    ///
    /// # Errors
    /// Fails only if enumeration itself fails; unreadable properties become `None`.
    pub fn list(&self, include_inactive: bool) -> Result<Vec<EndpointInfo>> {
        let defaults = self.defaults();
        let mask = if include_inactive { DEVICE_STATE(DEVICE_STATEMASK_ALL) } else { DEVICE_STATE_ACTIVE };
        let mut out = Vec::new();
        for flow in [Flow::Render, Flow::Capture] {
            // SAFETY: valid enumerator; collection and items are reference-counted COM objects.
            let coll = unsafe { self.en.EnumAudioEndpoints(to_edataflow(flow), mask)? };
            let count = unsafe { coll.GetCount()? };
            for i in 0..count {
                let dev = unsafe { coll.Item(i)? };
                out.push(self.describe(&dev, flow, &defaults)?);
            }
        }
        Ok(out)
    }

    /// Opens an endpoint by ID (or any string `GetDevice` accepts, e.g. a stable ID).
    ///
    /// # Errors
    /// `E_NOTFOUND` style errors if no such endpoint exists.
    pub fn device(&self, id: &str) -> Result<IMMDevice> {
        // SAFETY: HSTRING keeps the wide string alive for the duration of the call.
        unsafe { self.en.GetDevice(&HSTRING::from(id)) }
    }

    /// Describes a single endpoint.
    ///
    /// # Errors
    /// Fails if the endpoint's ID or state can't be read.
    pub fn info(&self, dev: &IMMDevice) -> Result<EndpointInfo> {
        let flow = flow_of(dev)?;
        self.describe(dev, flow, &self.defaults())
    }

    /// Current default endpoint ID per flow and role.
    #[must_use]
    pub fn defaults(&self) -> Vec<(Flow, Role, String)> {
        let mut out = Vec::new();
        for flow in [Flow::Render, Flow::Capture] {
            for role in Role::ALL {
                // SAFETY: plain query; "no default" is an error we skip.
                if let Ok(dev) = unsafe { self.en.GetDefaultAudioEndpoint(to_edataflow(flow), to_erole(role)) }
                    && let Ok(id) = device_id(&dev)
                {
                    out.push((flow, role, id));
                }
            }
        }
        out
    }

    /// Default endpoint for one flow/role.
    ///
    /// # Errors
    /// Fails if there is no default (e.g. no active device of that flow).
    pub fn default_device(&self, flow: Flow, role: Role) -> Result<IMMDevice> {
        // SAFETY: plain query on a valid enumerator.
        unsafe { self.en.GetDefaultAudioEndpoint(to_edataflow(flow), to_erole(role)) }
    }

    /// Phase 0.3 probe for `PKEY_AudioEndpoint_StableId`, whose key isn't in the windows crate or the
    /// local SDK headers. Returns every string property (other than the endpoint ID itself) that
    /// `IMMDeviceEnumerator::GetDevice` resolves back to this same endpoint.
    #[must_use]
    pub fn probe_stable_id(&self, dev: &IMMDevice) -> Vec<(PROPERTYKEY, String)> {
        let Ok(id) = device_id(dev) else { return Vec::new() };
        // SAFETY: read-only property store access.
        let Ok(store) = (unsafe { dev.OpenPropertyStore(STGM_READ) }) else { return Vec::new() };
        props::dump(&store)
            .into_iter()
            .filter_map(|(key, value)| value.map(|v| (key, v)))
            .filter(|(_, v)| v != &id && v.len() < 512)
            .filter(|(_, v)| self.device(v).ok().and_then(|d| device_id(&d).ok()).as_deref() == Some(id.as_str()))
            .collect()
    }

    fn describe(&self, dev: &IMMDevice, flow: Flow, defaults: &[(Flow, Role, String)]) -> Result<EndpointInfo> {
        let id = device_id(dev)?;
        // SAFETY: plain getters on a valid IMMDevice.
        let state = DeviceState::from_bits(unsafe { dev.GetState()? }.0);
        let store = unsafe { dev.OpenPropertyStore(STGM_READ) }.ok();
        let get = |key| store.as_ref().and_then(|s| props::get_string(s, key));
        let devnode = get(&props::ENDPOINT_DEVNODE);
        Ok(EndpointInfo {
            flow,
            state,
            friendly_name: get(&props::FRIENDLY_NAME),
            endpoint_name: get(&props::DEVICE_DESC),
            interface_name: get(&props::INTERFACE_FRIENDLY_NAME),
            hardware_key: devnode.as_deref().and_then(hardware_key_from_devnode),
            devnode,
            container_id: get(&props::CONTAINER_ID),
            form_factor: store.as_ref().and_then(|s| props::get_u32(s, &props::FORM_FACTOR)),
            stable_id: get(&props::STABLE_ID),
            default_for: defaults.iter().filter(|(f, _, d)| *f == flow && *d == id).map(|(_, r, _)| *r).collect(),
            id,
        })
    }
}

/// Reads an endpoint's ID string.
///
/// # Errors
/// Fails if `IMMDevice::GetId` fails (device invalidated).
pub fn device_id(dev: &IMMDevice) -> Result<String> {
    // SAFETY: GetId returns a CoTaskMemAlloc'ed string that we copy and then free.
    unsafe {
        let p = dev.GetId()?;
        let s = String::from_utf16_lossy(p.as_wide());
        CoTaskMemFree(Some(p.0 as *const core::ffi::c_void));
        Ok(s)
    }
}

/// Opens an endpoint's property store read-only.
///
/// # Errors
/// Fails if the endpoint is invalidated.
pub fn open_store(dev: &IMMDevice) -> Result<IPropertyStore> {
    // SAFETY: read-only access on a valid device.
    unsafe { dev.OpenPropertyStore(STGM_READ) }
}

/// Reads an endpoint's data flow.
///
/// # Errors
/// Fails if the device doesn't expose `IMMEndpoint`.
pub fn flow_of(dev: &IMMDevice) -> Result<Flow> {
    // SAFETY: QueryInterface + getter on a valid device.
    let flow = unsafe { dev.cast::<IMMEndpoint>()?.GetDataFlow()? };
    Ok(from_edataflow(flow).unwrap_or(Flow::Render))
}

pub(crate) fn to_edataflow(flow: Flow) -> EDataFlow {
    match flow {
        Flow::Render => eRender,
        Flow::Capture => eCapture,
    }
}

pub(crate) fn from_edataflow(flow: EDataFlow) -> Option<Flow> {
    match flow {
        f if f == eRender => Some(Flow::Render),
        f if f == eCapture => Some(Flow::Capture),
        _ => None,
    }
}

pub(crate) fn to_erole(role: Role) -> ERole {
    match role {
        Role::Console => eConsole,
        Role::Multimedia => eMultimedia,
        Role::Communications => eCommunications,
    }
}

pub(crate) fn from_erole(role: ERole) -> Option<Role> {
    match role {
        r if r == eConsole => Some(Role::Console),
        r if r == eMultimedia => Some(Role::Multimedia),
        r if r == eCommunications => Some(Role::Communications),
        _ => None,
    }
}
