// Adapt time is interesting, and may actually require listening to the audio from the Mic.
// During set up I saw (f32) 100 -> 1000 -> 2000 -> 5000, before finally settling on 1000, I'm
// assuming these values are in milliseconds.
// I did *NOT* during this time check data received, I might need to ask Beacn how this is handled.

use crate::generate_range;
use crate::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::types::sealed::Sealed;
use crate::types::{BeacnValue, Percent, ReadBeacn, WriteBeacn, read_value, write_value};
use byteorder::{ByteOrder, LittleEndian};
use enum_map::Enum;
use strum::{EnumIter, IntoEnumIterator};
use crate::manager::DeviceType;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Suppressor {
    GetEnabled,
    Enabled(bool),

    GetAmount,
    Amount(Percent),

    GetStyle,
    Style(SuppressorStyle),

    GetSensitivity,
    Sensitivity(SuppressorSensitivity),

    GetAdaptTime,
    AdaptTime(SupressorAdaptTime),
}

impl BeacnSubMessage for Suppressor {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Suppressor::GetEnabled | Suppressor::Enabled(_) => [0x00, 0x00],
            Suppressor::GetAmount | Suppressor::Amount(_) => [0x02, 0x00],
            Suppressor::GetStyle | Suppressor::Style(_) => [0x04, 0x00],
            Suppressor::GetSensitivity | Suppressor::Sensitivity(_) => [0x05, 0x00],
            Suppressor::GetAdaptTime | Suppressor::AdaptTime(_) => [0x08, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Suppressor::Enabled(v) => v.write_beacn(),
            Suppressor::Amount(v) => write_value(v),
            Suppressor::Style(v) => v.write_beacn(),
            Suppressor::Sensitivity(v) => write_value(v),
            Suppressor::AdaptTime(v) => write_value(v),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType) -> Self {
        match key[0] {
            0x00 => Self::Enabled(bool::read_beacn(&value)),
            0x02 => Self::Amount(read_value(&value)),
            0x04 => Self::Style(SuppressorStyle::read_beacn(&value)),
            0x05 => Self::Sensitivity(read_value(&value)),
            0x08 => Self::AdaptTime(read_value(&value)),
            _ => panic!("Unexpected Key {}", key[0]),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType) -> Vec<Message> {
        vec![
            Message::Suppressor(Suppressor::GetEnabled),
            Message::Suppressor(Suppressor::GetAmount),
            Message::Suppressor(Suppressor::GetStyle),
            Message::Suppressor(Suppressor::GetSensitivity),
            Message::Suppressor(Suppressor::GetAdaptTime),
        ]
    }
}

generate_range!(SuppressorSensitivity, f32, -120.0..=-60.0);
generate_range!(SupressorAdaptTime, f32, 100.0..=5000.0);

// enum Suppressor {
//     Enabled = 0x00,
//     Amount = 0x02,      // f32 (0..=100)
//     Style = 0x04,       // SuppressorStyle
//     AdaptTime = 0x08,    // Suppressor Adaption Time
// }

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum SuppressorStyle {
    #[default]
    Off = 0x00,
    Adaptive = 0x01,
    Snapshot = 0x02,
}
impl Sealed for SuppressorStyle {}
impl WriteBeacn for SuppressorStyle {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

impl ReadBeacn for SuppressorStyle {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for var in Self::iter() {
            if var as u32 == value {
                return var;
            }
        }
        panic!("Could not Find Value");
    }
}
