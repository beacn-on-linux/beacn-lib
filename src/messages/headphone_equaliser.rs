use crate::generate_range;
use crate::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::types::{BeacnValue, PackedEnumKey, ReadBeacn, WriteBeacn, read_value, write_value};
use enum_map::Enum;
use strum::{EnumIter, IntoEnumIterator};
use crate::manager::DeviceType;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HeadphoneEQ {
    GetAmount(HPEQType),
    Amount(HPEQType, HPEQValue),

    GetEnabled(HPEQType),
    Enabled(HPEQType, bool),
}

impl BeacnSubMessage for HeadphoneEQ {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }


    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            HeadphoneEQ::GetAmount(t) | HeadphoneEQ::Amount(t, _) => {
                [PackedEnumKey(*t, HPEQKeys::Amount).to_encoded(), 0]
            }
            HeadphoneEQ::GetEnabled(t) | HeadphoneEQ::Enabled(t, _) => {
                [PackedEnumKey(*t, HPEQKeys::Enabled).to_encoded(), 0]
            }
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            HeadphoneEQ::Amount(_, v) => write_value(v),
            HeadphoneEQ::Enabled(_, v) => v.write_beacn(),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType) -> Self {
        let key = PackedEnumKey::from_encoded(key[0]).unwrap();
        let eq_type = key.get_upper();
        match key.get_lower() {
            HPEQKeys::Enabled => HeadphoneEQ::Enabled(eq_type, bool::read_beacn(&value)),
            HPEQKeys::Amount => HeadphoneEQ::Amount(eq_type, read_value(&value)),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType) -> Vec<Message> {
        let mut messages = vec![];
        for eq_type in HPEQType::iter() {
            messages.push(Message::HeadphoneEQ(HeadphoneEQ::GetEnabled(eq_type)));
            messages.push(Message::HeadphoneEQ(HeadphoneEQ::GetAmount(eq_type)));
        }
        messages
    }
}

generate_range!(HPEQValue, f32, -12.0..=12.0);

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum HPEQType {
    Bass = 0x00,
    Mids = 0x01,
    Treble = 0x02,
}
impl From<HPEQType> for u8 {
    fn from(value: HPEQType) -> Self {
        value as u8
    }
}

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum HPEQKeys {
    Amount = 0x02,  // f32 (-12..12)
    Enabled = 0x05, // bool
}
impl From<HPEQKeys> for u8 {
    fn from(value: HPEQKeys) -> Self {
        value as u8
    }
}
