use crate::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::types::{BeacnValue, PackedEnumKey, ReadBeacn, WriteBeacn, read_value, write_value};

use crate::generate_range;
use crate::types::sealed::Sealed;
use byteorder::{ByteOrder, LittleEndian};
use enum_map::Enum;
use strum::{EnumIter, IntoEnumIterator};
use crate::manager::DeviceType;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Equaliser {
    GetMode,
    Mode(EQMode),

    GetType(EQMode, EQBand),
    Type(EQMode, EQBand, EQBandType),

    GetGain(EQMode, EQBand),
    Gain(EQMode, EQBand, EQGain),

    GetFrequency(EQMode, EQBand),
    Frequency(EQMode, EQBand, EQFrequency),

    GetQ(EQMode, EQBand),
    Q(EQMode, EQBand, EQQ),

    GetEnabled(EQMode, EQBand),
    Enabled(EQMode, EQBand, bool),
}

impl BeacnSubMessage for Equaliser {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Equaliser::Mode(_) | Equaliser::GetMode => [0x00, 0x00],
            Equaliser::Type(m, b, _) | Equaliser::GetType(m, b) => [
                PackedEnumKey(*b, EqualiserKeys::Type).to_encoded(),
                *m as u8,
            ],
            Equaliser::Gain(m, b, _) | Equaliser::GetGain(m, b) => [
                PackedEnumKey(*b, EqualiserKeys::Gain).to_encoded(),
                *m as u8,
            ],
            Equaliser::Frequency(m, b, _) | Equaliser::GetFrequency(m, b) => [
                PackedEnumKey(*b, EqualiserKeys::Frequency).to_encoded(),
                *m as u8,
            ],
            Equaliser::Q(m, b, _) | Equaliser::GetQ(m, b) => {
                [PackedEnumKey(*b, EqualiserKeys::Q).to_encoded(), *m as u8]
            }
            Equaliser::Enabled(m, b, _) | Equaliser::GetEnabled(m, b) => [
                PackedEnumKey(*b, EqualiserKeys::Enabled).to_encoded(),
                *m as u8,
            ],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Equaliser::Mode(v) => v.write_beacn(),
            Equaliser::Type(_, _, v) => v.write_beacn(),
            Equaliser::Gain(_, _, v) => write_value(v),
            Equaliser::Frequency(_, _, v) => write_value(v),
            Equaliser::Q(_, _, v) => write_value(v),
            Equaliser::Enabled(_, _, v) => v.write_beacn(),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue) -> Self {
        // This one's kinda interesting, we need to first check for 00,00..
        if key == [0x00, 0x00] {
            return Self::Mode(EQMode::read_beacn(&value));
        }

        let mode = EQMode::from(key[1]);
        let key = PackedEnumKey::from_encoded(key[0]).unwrap();
        let band = key.get_upper();
        match key.get_lower() {
            EqualiserKeys::Q => Self::Q(mode, band, read_value(&value)),
            EqualiserKeys::Type => Self::Type(mode, band, EQBandType::read_beacn(&value)),
            EqualiserKeys::Gain => Self::Gain(mode, band, read_value(&value)),
            EqualiserKeys::Frequency => Self::Frequency(mode, band, read_value(&value)),
            EqualiserKeys::Enabled => Self::Enabled(mode, band, bool::read_beacn(&value)),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType) -> Vec<Message> {
        // This one's kinda obnoxious, because we need to handle it both for the modes, and
        // the bands, so lets get started.
        let mut messages = vec![];
        messages.push(Message::Equaliser(Equaliser::GetMode));
        for mode in EQMode::iter() {
            for band in EQBand::iter() {
                messages.push(Message::Equaliser(Equaliser::GetType(mode, band)));
                messages.push(Message::Equaliser(Equaliser::GetGain(mode, band)));
                messages.push(Message::Equaliser(Equaliser::GetFrequency(mode, band)));
                messages.push(Message::Equaliser(Equaliser::GetQ(mode, band)));
                messages.push(Message::Equaliser(Equaliser::GetEnabled(mode, band)));
            }
        }

        messages
    }
}

generate_range!(EQGain, f32, -12.0..=12.0);
generate_range!(EQFrequency, f32, 20.0..=2000.0);
generate_range!(EQQ, f32, -0.1..=10.0);

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum EQMode {
    #[default]
    Simple = 0x00,
    Advanced = 0x01,
}

impl Sealed for EQMode {}
impl WriteBeacn for EQMode {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

impl ReadBeacn for EQMode {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for var in Self::iter() {
            if var as u32 == value {
                return var;
            }
        }
        panic!("Unable to Locate Value")
    }
}

impl From<u8> for EQMode {
    fn from(value: u8) -> Self {
        for var in Self::iter() {
            if var as u8 == value {
                return var;
            }
        }
        panic!("Unable to Locate Value")
    }
}

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum EQBand {
    Band1 = 0x00,
    Band2 = 0x01,
    Band3 = 0x02,
    Band4 = 0x03,
    Band5 = 0x04,
    Band6 = 0x05,
    Band7 = 0x06,
    Band8 = 0x08,
}
impl From<EQBand> for u8 {
    fn from(value: EQBand) -> Self {
        value as u8
    }
}

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
enum EqualiserKeys {
    Type = 0x01,      // BandType
    Gain = 0x02,      // f32 (-12..=12)
    Frequency = 0x03, // f32 (20..=20000)
    Q = 0x04,         // f32 (0.1..=10)
    Enabled = 0x05,   // boolean
}
impl From<EqualiserKeys> for u8 {
    fn from(value: EqualiserKeys) -> Self {
        value as u8
    }
}

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum EQBandType {
    #[default]
    NotSet = 0x00,
    LowPassFilter = 0x01,
    HighPassFilter = 0x02,
    NotchFilter = 0x03,
    BellBand = 0x04,
    LowShelf = 0x05,
    HighShelf = 0x06,
}

impl Sealed for EQBandType {}
impl WriteBeacn for EQBandType {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}
impl ReadBeacn for EQBandType {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for var in Self::iter() {
            if var as u32 == value {
                return var;
            }
        }
        panic!("Unable to Locate Value {:?}", value)
    }
}
