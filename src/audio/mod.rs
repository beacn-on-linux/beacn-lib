mod common;
pub mod messages;
mod mic;
mod studio;

use std::panic::RefUnwindSafe;
use crate::audio::common::{BeacnAudioDeviceAttach, BeacnAudioMessageExecute, BeacnAudioMessaging};
use crate::audio::mic::BeacnMic;
use crate::audio::studio::BeacnStudio;
use crate::manager::{DeviceLocation, PID_BEACN_MIC, PID_BEACN_STUDIO, VENDOR_BEACN};
use anyhow::{Result, bail};
use rusb::{Device, DeviceDescriptor, GlobalContext};

struct DeviceDefinition {
    device: Device<GlobalContext>,
    descriptor: DeviceDescriptor,
}

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

fn find_device(location: DeviceLocation) -> Option<DeviceDefinition> {
    // We need to iterate through the devices and find the one at this location
    if let Ok(devices) = rusb::devices() {
        for device in devices.iter() {
            if let Ok(descriptor) = device.device_descriptor() {
                #[allow(clippy::collapsible_if)]
                if descriptor.vendor_id() == VENDOR_BEACN {
                    if DeviceLocation::from(device.clone()) == location {
                        return Some(DeviceDefinition { device, descriptor });
                    }
                }
            }
        }
    }
    None
}
