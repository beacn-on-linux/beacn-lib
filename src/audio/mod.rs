mod common;
pub mod messages;
mod mic;
mod studio;

use crate::audio::common::{BeacnAudioDeviceAttach, BeacnAudioMessageExecute, BeacnAudioMessaging};
use crate::audio::mic::BeacnMic;
use crate::audio::studio::BeacnStudio;
use crate::common::{DeviceDefinition, find_device};
use crate::manager::{DeviceLocation, PID_BEACN_MIC, PID_BEACN_STUDIO};
use crate::{BResult, beacn_bail};
use enum_map::Enum;
use std::panic::RefUnwindSafe;
use strum::EnumIter;

pub trait BeacnAudioDevice:
    BeacnAudioDeviceAttach + BeacnAudioMessageExecute + BeacnAudioMessaging + RefUnwindSafe
{
}

pub fn open_audio_device(location: DeviceLocation) -> BResult<Box<dyn BeacnAudioDevice>> {
    if let Some(device) = find_device(location) {
        // We need to return the correct type
        return if PID_BEACN_MIC.contains(&device.descriptor.product_id()) {
            BeacnMic::connect(device)
        } else if PID_BEACN_STUDIO.contains(&device.descriptor.product_id()) {
            BeacnStudio::connect(device)
        } else {
            beacn_bail!("Unknown Device")
        }
    }
    beacn_bail!("Unknown Device")
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct LinkedApp {
    pub channel: LinkChannel,
    pub name: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Enum, EnumIter)]
pub enum LinkChannel {
    System,
    Link1,
    Link2,
    Link3,
    Link4,
}

impl LinkChannel {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => LinkChannel::Link1,
            2 => LinkChannel::Link2,
            3 => LinkChannel::Link3,
            4 => LinkChannel::Link4,
            _ => LinkChannel::System,
        }
    }
}
