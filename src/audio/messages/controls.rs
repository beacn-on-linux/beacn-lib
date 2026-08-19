use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::manager::DeviceType;
use crate::types::{BeacnValue, ReadBeacn, WriteBeacn};
use crate::version::VersionNumber;
use crate::{EQ_HEADPHONES_VERSION, generate_range, message_group};
use serde::{Deserialize, Serialize};

message_group!(
    pub enum Controls {
        Mono() -> bool,
        Balance() -> f32,
    }
);

impl BeacnSubMessage for Controls {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn get_message_minimum_version(&self) -> VersionNumber {
        EQ_HEADPHONES_VERSION
    }

    fn is_device_message_set(&self) -> bool {
        self.is_message_set()
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Controls::Mono(_) | Controls::GetMono => [0x01, 0x00],
            Controls::Balance(_) | Controls::GetBalance => [0x00, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Controls::Mono(v) => v.write_beacn(),
            Controls::Balance(v) => v.write_beacn(),
            _ => panic!("Attempting to Set value for Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _: DeviceType) -> Self {
        match key[0] {
            0x00 => Self::Balance(f32::read_beacn(&value)),
            0x01 => Self::Mono(bool::read_beacn(&value)),
            _ => panic!("Unexpected Key: {}", key[0]),
        }
    }

    fn generate_fetch_message(_: DeviceType, version: VersionNumber) -> Vec<Message> {
        if version < EQ_HEADPHONES_VERSION {
            return vec![];
        }

        vec![
            Message::Controls(Controls::GetMono),
            Message::Controls(Controls::GetBalance),
        ]
    }
}

generate_range!(Balance, f32, -100.0..=100.0, i8);
