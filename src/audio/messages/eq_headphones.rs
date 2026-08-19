use crate::audio::messages::eq_common::{EQBand, EQBandType, EQFrequency, EQGain, EQQ, EQSubType};
use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::manager::DeviceType;
use crate::{message_group, EQ_HEADPHONES_VERSION};
use crate::types::{BeacnValue, PackedEnumKey, ReadBeacn, WriteBeacn, read_value, write_value};
use crate::version::VersionNumber;
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

message_group!(
    pub enum EQHeadphones {
        Linked() -> bool,
        Type(EQChannel, EQBand) -> EQBandType,
        Gain(EQChannel, EQBand) -> EQGain,
        Frequency(EQChannel, EQBand) -> EQFrequency,
        Q(EQChannel, EQBand) -> EQQ,
        Enabled(EQChannel, EQBand) -> bool,
    }
);

impl BeacnSubMessage for EQHeadphones {
    fn get_device_message_type(&self) -> DeviceMessageType {
        DeviceMessageType::Common
    }

    fn is_device_message_set(&self) -> bool {
        self.is_message_set()
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            EQHeadphones::Linked(_) | EQHeadphones::GetLinked => [0x01, 0x00],
            EQHeadphones::Type(m, b, _) | EQHeadphones::GetType(m, b) => [
                PackedEnumKey(*b, EQHeadphoneKeys::Type).to_encoded(),
                *m as u8,
            ],
            EQHeadphones::Gain(m, b, _) | EQHeadphones::GetGain(m, b) => [
                PackedEnumKey(*b, EQHeadphoneKeys::Gain).to_encoded(),
                *m as u8,
            ],
            EQHeadphones::Frequency(m, b, _) | EQHeadphones::GetFrequency(m, b) => [
                PackedEnumKey(*b, EQHeadphoneKeys::Frequency).to_encoded(),
                *m as u8,
            ],
            EQHeadphones::Q(m, b, _) | EQHeadphones::GetQ(m, b) => {
                [PackedEnumKey(*b, EQHeadphoneKeys::Q).to_encoded(), *m as u8]
            }
            EQHeadphones::Enabled(m, b, _) | EQHeadphones::GetEnabled(m, b) => [
                PackedEnumKey(*b, EQHeadphoneKeys::Enabled).to_encoded(),
                *m as u8,
            ],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            EQHeadphones::Linked(v) => v.write_beacn(),
            EQHeadphones::Type(_, _, v) => v.write_beacn(),
            EQHeadphones::Gain(_, _, v) => write_value(v),
            EQHeadphones::Frequency(_, _, v) => write_value(v),
            EQHeadphones::Q(_, _, v) => write_value(v),
            EQHeadphones::Enabled(_, _, v) => v.write_beacn(),
            _ => panic!("Attempted to Set a Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, _device_type: DeviceType) -> Self {
        // This one's kinda interesting, we need to first check for 01,00..
        if key == [0x01, 0x00] {
            return Self::Linked(bool::read_beacn(&value));
        }

        let channel = EQChannel::from(EQSubType::from(key[1]));
        let key = PackedEnumKey::from_encoded(key[0]).unwrap();
        let band = key.get_upper();
        match key.get_lower() {
            EQHeadphoneKeys::Q => Self::Q(channel, band, read_value(&value)),
            EQHeadphoneKeys::Type => Self::Type(channel, band, EQBandType::read_beacn(&value)),
            EQHeadphoneKeys::Gain => Self::Gain(channel, band, read_value(&value)),
            EQHeadphoneKeys::Frequency => Self::Frequency(channel, band, read_value(&value)),
            EQHeadphoneKeys::Enabled => Self::Enabled(channel, band, bool::read_beacn(&value)),
        }
    }

    fn generate_fetch_message(_device_type: DeviceType, v: VersionNumber) -> Vec<Message> {
        if v < EQ_HEADPHONES_VERSION {
            return vec![];
        }

        let mut messages = vec![];
        messages.push(Message::EQHeadphones(EQHeadphones::GetLinked));
        for mode in EQChannel::iter() {
            for band in EQBand::iter() {
                messages.push(Message::EQHeadphones(EQHeadphones::GetType(mode, band)));
                messages.push(Message::EQHeadphones(EQHeadphones::GetGain(mode, band)));
                messages.push(Message::EQHeadphones(EQHeadphones::GetFrequency(
                    mode, band,
                )));
                messages.push(Message::EQHeadphones(EQHeadphones::GetQ(mode, band)));
                messages.push(Message::EQHeadphones(EQHeadphones::GetEnabled(mode, band)));
            }
        }

        messages
    }
}

#[derive(
    Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq, Serialize, Deserialize,
)]
pub enum EQChannel {
    #[default]
    Left,
    Right,
}

impl From<EQSubType> for EQChannel {
    fn from(value: EQSubType) -> Self {
        match value {
            EQSubType::Zero => EQChannel::Left,
            EQSubType::One => EQChannel::Right,
        }
    }
}

impl From<EQChannel> for EQSubType {
    fn from(value: EQChannel) -> Self {
        match value {
            EQChannel::Left => EQSubType::Zero,
            EQChannel::Right => EQSubType::One,
        }
    }
}

// In what could only really be described as a hack, this is offset by 1 from our normal
// keys defined in the Mic. I suspect I know why this is the case, but ugh.
#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) enum EQHeadphoneKeys {
    Type = 0x02,
    Gain = 0x03,
    Frequency = 0x04,
    Q = 0x05,
    Enabled = 0x06,
}
impl From<EQHeadphoneKeys> for u8 {
    fn from(value: EQHeadphoneKeys) -> Self {
        value as u8
    }
}
