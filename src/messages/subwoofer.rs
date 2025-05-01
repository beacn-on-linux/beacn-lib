use crate::generate_range;
use crate::messages::{Message, BeacnSubMessage};
use crate::types::{read_value, write_value, BeacnValue, Percent, ReadBeacn, WriteBeacn};

#[derive(Debug)]
pub enum Subwoofer {
    GetMakeupGain,
    MakeupGain(SubwooferMakeupGain),

    GetRatio,
    Ratio(SubwooferRatio),

    GetMix,
    Mix(Percent),

    GetEnabled,
    Enabled(bool),

    GetAmount,
    Amount(SubwooferAmount),
}

impl BeacnSubMessage for Subwoofer {
    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Subwoofer::GetMakeupGain | Subwoofer::MakeupGain(_) => [0x04, 0x00],
            Subwoofer::GetRatio | Subwoofer::Ratio(_) => [0x05, 0x00],
            Subwoofer::GetMix | Subwoofer::Mix(_) => [0x0b, 0x00],
            Subwoofer::GetEnabled | Subwoofer::Enabled(_) => [0x0c, 0x00],
            Subwoofer::GetAmount | Subwoofer::Amount(_) => [0x0e, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Subwoofer::MakeupGain(v) => write_value(v),
            Subwoofer::Ratio(v) => write_value(v),
            Subwoofer::Mix(v) => write_value(v),
            Subwoofer::Enabled(v) => v.write_beacn(),
            Subwoofer::Amount(v) => write_value(v),
            _ => panic!("Attempted to Set a Getter")
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue) -> Self {
        match key[0] {
            0x04 => Self::MakeupGain(read_value(&value)),
            0x05 => Self::Ratio(read_value(&value)),
            0x0b => Self::Mix(read_value(&value)),
            0x0c => Self::Enabled(bool::read_beacn(&value)),
            0x0e => Self::Amount(read_value(&value)),
            _ => panic!("Unexpected Key: {}", key[0])
        }
    }

    fn generate_fetch_message() -> Vec<Message> {
        vec![
            Message::Subwoofer(Subwoofer::GetEnabled),
            Message::Subwoofer(Subwoofer::GetRatio),
            Message::Subwoofer(Subwoofer::GetAmount),
            Message::Subwoofer(Subwoofer::GetMakeupGain),
            Message::Subwoofer(Subwoofer::GetMix),
        ]
    }
}

generate_range!(SubwooferMakeupGain, f32, 2.0..=11.0);
generate_range!(SubwooferRatio, f32, 0.0..=12.0);
generate_range!(SubwooferAmount, i32, 0..=10);

// enum Subwoofer_ {
//     MakeupGain = 0x04, // f32 (2..=11), Value: (amount < 6) ? 2 : amount + 1
//     Ratio = 0x05,      // f32 (0..=12), Value: 12 - amount
//     Mix = 0x0b,        // f32 (1..=100), Value = amount * 10
//     Enabled = 0x0c,    // bool
//     Amount = 0x0e,     // int (0..10)
// }
