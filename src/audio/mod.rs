mod common;
pub mod data;
mod device_kind;
pub mod messages;

use crate::audio::common::{BeacnAudioAPI, BeacnAudioDeviceInternal};
use crate::audio::device_kind::BeacnDevice;
use crate::common::{BeacnDeviceInfo, BeacnDeviceKind, DeviceDefinition, find_device};
use crate::manager::{DeviceLocation, DeviceType, PID_BEACN_MIC, PID_BEACN_STUDIO};
use crate::sealed::Sealed;
use crate::{BResult, beacn_bail};
use enum_map::Enum;
use std::panic::RefUnwindSafe;
use strum::EnumIter;

#[allow(private_interfaces)]
pub type BeacnMic = BeacnDevice<BeacnMicKind>;
pub struct BeacnMicKind;
impl BeacnDeviceKind for BeacnMicKind {
    const PID: &[u16] = PID_BEACN_MIC;
    const TYPE: DeviceType = DeviceType::BeacnMic;
}

#[allow(private_interfaces)]
pub type BeacnStudio = BeacnDevice<BeacnStudioKind>;
pub struct BeacnStudioKind;
impl BeacnDeviceKind for BeacnStudioKind {
    const PID: &[u16] = PID_BEACN_STUDIO;
    const TYPE: DeviceType = DeviceType::BeacnStudio;
}

#[allow(private_bounds)]
#[cfg(not(target_arch = "wasm32"))]
pub trait BeacnAudioDevice:
    BeacnDeviceInfo + BeacnAudioDeviceInternal + BeacnAudioAPI + RefUnwindSafe + Sealed + Send + Sync
{
}

#[cfg(target_arch = "wasm32")]
#[allow(private_bounds)]
pub trait BeacnAudioDevice:
    BeacnDeviceInfo + BeacnAudioDeviceInternal + BeacnAudioAPI + RefUnwindSafe + Sealed
{
}

pub async fn open_audio_device(location: DeviceLocation) -> BResult<Box<dyn BeacnAudioDevice>> {
    let Some(device) = find_device(location).await else {
        beacn_bail!("Device not found");
    };

    let pid = device.descriptor.product_id();
    match pid {
        _ if PID_BEACN_MIC.contains(&pid) => BeacnMic::connect(device).await,
        _ if PID_BEACN_STUDIO.contains(&pid) => BeacnStudio::connect(device).await,
        _ => beacn_bail!("Unknown Device"),
    }
}

#[derive(Debug, Clone)]
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
