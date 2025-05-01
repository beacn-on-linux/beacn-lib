use crate::messages::bass_enhancement::BassEnhancement;
use crate::messages::compressor::Compressor;
use crate::messages::deesser::DeEsser;
use crate::messages::equaliser::Equaliser;
use crate::messages::exciter::Exciter;
use crate::messages::expander::Expander;
use crate::messages::headphone_equaliser::HeadphoneEQ;
use crate::messages::headphones::Headphones;
use crate::messages::lighting::Lighting;
use crate::messages::mic_setup::MicSetup;
use crate::messages::subwoofer::Subwoofer;
use crate::messages::suppressor::Suppressor;
use crate::types::BeacnValue;

pub mod bass_enhancement;
pub mod compressor;
pub mod deesser;
pub mod equaliser;
pub mod exciter;
pub mod expander;
pub mod headphone_equaliser;
pub mod headphones;
pub mod lighting;
pub mod mic_setup;
pub mod subwoofer;
pub mod suppressor;

#[derive(Debug)]
pub enum Message {
    BassEnhancement(BassEnhancement),
    Compressor(Compressor),
    DeEsser(DeEsser),
    Equaliser(Equaliser),
    Exciter(Exciter),
    Expander(Expander),
    HeadphoneEQ(HeadphoneEQ),
    Headphones(Headphones),
    Lighting(Lighting),
    MicSetup(MicSetup),
    Subwoofer(Subwoofer),
    Suppressor(Suppressor),
}

impl Message {
    pub fn to_beacn_key(&self) -> [u8; 3] {
        let (top, sub) = match self {
            Message::BassEnhancement(v) => (BeacnMessage::BassEnhancement as u8, v.to_beacn_key()),
            Message::Compressor(v) => (BeacnMessage::Compressor as u8, v.to_beacn_key()),
            Message::DeEsser(v) => (BeacnMessage::DeEsser as u8, v.to_beacn_key()),
            Message::Equaliser(v) => (BeacnMessage::Equaliser as u8, v.to_beacn_key()),
            Message::Exciter(v) => (BeacnMessage::Exciter as u8, v.to_beacn_key()),
            Message::Expander(v) => (BeacnMessage::Expander as u8, v.to_beacn_key()),
            Message::HeadphoneEQ(v) => (BeacnMessage::HeadphoneEQ as u8, v.to_beacn_key()),
            Message::Headphones(v) => (BeacnMessage::Headphones as u8, v.to_beacn_key()),
            Message::Lighting(v) => (BeacnMessage::Lighting as u8, v.to_beacn_key()),
            Message::MicSetup(v) => (BeacnMessage::MicSetup as u8, v.to_beacn_key()),
            Message::Subwoofer(v) => (BeacnMessage::Subwoofer as u8, v.to_beacn_key()),
            Message::Suppressor(v) => (BeacnMessage::Suppressor as u8, v.to_beacn_key()),
        };

        // Build the Key
        let mut key = [0; 3];
        key[0] = top;
        key[1..3].copy_from_slice(&sub);

        key
    }

    pub fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Message::BassEnhancement(v) => v.to_beacn_value(),
            Message::Compressor(v) => v.to_beacn_value(),
            Message::DeEsser(v) => v.to_beacn_value(),
            Message::Equaliser(v) => v.to_beacn_value(),
            Message::Exciter(v) => v.to_beacn_value(),
            Message::Expander(v) => v.to_beacn_value(),
            Message::HeadphoneEQ(v) => v.to_beacn_value(),
            Message::Headphones(v) => v.to_beacn_value(),
            Message::Lighting(v) => v.to_beacn_value(),
            Message::MicSetup(v) => v.to_beacn_value(),
            Message::Subwoofer(v) => v.to_beacn_value(),
            Message::Suppressor(v) => v.to_beacn_value(),
        }
    }

    pub fn from_beacn_message(bytes: [u8; 8]) -> Self {
        // Grab the initial type
        let message = bytes[0];

        // Ok, we need to first split the header and the value
        let key: [u8; 2] = bytes[1..3].try_into().unwrap();
        let value: BeacnValue = bytes[4..8].try_into().unwrap();

        match message {
            0x00 => Self::Headphones(Headphones::from_beacn(key, value)),
            0x01 => Self::Lighting(Lighting::from_beacn(key, value)),
            0x02 => Self::Equaliser(Equaliser::from_beacn(key, value)),
            0x03 => Self::HeadphoneEQ(HeadphoneEQ::from_beacn(key, value)),
            0x04 => Self::BassEnhancement(BassEnhancement::from_beacn(key, value)),
            0x05 => Self::Compressor(Compressor::from_beacn(key, value)),
            0x06 => Self::DeEsser(DeEsser::from_beacn(key, value)),
            0x07 => Self::Exciter(Exciter::from_beacn(key, value)),
            0x08 => Self::Expander(Expander::from_beacn(key, value)),
            0x09 => Self::Suppressor(Suppressor::from_beacn(key, value)),
            0x0a => Self::MicSetup(MicSetup::from_beacn(key, value)),
            0x0b => Self::Subwoofer(Subwoofer::from_beacn(key, value)),
            _ => panic!("Not Found!"),
        }
    }

    pub fn generate_fetch_message() -> Vec<Message> {
        let mut messages = Vec::new();
        messages.append(&mut BassEnhancement::generate_fetch_message());
        messages.append(&mut Compressor::generate_fetch_message());
        messages.append(&mut DeEsser::generate_fetch_message());
        messages.append(&mut Equaliser::generate_fetch_message());
        messages.append(&mut Exciter::generate_fetch_message());
        messages.append(&mut Expander::generate_fetch_message());
        messages.append(&mut HeadphoneEQ::generate_fetch_message());
        messages.append(&mut Headphones::generate_fetch_message());
        messages.append(&mut Lighting::generate_fetch_message());
        messages.append(&mut MicSetup::generate_fetch_message());
        messages.append(&mut Subwoofer::generate_fetch_message());
        messages.append(&mut Suppressor::generate_fetch_message());

        messages
    }
}

pub enum BeacnMessage {
    Headphones = 0x00, // HeadphoneMessage
    Lighting = 0x01,
    Equaliser = 0x02,
    HeadphoneEQ = 0x03,
    BassEnhancement = 0x04,
    Compressor = 0x05,
    DeEsser = 0x06,
    Exciter = 0x07,
    Expander = 0x08,
    Suppressor = 0x09,
    MicSetup = 0x0a,
    Subwoofer = 0x0b,
}

trait BeacnSubMessage {
    fn to_beacn_key(&self) -> [u8; 2];
    fn to_beacn_value(&self) -> BeacnValue;

    fn from_beacn(key: [u8; 2], value: BeacnValue) -> Self;
    fn generate_fetch_message() -> Vec<Message>;
}
