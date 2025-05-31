use crate::generate_range;
use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::types::sealed::Sealed;
use crate::types::{
    BeacnValue, MakeUpGain, PackedEnumKey, ReadBeacn, TimeFrame, WriteBeacn, read_value,
    write_value,
};
use byteorder::{ByteOrder, LittleEndian};
use enum_map::Enum;
use strum::{EnumIter, IntoEnumIterator};
use crate::manager::DeviceType;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Compressor {
    GetMode,
    Mode(CompressorMode),

    GetAttack(CompressorMode),
    Attack(CompressorMode, TimeFrame),

    GetRelease(CompressorMode),
    Release(CompressorMode, TimeFrame),

    GetThreshold(CompressorMode),
    Threshold(CompressorMode, CompressorThreshold),

    GetRatio(CompressorMode),
    Ratio(CompressorMode, CompressorRatio),

    GetMakeupGain(CompressorMode),
    MakeupGain(CompressorMode, MakeUpGain),

    GetEnabled(CompressorMode),
    Enabled(CompressorMode, bool),
}

impl BeacnSubMessage for Compressor {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }


    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Compressor::Mode(_) | Compressor::GetMode => [0, 0],
            Compressor::Attack(m, _) | Compressor::GetAttack(m) => {
                [PackedEnumKey(*m, CompressorKeys::Attack).to_encoded(), 0]
            }
            Compressor::Release(m, _) | Compressor::GetRelease(m) => {
                [PackedEnumKey(*m, CompressorKeys::Release).to_encoded(), 0]
            }
            Compressor::Threshold(m, _) | Compressor::GetThreshold(m) => {
                [PackedEnumKey(*m, CompressorKeys::Threshold).to_encoded(), 0]
            }
            Compressor::Ratio(m, _) | Compressor::GetRatio(m) => {
                [PackedEnumKey(*m, CompressorKeys::Ratio).to_encoded(), 0]
            }
            Compressor::MakeupGain(m, _) | Compressor::GetMakeupGain(m) => [
                PackedEnumKey(*m, CompressorKeys::MakeupGain).to_encoded(),
                0,
            ],
            Compressor::Enabled(m, _) | Compressor::GetEnabled(m) => {
                [PackedEnumKey(*m, CompressorKeys::Enabled).to_encoded(), 0]
            }
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Compressor::Mode(v) => v.write_beacn(),
            Compressor::Attack(_, v) => write_value(v),
            Compressor::Release(_, v) => write_value(v),
            Compressor::Threshold(_, v) => write_value(v),
            Compressor::Ratio(_, v) => write_value(v),
            Compressor::MakeupGain(_, v) => write_value(v),
            Compressor::Enabled(_, v) => v.write_beacn(),
            _ => panic!("Attempted to Set on a Get"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType) -> Self {
        if key == [0, 0] {
            return Self::Mode(CompressorMode::read_beacn(&value));
        }

        // For any other value, we need to unpack the key.
        let key = PackedEnumKey::from_encoded(key[0]).unwrap();
        let mode = key.get_upper();
        match key.get_lower() {
            CompressorKeys::Attack => Self::Attack(mode, read_value(&value)),
            CompressorKeys::Release => Self::Release(mode, read_value(&value)),
            CompressorKeys::Threshold => Self::Threshold(mode, read_value(&value)),
            CompressorKeys::Ratio => Self::Ratio(mode, read_value(&value)),
            CompressorKeys::MakeupGain => Self::MakeupGain(mode, read_value(&value)),
            CompressorKeys::Enabled => Self::Enabled(mode, bool::read_beacn(&value)),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType) -> Vec<Message> {
        let mut messages = vec![];
        messages.push(Message::Compressor(Compressor::GetMode));
        for mode in CompressorMode::iter() {
            messages.push(Message::Compressor(Compressor::GetAttack(mode)));
            messages.push(Message::Compressor(Compressor::GetRelease(mode)));
            messages.push(Message::Compressor(Compressor::GetThreshold(mode)));
            messages.push(Message::Compressor(Compressor::GetRatio(mode)));
            messages.push(Message::Compressor(Compressor::GetMakeupGain(mode)));
            messages.push(Message::Compressor(Compressor::GetEnabled(mode)));
        }
        messages
    }
}

generate_range!(CompressorThreshold, f32, -50.0..=0.0);
generate_range!(CompressorRatio, f32, 1.0..=16.0);

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum CompressorMode {
    #[default]
    Simple = 0x00,
    Advanced = 0x01,
}
impl From<CompressorMode> for u8 {
    fn from(value: CompressorMode) -> Self {
        value as u8
    }
}

impl Sealed for CompressorMode {}
impl WriteBeacn for CompressorMode {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}
impl ReadBeacn for CompressorMode {
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

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
enum CompressorKeys {
    Attack = 0x01,     // f32 (0..=2000)
    Release = 0x02,    // f32 (0..=2000)
    Threshold = 0x03,  // f32 (-50..0)
    Ratio = 0x06,      // f32, SIMPLE ONLY (amount == 0) ? 0 : 1 + (percent * 0.9)
    MakeupGain = 0x05, // f32 (0..=12)
    Enabled = 0x07,    // bool
}
impl From<CompressorKeys> for u8 {
    fn from(value: CompressorKeys) -> Self {
        value as u8
    }
}
