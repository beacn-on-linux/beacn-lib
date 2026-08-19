use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::types::{BeacnValue, PackedEnumKey, ReadBeacn, WriteBeacn, read_value, write_value};
use enum_map::Enum;
use serde::{Deserialize, Serialize};

use crate::audio::messages::eq_common::{EQBand, EQBandType, EQFrequency, EQGain, EQQ, EQSubType};
use crate::manager::DeviceType;
use crate::message_group;
use crate::version::VersionNumber;
use strum::{EnumIter, IntoEnumIterator};

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

    fn to_beacn_key(&self, _: VersionNumber) -> [u8; 2] {
        match self {
            EQMicrophone::Mode(_) | EQMicrophone::GetMode => [0x00, 0x00],
            EQMicrophone::Type(m, b, _) | EQMicrophone::GetType(m, b) => [
                PackedEnumKey(*b, EQMicrophoneKeys::Type).to_encoded(),
                *m as u8,
            ],
            EQMicrophone::Gain(m, b, _) | EQMicrophone::GetGain(m, b) => [
                PackedEnumKey(*b, EQMicrophoneKeys::Gain).to_encoded(),
                *m as u8,
            ],
            EQMicrophone::Frequency(m, b, _) | EQMicrophone::GetFrequency(m, b) => [
                PackedEnumKey(*b, EQMicrophoneKeys::Frequency).to_encoded(),
                *m as u8,
            ],
            EQMicrophone::Q(m, b, _) | EQMicrophone::GetQ(m, b) => [
                PackedEnumKey(*b, EQMicrophoneKeys::Q).to_encoded(),
                *m as u8,
            ],
            EQMicrophone::Enabled(m, b, _) | EQMicrophone::GetEnabled(m, b) => [
                PackedEnumKey(*b, EQMicrophoneKeys::Enabled).to_encoded(),
                *m as u8,
            ],
        }
    }

    fn to_beacn_value(&self, _: VersionNumber) -> BeacnValue {
        match self {
            EQMicrophone::Mode(v) => EQSubType::from(*v).write_beacn(),
            EQMicrophone::Type(_, _, v) => v.write_beacn(),
            EQMicrophone::Gain(_, _, v) => write_value(v),
            EQMicrophone::Frequency(_, _, v) => write_value(v),
            EQMicrophone::Q(_, _, v) => write_value(v),
            EQMicrophone::Enabled(_, _, v) => v.write_beacn(),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _: DeviceType, _: VersionNumber) -> Self {
        // This one's kinda interesting, we need to first check for 00,00..
        if key == [0x00, 0x00] {
            return Self::Mode(EQMode::from(EQSubType::read_beacn(&value)));
        }

        let mode = EQMode::from(EQSubType::from(key[1]));
        let key = PackedEnumKey::from_encoded(key[0]).unwrap();
        let band = key.get_upper();
        match key.get_lower() {
            EQMicrophoneKeys::Q => Self::Q(mode, band, read_value(&value)),
            EQMicrophoneKeys::Type => Self::Type(mode, band, EQBandType::read_beacn(&value)),
            EQMicrophoneKeys::Gain => Self::Gain(mode, band, read_value(&value)),
            EQMicrophoneKeys::Frequency => Self::Frequency(mode, band, read_value(&value)),
            EQMicrophoneKeys::Enabled => Self::Enabled(mode, band, bool::read_beacn(&value)),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType, _: VersionNumber) -> Vec<Message> {
        // This one's kinda obnoxious, because we need to handle it both for the modes, and
        // the bands, so lets get started.
        let mut messages = vec![];
        messages.push(Message::EQMicrophone(EQMicrophone::GetMode));
        for mode in EQMode::iter() {
            for band in EQBand::iter() {
                messages.push(Message::EQMicrophone(EQMicrophone::GetType(mode, band)));
                messages.push(Message::EQMicrophone(EQMicrophone::GetGain(mode, band)));
                messages.push(Message::EQMicrophone(EQMicrophone::GetFrequency(
                    mode, band,
                )));
                messages.push(Message::EQMicrophone(EQMicrophone::GetQ(mode, band)));
                messages.push(Message::EQMicrophone(EQMicrophone::GetEnabled(mode, band)));
            }
        }

        messages
    }
}

// This will transparent map back to EQSubType, so we can keep our naming, but share conversions
#[derive(
    Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq, Serialize, Deserialize,
)]
pub enum EQMode {
    #[default]
    Simple,
    Advanced,
}

impl From<EQSubType> for EQMode {
    fn from(value: EQSubType) -> Self {
        match value {
            EQSubType::Zero => EQMode::Simple,
            EQSubType::One => EQMode::Advanced,
        }
    }
}

impl From<EQMode> for EQSubType {
    fn from(value: EQMode) -> Self {
        match value {
            EQMode::Simple => EQSubType::Zero,
            EQMode::Advanced => EQSubType::One,
        }
    }
}

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) enum EQMicrophoneKeys {
    Type = 0x01,      // BandType
    Gain = 0x02,      // f32 (-12..=12)
    Frequency = 0x03, // f32 (20..=20000)
    Q = 0x04,         // f32 (0.1..=10)
    Enabled = 0x05,   // boolean
}
impl From<EQMicrophoneKeys> for u8 {
    fn from(value: EQMicrophoneKeys) -> Self {
        value as u8
    }
}
