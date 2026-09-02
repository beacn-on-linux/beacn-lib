use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message, VERSION_MAX_ALL};
use crate::manager::DeviceType;
use crate::types::{BeacnValue, Percent, ReadBeacn, WriteBeacn, read_value, write_value};
use crate::version::VersionNumber;
use crate::{EQ_HEADPHONES_VERSION, generate_range, message_group};
use serde::{Deserialize, Serialize};

message_group!(
    pub enum Subwoofer {
        MakeupGain() -> SubwooferMakeupGain,
        Ratio() -> SubwooferRatio,
        Mix() -> Percent,
        Enabled() -> bool,
        Amount() -> SubwooferAmount,
    }
);

impl BeacnSubMessage for Subwoofer {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn get_message_maximum_version(&self) -> VersionNumber {
        // For everything that's not Enabled and Amount, the messages aren't available after 1.3
        if matches!(
            self,
            Subwoofer::Enabled(_)
                | Subwoofer::GetEnabled
                | Subwoofer::Amount(_)
                | Subwoofer::GetAmount
        ) {
            VERSION_MAX_ALL
        } else {
            EQ_HEADPHONES_VERSION
        }
    }

    fn is_device_message_set(&self) -> bool {
        self.is_message_set()
    }

    fn to_beacn_key(&self, v: VersionNumber) -> [u8; 2] {
        if v < EQ_HEADPHONES_VERSION {
            match self {
                Subwoofer::GetMakeupGain | Subwoofer::MakeupGain(_) => [0x04, 0x00],
                Subwoofer::GetRatio | Subwoofer::Ratio(_) => [0x05, 0x00],
                Subwoofer::GetMix | Subwoofer::Mix(_) => [0x0b, 0x00],
                Subwoofer::GetEnabled | Subwoofer::Enabled(_) => [0x0c, 0x00],
                Subwoofer::GetAmount | Subwoofer::Amount(_) => [0x0e, 0x00],
            }
        } else {
            match self {
                Subwoofer::GetEnabled | Subwoofer::Enabled(_) => [0x03, 0x00],
                Subwoofer::GetAmount | Subwoofer::Amount(_) => [0x04, 0x00],
                _ => panic!("Attempted to Get a Setter"),
            }
        }
    }

    fn to_beacn_value(&self, vn: VersionNumber) -> BeacnValue {
        match self {
            Subwoofer::MakeupGain(v) => write_value(v),
            Subwoofer::Ratio(v) => write_value(v),
            Subwoofer::Mix(v) => write_value(v),
            Subwoofer::Enabled(v) => v.write_beacn(),
            Subwoofer::Amount(v) => {
                if vn < EQ_HEADPHONES_VERSION {
                    write_value(v)
                } else {
                    let value = SubwooferAmountInternal(v.0 as f32);
                    write_value(&value)
                }
            }
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _: DeviceType, v: VersionNumber) -> Self {
        if v < EQ_HEADPHONES_VERSION {
            match key[0] {
                0x04 => Self::MakeupGain(read_value(&value)),
                0x05 => Self::Ratio(read_value(&value)),
                0x0b => Self::Mix(read_value(&value)),
                0x0c => Self::Enabled(bool::read_beacn(&value)),
                0x0e => Self::Amount(read_value(&value)),
                _ => panic!("Unexpected Key: {}", key[0]),
            }
        } else {
            match key[0] {
                0x03 => Self::Enabled(bool::read_beacn(&value)),
                0x04 => Self::Amount({
                    let interim: SubwooferAmountInternal = read_value(&value);
                    SubwooferAmount(interim.0 as i32)
                }),
                _ => panic!("Unexpected Key: {}", key[0]),
            }
        }
    }

    fn generate_fetch_message(_device_type: DeviceType, v: VersionNumber) -> Vec<Message> {
        if v < EQ_HEADPHONES_VERSION {
            vec![
                Message::Subwoofer(Subwoofer::GetEnabled),
                Message::Subwoofer(Subwoofer::GetRatio),
                Message::Subwoofer(Subwoofer::GetAmount),
                Message::Subwoofer(Subwoofer::GetMakeupGain),
                Message::Subwoofer(Subwoofer::GetMix),
            ]
        } else {
            vec![
                Message::Subwoofer(Subwoofer::GetEnabled),
                Message::Subwoofer(Subwoofer::GetAmount),
            ]
        }
    }
}

impl Subwoofer {
    pub fn get_amount_messages(amount: u8, v: VersionNumber) -> Vec<Message> {
        if v < EQ_HEADPHONES_VERSION {
            let gain = if amount < 6 { 2 } else { amount + 1 };
            let ratio = 12 - amount;
            let mix = amount * 10;

            vec![
                Message::Subwoofer(Subwoofer::Amount(SubwooferAmount(amount as i32))),
                Message::Subwoofer(Subwoofer::Mix(Percent(mix as f32))),
                Message::Subwoofer(Subwoofer::Ratio(SubwooferRatio(ratio as f32))),
                Message::Subwoofer(Subwoofer::MakeupGain(SubwooferMakeupGain(gain as f32))),
            ]
        } else {
            vec![Message::Subwoofer(Subwoofer::Amount(SubwooferAmount(
                amount as i32,
            )))]
        }
    }
}

generate_range!(SubwooferMakeupGain, f32, 0.0..=12.0);
generate_range!(SubwooferRatio, f32, 0.0..=12.0);
generate_range!(SubwooferAmount, i32, 0..=10, u8);
generate_range!(SubwooferAmountInternal, f32, 0.0..=10.0, u8, i32, u32);
