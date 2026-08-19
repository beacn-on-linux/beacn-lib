use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::manager::DeviceType;
use crate::types::{BeacnValue, PackedEnumKey, ReadBeacn, WriteBeacn, read_value, write_value};
use crate::version::VersionNumber;
use crate::{EQ_HEADPHONES_VERSION, generate_range, message_group};
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

message_group!(
    pub enum EQHPLegacy {
        Amount(HPEQType) -> HPEQValue,
        Enabled(HPEQType) -> bool,
    }
);

impl BeacnSubMessage for EQHPLegacy {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }
    fn get_message_maximum_version(&self) -> VersionNumber {
        // None of these can execute on a 1.3+ Firmware, should use EQHeadphones
        EQ_HEADPHONES_VERSION
    }

    fn is_device_message_set(&self) -> bool {
        self.is_message_set()
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            EQHPLegacy::GetAmount(t) | EQHPLegacy::Amount(t, _) => {
                [PackedEnumKey(*t, HPEQKeys::Amount).to_encoded(), 0]
            }
            EQHPLegacy::GetEnabled(t) | EQHPLegacy::Enabled(t, _) => {
                [PackedEnumKey(*t, HPEQKeys::Enabled).to_encoded(), 0]
            }
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            EQHPLegacy::Amount(_, v) => write_value(v),
            EQHPLegacy::Enabled(_, v) => v.write_beacn(),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType) -> Self {
        let key = PackedEnumKey::from_encoded(key[0]).unwrap();
        let eq_type = key.get_upper();
        match key.get_lower() {
            HPEQKeys::Enabled => EQHPLegacy::Enabled(eq_type, bool::read_beacn(&value)),
            HPEQKeys::Amount => EQHPLegacy::Amount(eq_type, read_value(&value)),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType, v: VersionNumber) -> Vec<Message> {
        if v > EQ_HEADPHONES_VERSION {
            return vec![];
        }

        let mut messages = vec![];
        for eq_type in HPEQType::iter() {
            messages.push(Message::EQHPLegacy(EQHPLegacy::GetEnabled(eq_type)));
            messages.push(Message::EQHPLegacy(EQHPLegacy::GetAmount(eq_type)));
        }
        messages
    }
}

generate_range!(HPEQValue, f32, -12.0..=12.0);

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HPEQKeys {
    Amount = 0x02,  // f32 (-12..12)
    Enabled = 0x05, // bool
}
impl From<HPEQKeys> for u8 {
    fn from(value: HPEQKeys) -> Self {
        value as u8
    }
}
