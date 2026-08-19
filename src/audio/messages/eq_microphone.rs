use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::types::{BeacnValue, PackedEnumKey, ReadBeacn, WriteBeacn, read_value, write_value};

use crate::audio::messages::eq_common::{
    EQBand, EQBandType, EQFrequency, EQGain, EQMode, EQQ, EqualiserKeys,
};
use crate::manager::DeviceType;
use crate::message_group;
use strum::IntoEnumIterator;

message_group!(
    pub enum EQMicrophone {
        Mode() -> EQMode,
        Type(EQMode, EQBand) -> EQBandType,
        Gain(EQMode, EQBand) -> EQGain,
        Frequency(EQMode, EQBand) -> EQFrequency,
        Q(EQMode, EQBand) -> EQQ,
        Enabled(EQMode, EQBand) -> bool,
    }
);

impl BeacnSubMessage for EQMicrophone {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn is_device_message_set(&self) -> bool {
        self.is_message_set()
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            EQMicrophone::Mode(_) | EQMicrophone::GetMode => [0x00, 0x00],
            EQMicrophone::Type(m, b, _) | EQMicrophone::GetType(m, b) => [
                PackedEnumKey(*b, EqualiserKeys::Type).to_encoded(),
                *m as u8,
            ],
            EQMicrophone::Gain(m, b, _) | EQMicrophone::GetGain(m, b) => [
                PackedEnumKey(*b, EqualiserKeys::Gain).to_encoded(),
                *m as u8,
            ],
            EQMicrophone::Frequency(m, b, _) | EQMicrophone::GetFrequency(m, b) => [
                PackedEnumKey(*b, EqualiserKeys::Frequency).to_encoded(),
                *m as u8,
            ],
            EQMicrophone::Q(m, b, _) | EQMicrophone::GetQ(m, b) => {
                [PackedEnumKey(*b, EqualiserKeys::Q).to_encoded(), *m as u8]
            }
            EQMicrophone::Enabled(m, b, _) | EQMicrophone::GetEnabled(m, b) => [
                PackedEnumKey(*b, EqualiserKeys::Enabled).to_encoded(),
                *m as u8,
            ],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            EQMicrophone::Mode(v) => v.write_beacn(),
            EQMicrophone::Type(_, _, v) => v.write_beacn(),
            EQMicrophone::Gain(_, _, v) => write_value(v),
            EQMicrophone::Frequency(_, _, v) => write_value(v),
            EQMicrophone::Q(_, _, v) => write_value(v),
            EQMicrophone::Enabled(_, _, v) => v.write_beacn(),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType) -> Self {
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
        messages.push(Message::Equaliser(EQMicrophone::GetMode));
        for mode in EQMode::iter() {
            for band in EQBand::iter() {
                messages.push(Message::Equaliser(EQMicrophone::GetType(mode, band)));
                messages.push(Message::Equaliser(EQMicrophone::GetGain(mode, band)));
                messages.push(Message::Equaliser(EQMicrophone::GetFrequency(mode, band)));
                messages.push(Message::Equaliser(EQMicrophone::GetQ(mode, band)));
                messages.push(Message::Equaliser(EQMicrophone::GetEnabled(mode, band)));
            }
        }

        messages
    }
}
