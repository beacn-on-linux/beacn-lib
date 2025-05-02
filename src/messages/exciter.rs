use crate::generate_range;
use crate::messages::{BeacnSubMessage, Message};
use crate::types::{BeacnValue, Percent, ReadBeacn, WriteBeacn, read_value, write_value};
use enum_map::Enum;
use strum::EnumIter;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Exciter {
    GetAmount,
    Amount(Percent),

    GetFrequency,
    Frequency(ExciterFrequency),

    GetEnabled,
    Enabled(bool),
}

impl BeacnSubMessage for Exciter {
    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Exciter::Amount(_) | Exciter::GetAmount => [0x01, 0x00],
            Exciter::Frequency(_) | Exciter::GetFrequency => [0x02, 0x00],
            Exciter::Enabled(_) | Exciter::GetEnabled => [0x03, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Exciter::Amount(v) => write_value(v),
            Exciter::Frequency(v) => write_value(v),
            Exciter::Enabled(v) => v.write_beacn(),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue) -> Self {
        match key[0] {
            0x01 => Self::Amount(read_value(&value)),
            0x02 => Self::Frequency(read_value(&value)),
            0x03 => Self::Enabled(bool::read_beacn(&value)),
            _ => panic!("Couldn't Find Key {}", key[0]),
        }
    }

    fn generate_fetch_message() -> Vec<Message> {
        vec![
            Message::Exciter(Exciter::GetAmount),
            Message::Exciter(Exciter::GetFrequency),
            Message::Exciter(Exciter::GetEnabled),
        ]
    }
}

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum ExciterKeys {
    Amount = 0x01,    // f32 (0..=100)
    Frequency = 0x02, // f32 (0..=5000)
    Enabled = 0x03,   // bool
}

generate_range!(ExciterFrequency, f32, 0.0..=5000.0);
