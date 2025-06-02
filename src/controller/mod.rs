use crate::common::find_device;
use crate::controller::common::{BeacnControlDeviceAttach, BeacnControlInteraction};
use crate::controller::mix::BeacnMix;
use crate::controller::mix_create::BeacnMixCreate;
use crate::manager::{DeviceLocation, PID_BEACN_MIX, PID_BEACN_MIX_CREATE};
use anyhow::Result;
use anyhow::bail;
use enum_map::Enum;
use std::panic::RefUnwindSafe;
use std::sync::mpsc::Sender;
use strum::{Display, EnumIter};

mod common;
mod mix;
mod mix_create;
// BeacnAudioMessageExecute + BeacnAudioMessaging +

pub trait BeacnControlDevice: BeacnControlDeviceAttach + BeacnControlInteraction + RefUnwindSafe {}

pub fn open_control_device(
    location: DeviceLocation,
    interaction: Sender<Interactions>,
) -> Result<Box<dyn BeacnControlDevice>> {
    if let Some(device) = find_device(location) {
        // We need to return the correct type
        return match device.descriptor.product_id() {
            PID_BEACN_MIX => BeacnMix::connect(device, interaction),
            PID_BEACN_MIX_CREATE => BeacnMixCreate::connect(device, interaction),
            _ => bail!("Unknown Device"),
        };
    }
    bail!("Unknown Device")
}

// These are some helper enums, generally used in messaging :)
#[derive(Display, Copy, Clone, PartialEq)]
pub enum Interactions {
    ButtonPress(Buttons, ButtonState),
    DialChanged(Dials, i8),
}

#[derive(Display, Copy, Clone, EnumIter, PartialEq)]
pub enum ButtonState {
    Press,
    Release,
}

#[derive(Display, Copy, Clone, EnumIter, PartialEq)]
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
enum ButtonLighting {
    Dial1 = 0,
    Dial2 = 1,
    Dial3 = 2,
    Dial4 = 3,

    Mix = 4,
    Left = 5,
    Right = 6,
}

enum ControlThreadManager {
    STOP,
}