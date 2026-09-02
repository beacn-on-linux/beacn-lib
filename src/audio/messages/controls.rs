use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::manager::DeviceType;
use crate::types::{BeacnValue, ReadBeacn, WriteBeacn, read_value, write_value};
use crate::version::VersionNumber;
use crate::{EQ_HEADPHONES_VERSION, generate_range, message_group};
use serde::{Deserialize, Serialize};

message_group!(
    pub enum Controls {
        Mono() -> bool,
        Balance() -> Balance,
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

    fn to_beacn_key(&self, _: VersionNumber) -> [u8; 2] {
        match self {
            Controls::Mono(_) | Controls::GetMono => [0x01, 0x00],
            Controls::Balance(_) | Controls::GetBalance => [0x00, 0x00],
        }
    }

    fn to_beacn_value(&self, _: VersionNumber) -> BeacnValue {
        match self {
            Controls::Mono(v) => v.write_beacn(),
            // This needs to send as an f32, so use the internal variant
            Controls::Balance(v) => write_value(&BalanceInternal::from(v.0)),
            _ => panic!("Attempting to Set value for Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _: DeviceType, _: VersionNumber) -> Self {
        match key[0] {
            0x00 => Self::Balance(read_value(&value)),
            0x01 => Self::Mono(bool::read_beacn(&value)),
            _ => panic!("Unexpected Key: {}", key[0]),
        }
    }

    fn should_validate_response(&self) -> bool {
        // Don't error on validation for the Balance
        !matches!(self, Controls::Balance(_))
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

generate_range!(Balance, i32, -100..=100, i8, f32);
generate_range!(BalanceInternal, f32, -100.0..=100.0, i32, i8);
