use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message, VERSION_ALL};
use crate::manager::DeviceType;
use crate::types::{BeacnValue, Percent, ReadBeacn, WriteBeacn, read_value, write_value};
use crate::version::VersionNumber;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DeEsser {
    GetAmount,
    Amount(Percent),

    GetEnabled,
    Enabled(bool),
}

impl BeacnSubMessage for DeEsser {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn get_message_minimum_version(&self) -> VersionNumber {
        VERSION_ALL
    }
    
    fn is_device_message_set(&self) -> bool {
        matches!(self, DeEsser::Enabled(_) | DeEsser::Amount(_))
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            DeEsser::Amount(_) | DeEsser::GetAmount => [0x03, 0x00],
            DeEsser::Enabled(_) | DeEsser::GetEnabled => [0x04, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            DeEsser::Amount(v) => write_value(v),
            DeEsser::Enabled(v) => v.write_beacn(),
            _ => panic!("Attmpted to Set a Get"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType) -> Self {
        match key[0] {
            0x03 => Self::Amount(read_value(&value)),
            0x04 => Self::Enabled(bool::read_beacn(&value)),
            _ => panic!("Unexpected Key: {}", key[0]),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType) -> Vec<Message> {
        vec![
            Message::DeEsser(DeEsser::GetAmount),
            Message::DeEsser(DeEsser::GetEnabled),
        ]
    }
}
