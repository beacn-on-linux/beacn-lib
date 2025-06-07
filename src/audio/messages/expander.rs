use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::generate_range;
use crate::manager::DeviceType;
use crate::types::sealed::Sealed;
use crate::types::{
    BeacnValue, PackedEnumKey, ReadBeacn, TimeFrame, WriteBeacn, read_value, write_value,
};
use byteorder::{ByteOrder, LittleEndian};
use enum_map::Enum;
use std::iter::Iterator;
use strum::{EnumIter, IntoEnumIterator};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Expander {
    GetMode,
    Mode(ExpanderMode),

    GetThreshold(ExpanderMode),
    Threshold(ExpanderMode, ExpanderThreshold),

    GetRatio(ExpanderMode),
    Ratio(ExpanderMode, ExpanderRatio),

    GetEnabled(ExpanderMode),
    Enabled(ExpanderMode, bool),

    GetAttack(ExpanderMode),
    Attack(ExpanderMode, TimeFrame),

    GetRelease(ExpanderMode),
    Release(ExpanderMode, TimeFrame),
}

impl BeacnSubMessage for Expander {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn is_device_message_set(&self) -> bool {
        matches!(
            self,
            Expander::Mode(_)
                | Expander::Threshold(_, _)
                | Expander::Ratio(_, _)
                | Expander::Enabled(_, _)
                | Expander::Attack(_, _)
                | Expander::Release(_, _)
        )
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Expander::GetMode | Expander::Mode(_) => [0x00, 0x00],
            Expander::GetThreshold(m) | Expander::Threshold(m, _) => {
                [PackedEnumKey(*m, ExpanderKeys::Threshold).to_encoded(), 0]
            }
            Expander::GetRatio(m) | Expander::Ratio(m, _) => {
                [PackedEnumKey(*m, ExpanderKeys::Ratio).to_encoded(), 0]
            }
            Expander::GetEnabled(m) | Expander::Enabled(m, _) => {
                [PackedEnumKey(*m, ExpanderKeys::Enabled).to_encoded(), 0]
            }
            Expander::GetAttack(m) | Expander::Attack(m, _) => {
                [PackedEnumKey(*m, ExpanderKeys::Attack).to_encoded(), 0]
            }
            Expander::GetRelease(m) | Expander::Release(m, _) => {
                [PackedEnumKey(*m, ExpanderKeys::Release).to_encoded(), 0]
            }
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Expander::Mode(v) => v.write_beacn(),
            Expander::Threshold(_, v) => write_value(v),
            Expander::Ratio(_, v) => write_value(v),
            Expander::Enabled(_, v) => v.write_beacn(),
            Expander::Attack(_, v) => write_value(v),
            Expander::Release(_, v) => write_value(v),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType) -> Self {
        if key == [0x00, 0x00] {
            return Self::Mode(ExpanderMode::read_beacn(&value));
        }

        // For any other value, we need to unpack the key.
        let key = PackedEnumKey::from_encoded(key[0]).unwrap();
        let mode = key.get_upper();
        match key.get_lower() {
            ExpanderKeys::Threshold => Expander::Threshold(mode, read_value(&value)),
            ExpanderKeys::Ratio => Expander::Ratio(mode, read_value(&value)),
            ExpanderKeys::Enabled => Expander::Enabled(mode, bool::read_beacn(&value)),
            ExpanderKeys::Attack => Expander::Attack(mode, read_value(&value)),
            ExpanderKeys::Release => Expander::Release(mode, read_value(&value)),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType) -> Vec<Message> {
        let mut messages = vec![];
        messages.push(Message::Expander(Expander::GetMode));

        for mode in ExpanderMode::iter() {
            messages.push(Message::Expander(Expander::GetThreshold(mode)));
            messages.push(Message::Expander(Expander::GetRatio(mode)));
            messages.push(Message::Expander(Expander::GetEnabled(mode)));
            messages.push(Message::Expander(Expander::GetAttack(mode)));
            messages.push(Message::Expander(Expander::GetRelease(mode)));
        }

        messages
    }
}

generate_range!(ExpanderRatio, f32, 1.0..=10.0);
generate_range!(ExpanderThreshold, f32, -90.0..=0.0);

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum ExpanderMode {
    #[default]
    Simple = 0x00,
    Advanced = 0x01,
}
impl From<ExpanderMode> for u8 {
    fn from(value: ExpanderMode) -> Self {
        value as u8
    }
}

impl Sealed for ExpanderMode {}
impl WriteBeacn for ExpanderMode {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}
impl ReadBeacn for ExpanderMode {
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
pub enum ExpanderKeys {
    Threshold = 0x03, // f32 (-90..=0)
    Ratio = 0x04,     // f32 (1..=10)
    Enabled = 0x05,   // bool
    Attack = 0x01,    // f32 (1..=2000)
    Release = 0x02,   // f32 (1..=2000)
}
impl From<ExpanderKeys> for u8 {
    fn from(value: ExpanderKeys) -> Self {
        value as u8
    }
}

// static EXPANDER_SIMPLE_PRESET: Lazy<HashMap<ExpanderKeys, f32>> = Lazy::new(|| [
//     (ExpanderKeys::Attack, 10.0),
//     (ExpanderKeys::Release, 180.0)
// ].into_iter().collect());
