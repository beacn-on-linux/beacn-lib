use crate::generate_range;
use crate::messages::{BeacnSubMessage, Message};
use crate::types::{BeacnValue, read_value, write_value};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MicSetup {
    GetMicGain,
    MicGain(MicGain),
}

impl BeacnSubMessage for MicSetup {
    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            MicSetup::GetMicGain | MicSetup::MicGain(_) => [0x00, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            MicSetup::MicGain(v) => write_value(v),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue) -> Self {
        if key == [0x00, 0x00] {
            return Self::MicGain(read_value(&value));
        }
        panic!("Unknown Key: {:?}", key)
    }

    fn generate_fetch_message() -> Vec<Message> {
        vec![Message::MicSetup(MicSetup::GetMicGain)]
    }
}

generate_range!(MicGain, u32, 3..=20);
