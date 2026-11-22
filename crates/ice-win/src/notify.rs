//! Endpoint notifications (`IMMNotificationClient`) → [`AudioEvent`]s delivered to a caller-supplied sink.

use ice_core::device::DeviceState;
use ice_core::event::AudioEvent;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    DEVICE_STATE, EDataFlow, ERole, IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl,
};
use windows::core::{PCWSTR, Result, implement};

use crate::endpoints::{from_edataflow, from_erole};
use crate::props::key_to_string;

#[implement(IMMNotificationClient)]
struct Client {
    sink: Box<dyn Fn(AudioEvent) + Send + Sync>,
}

fn id_of(id: &PCWSTR) -> Option<String> {
    if id.is_null() {
        return None;
    }
    // SAFETY: Windows passes a valid null-terminated string for the duration of the callback.
    Some(String::from_utf16_lossy(unsafe { id.as_wide() }))
}

// Callbacks run on arbitrary MMDevAPI threads. They must not block and must not call back into
// MMDevAPI synchronously (documented deadlock), so they only send to the channel.
impl IMMNotificationClient_Impl for Client_Impl {
    fn OnDeviceStateChanged(&self, id: &PCWSTR, state: DEVICE_STATE) -> Result<()> {
        if let Some(id) = id_of(id) {
            (self.sink)(AudioEvent::StateChanged { id, state: DeviceState::from_bits(state.0) });
        }
        Ok(())
    }

    fn OnDeviceAdded(&self, id: &PCWSTR) -> Result<()> {
        if let Some(id) = id_of(id) {
            (self.sink)(AudioEvent::DeviceAdded { id });
        }
        Ok(())
    }

    fn OnDeviceRemoved(&self, id: &PCWSTR) -> Result<()> {
        if let Some(id) = id_of(id) {
            (self.sink)(AudioEvent::DeviceRemoved { id });
        }
        Ok(())
    }

    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, id: &PCWSTR) -> Result<()> {
        if let (Some(flow), Some(role)) = (from_edataflow(flow), from_erole(role)) {
            (self.sink)(AudioEvent::DefaultChanged { flow, role, id: id_of(id) });
        }
        Ok(())
    }

    fn OnPropertyValueChanged(&self, id: &PCWSTR, key: &PROPERTYKEY) -> Result<()> {
        if let Some(id) = id_of(id) {
            (self.sink)(AudioEvent::PropertyChanged { id, key: key_to_string(key) });
        }
        Ok(())
    }
}

/// Keeps the notification client registered; unregisters on drop.
pub struct Registration {
    en: IMMDeviceEnumerator,
    client: IMMNotificationClient,
}

/// Registers for endpoint notifications. `sink` runs on MMDevAPI threads and must not block.
///
/// # Errors
/// Fails if registration is rejected by MMDevAPI.
pub fn register(en: &IMMDeviceEnumerator, sink: impl Fn(AudioEvent) + Send + Sync + 'static) -> Result<Registration> {
    let client: IMMNotificationClient = Client { sink: Box::new(sink) }.into();
    // SAFETY: `client` is a valid COM object kept alive by `Registration` until unregistered.
    unsafe { en.RegisterEndpointNotificationCallback(&client)? };
    Ok(Registration { en: en.clone(), client })
}

impl Drop for Registration {
    fn drop(&mut self) {
        // SAFETY: unregisters the same client registered in `register`.
        unsafe {
            let _ = self.en.UnregisterEndpointNotificationCallback(&self.client);
        }
    }
}
