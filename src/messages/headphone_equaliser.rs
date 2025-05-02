use crate::generate_range;
use crate::messages::{Message, BeacnSubMessage};
use crate::types::{read_value, write_value, BeacnValue, PackedEnumKey, ReadBeacn, WriteBeacn};
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug)]
pub enum HeadphoneEQ {
    GetValue(HPEQType),
    Value(HPEQType, HPEQValue),

    GetEnabled(HPEQType),
    Enabled(HPEQType, bool),
}

impl BeacnSubMessage for HeadphoneEQ {
    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            HeadphoneEQ::GetValue(t) | HeadphoneEQ::Value(t, _) => {
                [PackedEnumKey(*t, HPEQKeys::Value).to_encoded(), 0]
            }
            HeadphoneEQ::GetEnabled(t) | HeadphoneEQ::Enabled(t, _) => {
                [PackedEnumKey(*t, HPEQKeys::Enabled).to_encoded(), 0]
            }
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            HeadphoneEQ::Value(_, v) => write_value(v),
            HeadphoneEQ::Enabled(_, v) => v.write_beacn(),
            _ => panic!("Attempted to Set a Getter")
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue) -> Self {
        let key = PackedEnumKey::from_encoded(key[0]).unwrap();
        let eq_type = key.get_upper();
        match key.get_lower() {
            HPEQKeys::Enabled => HeadphoneEQ::Enabled(eq_type, bool::read_beacn(&value)),
            HPEQKeys::Value => HeadphoneEQ::Value(eq_type, read_value(&value)),
        }
    }

    fn generate_fetch_message() -> Vec<Message> {
        let mut messages = vec![];
        for eq_type in HPEQType::iter() {
            messages.push(Message::HeadphoneEQ(HeadphoneEQ::GetEnabled(eq_type)));
            messages.push(Message::HeadphoneEQ(HeadphoneEQ::GetValue(eq_type)));
        }
        messages
    }
}

generate_range!(HPEQValue, f32, -12.0..=12.0);

#[derive(Copy, Clone, EnumIter, Debug)]
pub enum HPEQType {
    Bass = 0x00,
    Mids = 0x01,
    Treble = 0x02,
}
impl Into<u8> for HPEQType {
    fn into(self) -> u8 {
        self as u8
    }
}

#[derive(Copy, Clone, EnumIter)]
pub enum HPEQKeys {
    Value = 0x02,   // f32 (-12..12)
    Enabled = 0x05, // bool
}
impl Into<u8> for HPEQKeys {
    fn into(self) -> u8 {
        self as u8
    }
}
