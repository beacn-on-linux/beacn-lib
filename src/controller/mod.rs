use crate::common::{BeacnDeviceInfo, BeacnDeviceKind, find_device};
use crate::controller::common::{
    BeacnControlAPI, BeacnControlDeviceInfo, BeacnControlDeviceInternal,
};
use crate::controller::device_kind::BeacnDevice;
use crate::manager::{DeviceLocation, DeviceType, PID_BEACN_MIX, PID_BEACN_MIX_CREATE};
use crate::sealed::Sealed;
use crate::types::RGBA;
use crate::{BResult, beacn_bail};
use enum_map::Enum;
use flume::Sender;
use std::panic::RefUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use strum::{Display, EnumIter};

mod common;
mod device;
pub mod device_kind;

#[allow(private_interfaces)]
pub type BeacnMix = BeacnDevice<BeacnMixKind>;
pub struct BeacnMixKind;
impl BeacnDeviceKind for BeacnMixKind {
    const PID: &[u16] = PID_BEACN_MIX;
    const TYPE: DeviceType = DeviceType::BeacnMix;
}

#[allow(private_interfaces)]
pub type BeacnMixCreate = BeacnDevice<BeacnMixCreateKind>;
pub struct BeacnMixCreateKind;
impl BeacnDeviceKind for BeacnMixCreateKind {
    const PID: &[u16] = PID_BEACN_MIX_CREATE;
    const TYPE: DeviceType = DeviceType::BeacnMixCreate;
}

#[allow(private_bounds)]
pub trait BeacnControlDevice:
    BeacnDeviceInfo
    + BeacnControlDeviceInfo
    + BeacnControlDeviceInternal
    + BeacnControlAPI
    + Sealed
    + RefUnwindSafe
    + Sync
    + Send
{
}

pub async fn open_control_device(
    location: DeviceLocation,
    interaction: Option<Sender<Interactions>>,
    health_tx: Sender<()>,
) -> BResult<Arc<Box<dyn BeacnControlDevice>>> {
    let Some(device) = find_device(location).await else {
        beacn_bail!("Device not found");
    };

    let pid = device.descriptor.product_id();
    match pid {
        _ if PID_BEACN_MIX.contains(&pid) => {
            BeacnMix::connect(device, interaction, health_tx).await
        }
        _ if PID_BEACN_MIX_CREATE.contains(&pid) => {
            BeacnMixCreate::connect(device, interaction, health_tx).await
        }
        _ => beacn_bail!("Unknown Device"),
    }
}

// These are some helper enums, generally used in messaging :)
#[derive(Display, Debug, Copy, Clone, PartialEq)]
pub enum Interactions {
    ButtonPress(Buttons, ButtonState),
    DialChanged(Dials, i8),
}

#[derive(Display, Debug, Copy, Clone, Enum, EnumIter, PartialEq)]
pub enum ButtonState {
    Press,
    Release,
}

#[derive(Display, Debug, Copy, Clone, Enum, EnumIter, PartialEq)]
pub enum Buttons {
    AudienceMix = 0,

    PageLeft = 1,
    PageRight = 2,

    Dial1 = 8,
    Dial2 = 9,
    Dial3 = 10,
    Dial4 = 11,

    Audience1 = 12,
    Audience2 = 13,
    Audience3 = 14,
    Audience4 = 15,
}

#[derive(Display, Debug, Copy, Clone, Enum, EnumIter, PartialEq)]
pub enum Dials {
    Dial1 = 0,
    Dial2 = 1,
    Dial3 = 2,
    Dial4 = 3,
}

#[derive(Display, Debug, Copy, Clone, Enum, EnumIter, PartialEq)]
pub enum ButtonLighting {
    Dial1 = 0,
    Dial2 = 1,
    Dial3 = 2,
    Dial4 = 3,

    Mix = 4,
    Left = 5,
    Right = 6,
}

#[derive(Display, Debug)]
pub enum ControlThreadSender {
    Stop,
    KeepAlive(oneshot::Sender<()>),
    SetEnabled(bool, oneshot::Sender<()>),
    SetImage(u32, u32, Vec<u8>, oneshot::Sender<()>),
    SetDimTimeout(Duration, oneshot::Sender<()>),
    SetActiveBrightness(u8, oneshot::Sender<()>),
    SetButtonBrightness(u8, oneshot::Sender<()>),
    SetButtonColour(u8, RGBA, oneshot::Sender<()>),
}
