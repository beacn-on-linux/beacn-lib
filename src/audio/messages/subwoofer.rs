use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message, VERSION_ALL};
use crate::generate_range;
use crate::manager::DeviceType;
use crate::types::{BeacnValue, Percent, ReadBeacn, WriteBeacn, read_value, write_value};
use crate::version::VersionNumber;

#[derive(Copy, Clone, Debug, PartialEq)]
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
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn get_message_minimum_version(&self) -> VersionNumber {
        VERSION_ALL
    }

    fn is_device_message_set(&self) -> bool {
        matches!(
            self,
            Subwoofer::MakeupGain(_)
                | Subwoofer::Ratio(_)
                | Subwoofer::Mix(_)
                | Subwoofer::Enabled(_)
                | Subwoofer::Amount(_)
        )
    }

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
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType) -> Self {
        match key[0] {
            0x04 => Self::MakeupGain(read_value(&value)),
            0x05 => Self::Ratio(read_value(&value)),
            0x0b => Self::Mix(read_value(&value)),
            0x0c => Self::Enabled(bool::read_beacn(&value)),
            0x0e => Self::Amount(read_value(&value)),
            _ => panic!("Unexpected Key: {}", key[0]),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType) -> Vec<Message> {
        vec![
            Message::Subwoofer(Subwoofer::GetEnabled),
            Message::Subwoofer(Subwoofer::GetRatio),
            Message::Subwoofer(Subwoofer::GetAmount),
            Message::Subwoofer(Subwoofer::GetMakeupGain),
            Message::Subwoofer(Subwoofer::GetMix),
        ]
    }
}

impl Subwoofer {
    pub fn get_amount_messages(amount: u8) -> Vec<Message> {
        let gain = if amount < 6 { 2 } else { amount + 1 };
        let ratio = 12 - amount;
        let mix = amount * 10;

        let messages = vec![
            Message::Subwoofer(Subwoofer::Amount(SubwooferAmount(amount as i32))),
            Message::Subwoofer(Subwoofer::Mix(Percent(mix as f32))),
            Message::Subwoofer(Subwoofer::Ratio(SubwooferRatio(ratio as f32))),
            Message::Subwoofer(Subwoofer::MakeupGain(SubwooferMakeupGain(gain as f32))),
        ];

        messages
    }
}

generate_range!(SubwooferMakeupGain, f32, 2.0..=12.0);
generate_range!(SubwooferRatio, f32, 0.0..=12.0);
generate_range!(SubwooferAmount, i32, 0..=10);

// enum Subwoofer_ {
//     MakeupGain = 0x04, // f32 (2..=11), Value: (amount < 6) ? 2 : amount + 1
//     Ratio = 0x05,      // f32 (0..=12), Value: 12 - amount
//     Mix = 0x0b,        // f32 (1..=100), Value = amount * 10
//     Enabled = 0x0c,    // bool
//     Amount = 0x0e,     // int (0..10)
// }
