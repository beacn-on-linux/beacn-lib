use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::manager::DeviceType;
use crate::message_group;
use crate::types::{BeacnValue, Percent, ReadBeacn, WriteBeacn, read_value, write_value};
use crate::version::VersionNumber;

message_group!(
    pub enum DeEsser {
        Amount() -> Percent,
        Enabled() -> bool,
    }
);

impl BeacnSubMessage for DeEsser {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn is_device_message_set(&self) -> bool {
        self.is_message_set()
    }

    fn to_beacn_key(&self, _: VersionNumber) -> [u8; 2] {
        match self {
            DeEsser::Amount(_) | DeEsser::GetAmount => [0x03, 0x00],
            DeEsser::Enabled(_) | DeEsser::GetEnabled => [0x04, 0x00],
        }
    }

    fn to_beacn_value(&self, _: VersionNumber) -> BeacnValue {
        match self {
            DeEsser::Amount(v) => write_value(v),
            DeEsser::Enabled(v) => v.write_beacn(),
            _ => panic!("Attmpted to Set a Get"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType, _: VersionNumber) -> Self {
        match key[0] {
            0x03 => Self::Amount(read_value(&value)),
            0x04 => Self::Enabled(bool::read_beacn(&value)),
            _ => panic!("Unexpected Key: {}", key[0]),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType, _: VersionNumber) -> Vec<Message> {
        vec![
            Message::DeEsser(DeEsser::GetAmount),
            Message::DeEsser(DeEsser::GetEnabled),
        ]
    }
}
