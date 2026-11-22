//! Peak meter (`IAudioMeterInformation`): cheap visualizer input, no audio capture involved.

use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::IMMDevice;
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::core::Result;

pub struct Meter(IAudioMeterInformation);

impl Meter {
    /// # Errors
    /// Fails if the endpoint can't be activated.
    pub fn new(dev: &IMMDevice) -> Result<Self> {
        // SAFETY: documented activation on a valid endpoint.
        Ok(Self(unsafe { dev.Activate(CLSCTX_ALL, None)? }))
    }

    /// Current peak sample value (0.0–1.0) across channels.
    ///
    /// # Errors
    /// Fails if the endpoint was invalidated.
    pub fn peak(&self) -> Result<f32> {
        // SAFETY: plain getter on a valid interface.
        unsafe { self.0.GetPeakValue() }
    }
}
