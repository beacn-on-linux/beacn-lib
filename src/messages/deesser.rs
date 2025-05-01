use crate::messages::{Message, BeacnSubMessage};
use crate::types::{read_value, write_value, BeacnValue, Percent, ReadBeacn, WriteBeacn};

#[derive(Debug)]
pub enum DeEsser {
    GetAmount,
    Amount(Percent),

    GetEnabled,
    Enabled(bool),
}

impl BeacnSubMessage for DeEsser {
    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            DeEsser::Amount(_) | DeEsser::GetAmount => [0x03, 0x00],
            DeEsser::Enabled(_) | DeEsser::GetEnabled => [0x04, 0x00]
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            DeEsser::Amount(v) => write_value(v),
            DeEsser::Enabled(v) => v.write_beacn(),
            _ => panic!("Attmpted to Set a Get")
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue) -> Self {
        match key[0] {
            0x03 => Self::Amount(read_value(&value)),
            0x04 => Self::Enabled(bool::read_beacn(&value)),
            _ => panic!("Unexpected Key: {}", key[0])
        }
    }

    fn generate_fetch_message() -> Vec<Message> {
        vec![
            Message::DeEsser(DeEsser::GetAmount),
            Message::DeEsser(DeEsser::GetEnabled)
        ]
    }
}