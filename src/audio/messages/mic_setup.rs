use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message, VERSION_ALL};
use crate::generate_range;
use crate::manager::DeviceType;
use crate::types::{BeacnValue, ReadBeacn, WriteBeacn, read_value, write_value};
use crate::version::VersionNumber;

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
        match self {
            MicSetup::GetMicGain => DeviceMessageType::BeacnMic,
            MicSetup::MicGain(_) => DeviceMessageType::BeacnMic,
            MicSetup::GetStudioMicGain => DeviceMessageType::BeacnStudio,
            MicSetup::StudioMicGain(_) => DeviceMessageType::BeacnStudio,
            MicSetup::GetStudioPhantomPower => DeviceMessageType::BeacnStudio,
            MicSetup::StudioPhantomPower(_) => DeviceMessageType::BeacnStudio,
        }
    }

    fn get_message_minimum_version(&self) -> VersionNumber {
        VERSION_ALL
    }

    fn is_device_message_set(&self) -> bool {
        matches!(
            self,
            MicSetup::MicGain(_) | MicSetup::StudioMicGain(_) | MicSetup::StudioPhantomPower(_)
        )
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
            0x00 => match device_type {
                DeviceType::BeacnMic => Self::MicGain(read_value(&value)),
                DeviceType::BeacnStudio => Self::StudioMicGain(read_value(&value)),
                _ => panic!("This isn't an Audio Device!"),
            },
            0x02 => Self::StudioPhantomPower(bool::read_beacn(&value)),
            _ => panic!("Unknown Key"),
        }
    }

    fn generate_fetch_message(device_type: DeviceType) -> Vec<Message> {
        match device_type {
            DeviceType::BeacnMic => vec![Message::MicSetup(MicSetup::GetMicGain)],
            DeviceType::BeacnStudio => vec![
                Message::MicSetup(MicSetup::GetStudioMicGain),
                Message::MicSetup(MicSetup::GetStudioPhantomPower),
            ],
            _ => panic!("This isn't an Audio Device!"),
        }
    }
}

generate_range!(MicGain, u32, 3..=20);
generate_range!(StudioMicGain, u32, 0..=69); // NICE.
