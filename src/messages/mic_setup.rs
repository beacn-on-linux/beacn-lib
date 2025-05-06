use crate::generate_range;
use crate::manager::DeviceType;
use crate::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::types::{BeacnValue, read_value, write_value, WriteBeacn, ReadBeacn};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MicSetup {
    GetMicGain,
    MicGain(MicGain),

    GetStudioMicGain,
    StudioMicGain(StudioMicGain),

    GetStudioPhantomPower,
    StudioPhantomPower(bool),
}

impl BeacnSubMessage for MicSetup {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            MicSetup::GetMicGain | MicSetup::MicGain(_) => [0x00, 0x00],
            MicSetup::GetStudioMicGain | MicSetup::StudioMicGain(_) => [0x00, 0x00],
            MicSetup::GetStudioPhantomPower | MicSetup::StudioPhantomPower(_) => [0x02, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            MicSetup::MicGain(v) => write_value(v),
            MicSetup::StudioMicGain(v) => write_value(v),
            MicSetup::StudioPhantomPower(v) => v.write_beacn(),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, device_type: DeviceType) -> Self {
        match key[0] {
            0x00 => {
                match device_type {
                    DeviceType::BeacnMic => Self::MicGain(read_value(&value)),
                    DeviceType::BeacnStudio => Self::StudioMicGain(read_value(&value))
                }
            },
            0x02 => Self::StudioPhantomPower(bool::read_beacn(&value)),
            _ => panic!("Unknown Key")
        }
    }

    fn generate_fetch_message(device_type: DeviceType) -> Vec<Message> {
        match device_type {
            DeviceType::BeacnMic => vec![
                Message::MicSetup(MicSetup::GetMicGain)
            ],
            DeviceType::BeacnStudio => vec![
                Message::MicSetup(MicSetup::GetStudioMicGain),
                Message::MicSetup(MicSetup::GetStudioPhantomPower),
            ]
        }
    }
}

generate_range!(MicGain, u32, 3..=20);
generate_range!(StudioMicGain, u32, 0..=69);    // NICE.