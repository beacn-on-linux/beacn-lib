use crate::common::find_device;
use crate::controller::common::BeacnControlDeviceAttach;
use crate::controller::mix::BeacnMix;
use crate::controller::mix_create::BeacnMixCreate;
use crate::manager::{DeviceLocation, PID_BEACN_MIX, PID_BEACN_MIX_CREATE};
use anyhow::Result;
use anyhow::bail;
use std::panic::RefUnwindSafe;

mod common;
mod mix;
mod mix_create;
// BeacnAudioMessageExecute + BeacnAudioMessaging +

pub trait BeacnControlDevice: BeacnControlDeviceAttach + RefUnwindSafe {}

pub fn open_control_device(location: DeviceLocation) -> Result<Box<dyn BeacnControlDevice>> {
    if let Some(device) = find_device(location) {
        // We need to return the correct type
        return match device.descriptor.product_id() {
            PID_BEACN_MIX => BeacnMix::connect(device),
            PID_BEACN_MIX_CREATE => BeacnMixCreate::connect(device),
            _ => bail!("Unknown Device"),
        };
    }
    bail!("Unknown Device")
}
