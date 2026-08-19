use crate::EQ_HEADPHONES_VERSION;
use crate::audio::messages::bass_enhancement::BassEnhancement;
use crate::audio::messages::compressor::Compressor;
use crate::audio::messages::controls::Controls;
use crate::audio::messages::deesser::DeEsser;
use crate::audio::messages::eq_headphones::EQHeadphones;
use crate::audio::messages::eq_headphones_legacy::EQHPLegacy;
use crate::audio::messages::eq_microphone::EQMicrophone;
use crate::audio::messages::exciter::Exciter;
use crate::audio::messages::expander::Expander;
use crate::audio::messages::headphones::Headphones;
use crate::audio::messages::lighting::Lighting;
use crate::audio::messages::mic_setup::MicSetup;
use crate::audio::messages::subwoofer::Subwoofer;
use crate::audio::messages::suppressor::Suppressor;
use crate::manager::DeviceType;
use crate::types::BeacnValue;
use crate::version::VersionNumber;
use serde::{Deserialize, Serialize};

mod _macros;

pub mod bass_enhancement;
pub mod compressor;
pub mod controls;
pub mod deesser;
pub mod eq_common;
pub mod eq_headphones;
pub mod eq_headphones_legacy;
pub mod eq_microphone;
pub mod exciter;
pub mod expander;
pub mod headphones;
pub mod lighting;
pub mod mic_setup;
pub mod subwoofer;
pub mod suppressor;

const VERSION_MIN_ALL: VersionNumber = VersionNumber(0, 0, 0, 0);
const VERSION_MAX_ALL: VersionNumber = VersionNumber(u32::MAX, u32::MAX, u32::MAX, u32::MAX);

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Message {
    BassEnhancement(BassEnhancement),
    Compressor(Compressor),
    DeEsser(DeEsser),
    EQMicrophone(EQMicrophone),
    EQHeadphones(EQHeadphones),
    Exciter(Exciter),
    Expander(Expander),
    EQHPLegacy(EQHPLegacy),
    Headphones(Headphones),
    Lighting(Lighting),
    MicSetup(MicSetup),
    Subwoofer(Subwoofer),
    Suppressor(Suppressor),
    Controls(Controls),
}

impl Message {
    pub(crate) fn is_device_message_set(&self) -> bool {
        match self {
            Message::BassEnhancement(v) => v.is_device_message_set(),
            Message::Compressor(v) => v.is_device_message_set(),
            Message::DeEsser(v) => v.is_device_message_set(),
            Message::EQMicrophone(v) => v.is_device_message_set(),
            Message::EQHeadphones(v) => v.is_device_message_set(),
            Message::Exciter(v) => v.is_device_message_set(),
            Message::Expander(v) => v.is_device_message_set(),
            Message::EQHPLegacy(v) => v.is_device_message_set(),
            Message::Headphones(v) => v.is_device_message_set(),
            Message::Lighting(v) => v.is_device_message_set(),
            Message::MicSetup(v) => v.is_device_message_set(),
            Message::Subwoofer(v) => v.is_device_message_set(),
            Message::Suppressor(v) => v.is_device_message_set(),
            Message::Controls(v) => v.is_device_message_set(),
        }
    }

    pub(crate) fn get_device_message_type(&self) -> DeviceMessageType {
        match self {
            Message::BassEnhancement(v) => v.get_device_message_type(),
            Message::Compressor(v) => v.get_device_message_type(),
            Message::DeEsser(v) => v.get_device_message_type(),
            Message::EQMicrophone(v) => v.get_device_message_type(),
            Message::EQHeadphones(v) => v.get_device_message_type(),
            Message::Exciter(v) => v.get_device_message_type(),
            Message::Expander(v) => v.get_device_message_type(),
            Message::EQHPLegacy(v) => v.get_device_message_type(),
            Message::Headphones(v) => v.get_device_message_type(),
            Message::Lighting(v) => v.get_device_message_type(),
            Message::MicSetup(v) => v.get_device_message_type(),
            Message::Subwoofer(v) => v.get_device_message_type(),
            Message::Suppressor(v) => v.get_device_message_type(),
            Message::Controls(v) => v.get_device_message_type(),
        }
    }

    pub fn get_message_minimum_version(&self) -> VersionNumber {
        match self {
            Message::BassEnhancement(v) => v.get_message_minimum_version(),
            Message::Compressor(v) => v.get_message_minimum_version(),
            Message::DeEsser(v) => v.get_message_minimum_version(),
            Message::EQMicrophone(v) => v.get_message_minimum_version(),
            Message::EQHeadphones(v) => v.get_message_minimum_version(),
            Message::Exciter(v) => v.get_message_minimum_version(),
            Message::Expander(v) => v.get_message_minimum_version(),
            Message::EQHPLegacy(v) => v.get_message_minimum_version(),
            Message::Headphones(v) => v.get_message_minimum_version(),
            Message::Lighting(v) => v.get_message_minimum_version(),
            Message::MicSetup(v) => v.get_message_minimum_version(),
            Message::Subwoofer(v) => v.get_message_minimum_version(),
            Message::Suppressor(v) => v.get_message_minimum_version(),
            Message::Controls(v) => v.get_message_minimum_version(),
        }
    }

    pub fn get_message_maximum_version(&self) -> VersionNumber {
        match self {
            Message::BassEnhancement(v) => v.get_message_maximum_version(),
            Message::Compressor(v) => v.get_message_maximum_version(),
            Message::DeEsser(v) => v.get_message_maximum_version(),
            Message::EQMicrophone(v) => v.get_message_maximum_version(),
            Message::EQHeadphones(v) => v.get_message_maximum_version(),
            Message::Exciter(v) => v.get_message_maximum_version(),
            Message::Expander(v) => v.get_message_maximum_version(),
            Message::EQHPLegacy(v) => v.get_message_maximum_version(),
            Message::Headphones(v) => v.get_message_maximum_version(),
            Message::Lighting(v) => v.get_message_maximum_version(),
            Message::MicSetup(v) => v.get_message_maximum_version(),
            Message::Subwoofer(v) => v.get_message_maximum_version(),
            Message::Suppressor(v) => v.get_message_maximum_version(),
            Message::Controls(v) => v.get_message_maximum_version(),
        }
    }

    pub fn to_beacn_key(&self, vn: VersionNumber) -> [u8; 3] {
        let (top, sub) = match self {
            Message::BassEnhancement(v) => (BeacnMessage::BassEnhance as u8, v.to_beacn_key(vn)),
            Message::Compressor(v) => (BeacnMessage::Compressor as u8, v.to_beacn_key(vn)),
            Message::DeEsser(v) => (BeacnMessage::DeEsser as u8, v.to_beacn_key(vn)),
            Message::EQMicrophone(v) => (BeacnMessage::EQMicrophone as u8, v.to_beacn_key(vn)),
            Message::EQHeadphones(v) => (BeacnMessage::EQHeadphones as u8, v.to_beacn_key(vn)),

            // This is the legacy (pre 1.3) headphone EQ
            Message::EQHPLegacy(v) => (BeacnMessage::EQHeadphones as u8, v.to_beacn_key(vn)),
            Message::Exciter(v) => (BeacnMessage::Exciter as u8, v.to_beacn_key(vn)),
            Message::Expander(v) => (BeacnMessage::Expander as u8, v.to_beacn_key(vn)),
            Message::Headphones(v) => (BeacnMessage::Headphones as u8, v.to_beacn_key(vn)),
            Message::Lighting(v) => (BeacnMessage::Lighting as u8, v.to_beacn_key(vn)),
            Message::MicSetup(v) => (BeacnMessage::MicSetup as u8, v.to_beacn_key(vn)),
            Message::Subwoofer(v) => (BeacnMessage::Subwoofer as u8, v.to_beacn_key(vn)),
            Message::Suppressor(v) => (BeacnMessage::Suppressor as u8, v.to_beacn_key(vn)),
            Message::Controls(v) => (BeacnMessage::Controls as u8, v.to_beacn_key(vn)),
        };

        // Build the Key
        let mut key = [0; 3];
        key[0] = top;
        key[1..3].copy_from_slice(&sub);

        key
    }

    pub fn to_beacn_value(&self, vn: VersionNumber) -> BeacnValue {
        match self {
            Message::BassEnhancement(v) => v.to_beacn_value(vn),
            Message::Compressor(v) => v.to_beacn_value(vn),
            Message::DeEsser(v) => v.to_beacn_value(vn),
            Message::EQMicrophone(v) => v.to_beacn_value(vn),
            Message::EQHeadphones(v) => v.to_beacn_value(vn),
            Message::Exciter(v) => v.to_beacn_value(vn),
            Message::Expander(v) => v.to_beacn_value(vn),
            Message::EQHPLegacy(v) => v.to_beacn_value(vn),
            Message::Headphones(v) => v.to_beacn_value(vn),
            Message::Lighting(v) => v.to_beacn_value(vn),
            Message::MicSetup(v) => v.to_beacn_value(vn),
            Message::Subwoofer(v) => v.to_beacn_value(vn),
            Message::Suppressor(v) => v.to_beacn_value(vn),
            Message::Controls(v) => v.to_beacn_value(vn),
        }
    }

    pub fn from_beacn_message(bytes: [u8; 8], device_type: DeviceType, v: VersionNumber) -> Self {
        // Grab the initial type
        let message = bytes[0];

        // Ok, we need to first split the header and the value
        let key: [u8; 2] = bytes[1..3].try_into().unwrap();
        let value: BeacnValue = bytes[4..8].try_into().unwrap();

        match message {
            0x00 => Self::Headphones(Headphones::from_beacn(key, value, device_type, v)),
            0x01 => Self::Lighting(Lighting::from_beacn(key, value, device_type, v)),
            0x02 => Self::EQMicrophone(EQMicrophone::from_beacn(key, value, device_type, v)),
            0x03 => match v > EQ_HEADPHONES_VERSION {
                true => Self::EQHeadphones(EQHeadphones::from_beacn(key, value, device_type, v)),
                false => Self::EQHPLegacy(EQHPLegacy::from_beacn(key, value, device_type, v)),
            },
            0x04 => Self::BassEnhancement(BassEnhancement::from_beacn(key, value, device_type, v)),
            0x05 => Self::Compressor(Compressor::from_beacn(key, value, device_type, v)),
            0x06 => Self::DeEsser(DeEsser::from_beacn(key, value, device_type, v)),
            0x07 => Self::Exciter(Exciter::from_beacn(key, value, device_type, v)),
            0x08 => Self::Expander(Expander::from_beacn(key, value, device_type, v)),
            0x09 => Self::Suppressor(Suppressor::from_beacn(key, value, device_type, v)),
            0x0a => Self::MicSetup(MicSetup::from_beacn(key, value, device_type, v)),
            0x0b => Self::Subwoofer(Subwoofer::from_beacn(key, value, device_type, v)),
            0x0c => Self::Controls(Controls::from_beacn(key, value, device_type, v)),
            _ => panic!("Not Found!"),
        }
    }

    pub fn generate_fetch_message(device_type: DeviceType, v: VersionNumber) -> Vec<Message> {
        let mut msg = Vec::new();
        msg.append(&mut BassEnhancement::generate_fetch_message(device_type, v));
        msg.append(&mut DeEsser::generate_fetch_message(device_type, v));
        msg.append(&mut EQMicrophone::generate_fetch_message(device_type, v));
        msg.append(&mut Exciter::generate_fetch_message(device_type, v));
        msg.append(&mut Expander::generate_fetch_message(device_type, v));
        msg.append(&mut EQHPLegacy::generate_fetch_message(device_type, v));
        msg.append(&mut Headphones::generate_fetch_message(device_type, v));
        msg.append(&mut Lighting::generate_fetch_message(device_type, v));
        msg.append(&mut MicSetup::generate_fetch_message(device_type, v));
        msg.append(&mut Subwoofer::generate_fetch_message(device_type, v));
        msg.append(&mut Suppressor::generate_fetch_message(device_type, v));
        msg.append(&mut Controls::generate_fetch_message(device_type, v));

        msg
    }

    pub fn is_same_target(&self, other: &Self) -> bool {
        match (self, other) {
            (Message::BassEnhancement(a), Message::BassEnhancement(b)) => a.is_same_target(b),
            (Message::Compressor(a), Message::Compressor(b)) => a.is_same_target(b),
            (Message::DeEsser(a), Message::DeEsser(b)) => a.is_same_target(b),
            (Message::EQMicrophone(a), Message::EQMicrophone(b)) => a.is_same_target(b),
            (Message::Exciter(a), Message::Exciter(b)) => a.is_same_target(b),
            (Message::Expander(a), Message::Expander(b)) => a.is_same_target(b),
            (Message::EQHPLegacy(a), Message::EQHPLegacy(b)) => a.is_same_target(b),
            (Message::Headphones(a), Message::Headphones(b)) => a.is_same_target(b),
            (Message::Lighting(a), Message::Lighting(b)) => a.is_same_target(b),
            (Message::MicSetup(a), Message::MicSetup(b)) => a.is_same_target(b),
            (Message::Subwoofer(a), Message::Subwoofer(b)) => a.is_same_target(b),
            (Message::Suppressor(a), Message::Suppressor(b)) => a.is_same_target(b),
            (Message::Controls(a), Message::Controls(b)) => a.is_same_target(b),
            _ => false, // different top-level groups entirely
        }
    }
}

pub enum BeacnMessage {
    Headphones = 0x00, // HeadphoneMessage
    Lighting = 0x01,
    EQMicrophone = 0x02,
    EQHeadphones = 0x03,
    BassEnhance = 0x04,
    Compressor = 0x05,
    DeEsser = 0x06,
    Exciter = 0x07,
    Expander = 0x08,
    Suppressor = 0x09,
    MicSetup = 0x0a,
    Subwoofer = 0x0b,
    Controls = 0x0c,
}

pub(crate) enum DeviceMessageType {
    Common,
    BeacnMic,
    BeacnStudio,
}

trait BeacnSubMessage {
    fn get_device_message_type(&self) -> DeviceMessageType;
    fn get_message_minimum_version(&self) -> VersionNumber {
        VERSION_MIN_ALL
    }
    fn get_message_maximum_version(&self) -> VersionNumber {
        VERSION_MAX_ALL
    }

    fn is_device_message_set(&self) -> bool;

    fn to_beacn_key(&self, v: VersionNumber) -> [u8; 2];
    fn to_beacn_value(&self, v: VersionNumber) -> BeacnValue;

    fn from_beacn(
        key: [u8; 2],
        value: BeacnValue,
        device_type: DeviceType,
        version: VersionNumber,
    ) -> Self;

    fn generate_fetch_message(device_type: DeviceType, version: VersionNumber) -> Vec<Message>;
}
