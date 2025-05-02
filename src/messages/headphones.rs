use crate::messages::{BeacnSubMessage, Message};

use crate::generate_range;
use crate::types::sealed::Sealed;
use crate::types::{BeacnValue, ReadBeacn, WriteBeacn, read_value, write_value};
use byteorder::{ByteOrder, LittleEndian};
use enum_map::Enum;
use strum::{EnumIter, IntoEnumIterator};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Headphones {
    GetHeadphoneLevel,
    HeadphoneLevel(HPLevel),

    GetMicMonitor,
    MicMonitor(HPMicMonitorLevel),

    GetChannelsLinked,
    ChannelsLinked(bool),

    GetMicOutputGain,
    MicOutputGain(HPMicOutputGain),

    GetHeadphoneType,
    HeadphoneType(HeadphoneTypes),

    GetFXEnabled,
    FXEnabled(bool),
}

impl BeacnSubMessage for Headphones {
    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Headphones::HeadphoneLevel(_) | Headphones::GetHeadphoneLevel => [0x04, 0x00],
            Headphones::MicMonitor(_) | Headphones::GetMicMonitor => [0x06, 0x00],
            Headphones::ChannelsLinked(_) | Headphones::GetMicOutputGain => [0x07, 0x00],
            Headphones::MicOutputGain(_) | Headphones::GetChannelsLinked => [0x10, 0x00],
            Headphones::HeadphoneType(_) | Headphones::GetHeadphoneType => [0x11, 0x00],
            Headphones::FXEnabled(_) | Headphones::GetFXEnabled => [0x12, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Headphones::HeadphoneLevel(v) => write_value(v),
            Headphones::MicMonitor(v) => write_value(v),
            Headphones::ChannelsLinked(v) => v.write_beacn(),
            Headphones::MicOutputGain(v) => write_value(v),
            Headphones::HeadphoneType(v) => v.write_beacn(),
            Headphones::FXEnabled(v) => v.write_beacn(),
            _ => panic!("Attempted to get Value on Setter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue) -> Self {
        match key[0] {
            0x04 => Self::HeadphoneLevel(read_value(&value)),
            0x06 => Self::MicMonitor(read_value(&value)),
            0x07 => Self::MicOutputGain(read_value(&value)),
            0x10 => Self::ChannelsLinked(bool::read_beacn(&value)),
            0x11 => Self::HeadphoneType(HeadphoneTypes::read_beacn(&value)),
            0x12 => Self::FXEnabled(bool::read_beacn(&value)),
            _ => panic!("Unexpected Key: {}", key[0]),
        }
    }

    fn generate_fetch_message() -> Vec<Message> {
        vec![
            Message::Headphones(Headphones::GetHeadphoneLevel),
            Message::Headphones(Headphones::GetMicMonitor),
            Message::Headphones(Headphones::GetMicOutputGain),
            Message::Headphones(Headphones::GetChannelsLinked),
            Message::Headphones(Headphones::GetHeadphoneType),
            Message::Headphones(Headphones::GetFXEnabled),
        ]
    }
}

generate_range!(HPLevel, f32, -70.0..=-0.0);
generate_range!(HPMicMonitorLevel, f32, -100.0..=0.0);
generate_range!(HPMicOutputGain, f32, 0.0..=12.0);

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum HeadphoneTypes {
    #[default]
    LineLevel = 0x00,
    NormalPower = 0x01,
    HighImpedance = 0x02,
    InEarMonitors = 0x03,
}

impl Sealed for HeadphoneTypes {}
impl WriteBeacn for HeadphoneTypes {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

impl ReadBeacn for HeadphoneTypes {
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
