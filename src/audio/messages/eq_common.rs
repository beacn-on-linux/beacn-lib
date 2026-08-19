use crate::generate_range;
use crate::types::sealed::Sealed;
use crate::types::{BeacnValue, ReadBeacn, WriteBeacn};
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
