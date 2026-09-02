use crate::generate_range;
use crate::types::sealed::Sealed;
use crate::types::{BeacnValue, PackedEnumKey, ReadBeacn, WriteBeacn};
use byteorder::{ByteOrder, LittleEndian};
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

generate_range!(EQGain, f32, -12.0..=12.0);
generate_range!(EQFrequency, f32, 20.0..=20000.0, u32);
generate_range!(EQQ, f32, 0.1..=10.0);

// This is a shared type, so we don't have to worry so much about having to implement to or
// from beacn multiple times. We just wrap our enum in this.
#[derive(
    Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq, Serialize, Deserialize,
)]
pub enum EQSubType {
    #[default]
    Zero = 0x00,
    One = 0x01,
}

impl Sealed for EQSubType {}
impl WriteBeacn for EQSubType {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

impl ReadBeacn for EQSubType {
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

impl From<u8> for EQSubType {
    fn from(value: u8) -> Self {
        for var in Self::iter() {
            if var as u8 == value {
                return var;
            }
        }
        panic!("Unable to Locate Value")
    }
}

#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EQBand {
    Band1 = 0x00,
    Band2 = 0x01,
    Band3 = 0x02,
    Band4 = 0x03,
    Band5 = 0x04,
    Band6 = 0x05,
    Band7 = 0x06,
    Band8 = 0x07,
    Band9 = 0x08,
}
impl From<EQBand> for u8 {
    fn from(value: EQBand) -> Self {
        value as u8
    }
}

#[derive(
    Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq, Serialize, Deserialize,
)]
pub enum EQBandType {
    #[default]
    NotSet = 0x00,
    LowPassFilter = 0x01,
    HighPassFilter = 0x02,
    NotchFilter = 0x03,
    BellBand = 0x04,
    LowShelf = 0x05,
    HighShelf = 0x06,
}

impl Sealed for EQBandType {}
impl WriteBeacn for EQBandType {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}
impl ReadBeacn for EQBandType {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for var in Self::iter() {
            if var as u32 == value {
                return var;
            }
        }
        panic!("Unable to Locate Value {:?}", value)
    }
}

// Base Keys struct, the code after will do all the work with wrapping it to the correct
// type, we just need a version to pick.
#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub(crate) enum EQKeys {
    Type,
    Gain,
    Frequency,
    Q,
    Enabled,
}

// Used currently, they all got shifted because:
// EQMicrophoneMode = 0x00 (previous)
// EQHeadphoneLinked = 0x01 (new)
//
// The packed enum of an EQ band is Band + Key, so band 0 with field 'Type' would previously have a
// presented packed key of 0x01, which now collides with the link command. So these keys have been
// incremented to prevent that.. ugh.
#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
enum EQKeysModern {
    Type = 0x02,      // BandType
    Gain = 0x03,      // f32 (-12..=12)
    Frequency = 0x04, // f32 (20..=20000)
    Q = 0x05,         // f32 (0.1..=10)
    Enabled = 0x06,   // boolean
}
impl From<EQKeysModern> for u8 {
    fn from(value: EQKeysModern) -> Self {
        value as u8
    }
}

// Used pre-1.3.0, can't wait for this to become LegacyLegacy :D
#[derive(Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
enum EQKeysLegacy {
    Type = 0x01,      // BandType
    Gain = 0x02,      // f32 (-12..=12)
    Frequency = 0x03, // f32 (20..=20000)
    Q = 0x04,         // f32 (0.1..=10)
    Enabled = 0x05,   // boolean
}
impl From<EQKeysLegacy> for u8 {
    fn from(value: EQKeysLegacy) -> Self {
        value as u8
    }
}

pub(crate) enum EQKeySet {
    Modern,
    Legacy,
}

impl EQKeySet {
    pub fn encode(self, band: EQBand, key: EQKeys) -> u8 {
        match self {
            Self::Modern => {
                let key = match key {
                    EQKeys::Q => EQKeysModern::Q,
                    EQKeys::Type => EQKeysModern::Type,
                    EQKeys::Gain => EQKeysModern::Gain,
                    EQKeys::Frequency => EQKeysModern::Frequency,
                    EQKeys::Enabled => EQKeysModern::Enabled,
                };

                PackedEnumKey(band, key).to_encoded()
            }

            Self::Legacy => {
                let key = match key {
                    EQKeys::Q => EQKeysLegacy::Q,
                    EQKeys::Type => EQKeysLegacy::Type,
                    EQKeys::Gain => EQKeysLegacy::Gain,
                    EQKeys::Frequency => EQKeysLegacy::Frequency,
                    EQKeys::Enabled => EQKeysLegacy::Enabled,
                };

                PackedEnumKey(band, key).to_encoded()
            }
        }
    }

    pub fn decode(self, encoded: u8) -> Option<(EQBand, EQKeys)> {
        match self {
            Self::Modern => {
                let key = PackedEnumKey::<EQBand, EQKeysModern>::from_encoded(encoded)?;
                let band = key.get_upper();
                let key = match key.get_lower() {
                    EQKeysModern::Q => EQKeys::Q,
                    EQKeysModern::Type => EQKeys::Type,
                    EQKeysModern::Gain => EQKeys::Gain,
                    EQKeysModern::Frequency => EQKeys::Frequency,
                    EQKeysModern::Enabled => EQKeys::Enabled,
                };

                Some((band, key))
            }

            Self::Legacy => {
                let key = PackedEnumKey::<EQBand, EQKeysLegacy>::from_encoded(encoded)?;
                let band = key.get_upper();
                let key = match key.get_lower() {
                    EQKeysLegacy::Q => EQKeys::Q,
                    EQKeysLegacy::Type => EQKeys::Type,
                    EQKeysLegacy::Gain => EQKeys::Gain,
                    EQKeysLegacy::Frequency => EQKeys::Frequency,
                    EQKeysLegacy::Enabled => EQKeys::Enabled,
                };

                Some((band, key))
            }
        }
    }
}
