mod common;
pub mod messages;
mod mic;
mod studio;

use crate::audio::common::{BeacnAudioDeviceAttach, BeacnAudioMessageExecute, BeacnAudioMessaging};
use crate::audio::mic::BeacnMic;
use crate::audio::studio::BeacnStudio;
use crate::common::{DeviceDefinition, find_device};
use crate::manager::{DeviceLocation, PID_BEACN_MIC, PID_BEACN_STUDIO};
use anyhow::{Result, bail};
use std::panic::RefUnwindSafe;

pub trait BeacnAudioDevice:
    BeacnAudioDeviceAttach + BeacnAudioMessageExecute + BeacnAudioMessaging + RefUnwindSafe
{
}

pub fn open_audio_device(location: DeviceLocation) -> Result<Box<dyn BeacnAudioDevice>> {
    if let Some(device) = find_device(location) {
        // We need to return the correct type
        return match device.descriptor.product_id() {
            PID_BEACN_MIC => BeacnMic::connect(device),
            PID_BEACN_STUDIO => BeacnStudio::connect(device),
            _ => bail!("Unknown Device"),
        };
    }
    bail!("Unknown Device")
}
