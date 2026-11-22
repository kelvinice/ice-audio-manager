//! Endpoint master volume: read/write and change notifications (`IAudioEndpointVolumeCallback`).
//!
//! This is how the notch learns about volume changes from hardware keys *without* any keyboard hook.

use std::sync::mpsc::Sender;

use ice_core::event::AudioEvent;
use windows::Win32::Media::Audio::AUDIO_VOLUME_NOTIFICATION_DATA;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::IMMDevice;
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::core::{Result, implement};

#[implement(IAudioEndpointVolumeCallback)]
struct Callback {
    tx: Sender<AudioEvent>,
}

impl IAudioEndpointVolumeCallback_Impl for Callback_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        // SAFETY: Windows passes a valid pointer (or null) for the duration of the callback.
        if let Some(d) = unsafe { data.as_ref() } {
            let _ = self.tx.send(AudioEvent::Volume { scalar: d.fMasterVolume, muted: d.bMuted.as_bool() });
        }
        Ok(())
    }
}

fn endpoint_volume(dev: &IMMDevice) -> Result<IAudioEndpointVolume> {
    // SAFETY: documented activation of IAudioEndpointVolume on a valid endpoint.
    unsafe { dev.Activate(CLSCTX_ALL, None) }
}

/// Current master volume (0.0–1.0) and mute state.
///
/// # Errors
/// Fails if the endpoint is invalidated.
pub fn get(dev: &IMMDevice) -> Result<(f32, bool)> {
    let vol = endpoint_volume(dev)?;
    // SAFETY: plain getters on a valid interface.
    unsafe { Ok((vol.GetMasterVolumeLevelScalar()?, vol.GetMute()?.as_bool())) }
}

/// Watches one endpoint's volume; unregisters on drop. Re-create it when the default device changes.
pub struct VolumeWatch {
    vol: IAudioEndpointVolume,
    cb: IAudioEndpointVolumeCallback,
}

impl VolumeWatch {
    /// # Errors
    /// Fails if the endpoint can't be activated or registration is rejected.
    pub fn new(dev: &IMMDevice, tx: Sender<AudioEvent>) -> Result<Self> {
        let vol = endpoint_volume(dev)?;
        let cb: IAudioEndpointVolumeCallback = Callback { tx }.into();
        // SAFETY: `cb` stays alive in `Self` until unregistered in Drop.
        unsafe { vol.RegisterControlChangeNotify(&cb)? };
        Ok(Self { vol, cb })
    }
}

impl Drop for VolumeWatch {
    fn drop(&mut self) {
        // SAFETY: unregisters the callback registered in `new`.
        unsafe {
            let _ = self.vol.UnregisterControlChangeNotify(&self.cb);
        }
    }
}
