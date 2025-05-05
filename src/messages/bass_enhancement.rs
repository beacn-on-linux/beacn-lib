use crate::generate_range;
use crate::messages::bass_enhancement::BassPreset::{Preset1, Preset2, Preset3, Preset4};
use crate::messages::{BeacnSubMessage, Message};
use crate::types::{
    BeacnValue, MakeUpGain, Percent, ReadBeacn, TimeFrame, WriteBeacn, read_value, write_value,
};
use byteorder::{ByteOrder, LittleEndian};
use enum_map::Enum;
use strum::{EnumIter, IntoEnumIterator};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BassEnhancement {
    GetDrive,
    Drive(BassDrive),

    GetMix,
    Mix(Percent),

    GetEnabled,
    Enabled(bool),

    GetPreset,
    Preset(BassPreset),

    GetAmount,
    Amount(BassAmount),

    // Realistically, a user shouldn't be calling these directly, but they
    // need to exist so that we can load the presets, what I'll likely do
    // is have helper functions which instead generates a list of commands
    // to change the preset.
    GetAttack,
    Attack(TimeFrame),

    GetRelease,
    Release(TimeFrame),

    GetThreshold,
    Threshold(BassThreshold),

    GetKnee,
    Knee(BassKnee),

    GetMakeupGain,
    MakeupGain(MakeUpGain),

    GetRatio,
    Ratio(BassRatio),

    GetCutoff,
    Cutoff(BassCutoff),

    GetQ,
    Q(BassQ),

    GetLowerCutoff,
    LowerCutoff(BassCutoff),

    GetLowerQ,
    LowerQ(BassQ),
}

impl BeacnSubMessage for BassEnhancement {
    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            BassEnhancement::Attack(_) | BassEnhancement::GetAttack => [0x00, 0x00],
            BassEnhancement::Release(_) | BassEnhancement::GetRelease => [0x01, 0x00],
            BassEnhancement::Threshold(_) | BassEnhancement::GetThreshold => [0x02, 0x00],
            BassEnhancement::Knee(_) | BassEnhancement::GetKnee => [0x03, 0x00],
            BassEnhancement::MakeupGain(_) | BassEnhancement::GetMakeupGain => [0x04, 0x00],
            BassEnhancement::Ratio(_) | BassEnhancement::GetRatio => [0x05, 0x00],
            BassEnhancement::Cutoff(_) | BassEnhancement::GetCutoff => [0x06, 0x00],
            BassEnhancement::Q(_) | BassEnhancement::GetQ => [0x07, 0x00],
            BassEnhancement::LowerCutoff(_) | BassEnhancement::GetLowerCutoff => [0x08, 0x00],
            BassEnhancement::LowerQ(_) | BassEnhancement::GetLowerQ => [0x09, 0x00],
            BassEnhancement::Drive(_) | BassEnhancement::GetDrive => [0x0a, 0x00],
            BassEnhancement::Mix(_) | BassEnhancement::GetMix => [0x0b, 0x00],
            BassEnhancement::Enabled(_) | BassEnhancement::GetEnabled => [0x0c, 0x00],
            BassEnhancement::Preset(_) | BassEnhancement::GetPreset => [0x0d, 0x00],
            BassEnhancement::Amount(_) | BassEnhancement::GetAmount => [0x0e, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            BassEnhancement::Drive(v) => write_value(v),
            BassEnhancement::Mix(v) => write_value(v),
            BassEnhancement::Enabled(v) => v.write_beacn(),
            BassEnhancement::Preset(v) => v.write_beacn(),
            BassEnhancement::Amount(v) => write_value(v),
            BassEnhancement::Attack(v) => write_value(v),
            BassEnhancement::Release(v) => write_value(v),
            BassEnhancement::Threshold(v) => write_value(v),
            BassEnhancement::Knee(v) => write_value(v),
            BassEnhancement::MakeupGain(v) => write_value(v),
            BassEnhancement::Ratio(v) => write_value(v),
            BassEnhancement::Cutoff(v) => write_value(v),
            BassEnhancement::Q(v) => write_value(v),
            BassEnhancement::LowerCutoff(v) => write_value(v),
            BassEnhancement::LowerQ(v) => write_value(v),
            _ => panic!("Attempting to Set value for Getter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue) -> Self {
        match key[0] {
            0x00 => Self::Attack(read_value(&value)),
            0x01 => Self::Release(read_value(&value)),
            0x02 => Self::Threshold(read_value(&value)),
            0x03 => Self::Knee(read_value(&value)),
            0x04 => Self::MakeupGain(read_value(&value)),
            0x05 => Self::Ratio(read_value(&value)),
            0x06 => Self::Cutoff(read_value(&value)),
            0x07 => Self::Q(read_value(&value)),
            0x08 => Self::LowerCutoff(read_value(&value)),
            0x09 => Self::LowerQ(read_value(&value)),
            0x0a => Self::Drive(read_value(&value)),
            0x0b => Self::Mix(read_value(&value)),
            0x0c => Self::Enabled(bool::read_beacn(&value)),
            0x0d => Self::Preset(BassPreset::read_beacn(&value)),
            0x0e => Self::Amount(read_value(&value)),
            _ => panic!("Unexpected Bass Enhancement Key: {}", key[0]),
        }
    }

    fn generate_fetch_message() -> Vec<Message> {
        vec![
            Message::BassEnhancement(BassEnhancement::GetDrive),
            Message::BassEnhancement(BassEnhancement::GetMix),
            Message::BassEnhancement(BassEnhancement::GetEnabled),
            Message::BassEnhancement(BassEnhancement::GetPreset),
            Message::BassEnhancement(BassEnhancement::GetAmount),
            Message::BassEnhancement(BassEnhancement::GetAttack),
            Message::BassEnhancement(BassEnhancement::GetRelease),
            Message::BassEnhancement(BassEnhancement::GetThreshold),
            Message::BassEnhancement(BassEnhancement::GetKnee),
            Message::BassEnhancement(BassEnhancement::GetMakeupGain),
            Message::BassEnhancement(BassEnhancement::GetRatio),
            Message::BassEnhancement(BassEnhancement::GetCutoff),
            Message::BassEnhancement(BassEnhancement::GetQ),
            Message::BassEnhancement(BassEnhancement::GetLowerCutoff),
            Message::BassEnhancement(BassEnhancement::GetLowerQ),
        ]
    }
}

impl BassEnhancement {
    pub fn get_preset(preset: BassPreset) -> Vec<Message> {
        match preset {
            Preset1 => vec![
                Message::BassEnhancement(BassEnhancement::Preset(Preset1)),
                Message::BassEnhancement(BassEnhancement::Attack(TimeFrame(10.0))),
                Message::BassEnhancement(BassEnhancement::Release(TimeFrame(250.0))),
                Message::BassEnhancement(BassEnhancement::Threshold(BassThreshold(-27.0))),
                Message::BassEnhancement(BassEnhancement::Knee(BassKnee(2.0))),
                Message::BassEnhancement(BassEnhancement::MakeupGain(MakeUpGain(6.0))),
                Message::BassEnhancement(BassEnhancement::Ratio(BassRatio(8.0))),
                Message::BassEnhancement(BassEnhancement::Cutoff(BassCutoff(102.0))),
                Message::BassEnhancement(BassEnhancement::Q(BassQ(0.7))),
                Message::BassEnhancement(BassEnhancement::LowerCutoff(BassCutoff(10.0))),
                Message::BassEnhancement(BassEnhancement::LowerQ(BassQ(0.2))),
            ],
            Preset2 => vec![
                Message::BassEnhancement(BassEnhancement::Preset(Preset2)),
                Message::BassEnhancement(BassEnhancement::Attack(TimeFrame(10.0))),
                Message::BassEnhancement(BassEnhancement::Release(TimeFrame(250.0))),
                Message::BassEnhancement(BassEnhancement::Threshold(BassThreshold(-21.0))),
                Message::BassEnhancement(BassEnhancement::Knee(BassKnee(2.0))),
                Message::BassEnhancement(BassEnhancement::MakeupGain(MakeUpGain(8.0))),
                Message::BassEnhancement(BassEnhancement::Ratio(BassRatio(5.5))),
                Message::BassEnhancement(BassEnhancement::Cutoff(BassCutoff(105.0))),
                Message::BassEnhancement(BassEnhancement::Q(BassQ(0.9))),
                Message::BassEnhancement(BassEnhancement::LowerCutoff(BassCutoff(40.0))),
                Message::BassEnhancement(BassEnhancement::LowerQ(BassQ(0.2))),
            ],
            Preset3 => vec![
                Message::BassEnhancement(BassEnhancement::Preset(Preset3)),
                Message::BassEnhancement(BassEnhancement::Attack(TimeFrame(10.0))),
                Message::BassEnhancement(BassEnhancement::Release(TimeFrame(250.0))),
                Message::BassEnhancement(BassEnhancement::Threshold(BassThreshold(0.0))),
                Message::BassEnhancement(BassEnhancement::Knee(BassKnee(3.0))),
                Message::BassEnhancement(BassEnhancement::MakeupGain(MakeUpGain(0.0))),
                Message::BassEnhancement(BassEnhancement::Ratio(BassRatio(16.0))),
                Message::BassEnhancement(BassEnhancement::Cutoff(BassCutoff(160.0))),
                Message::BassEnhancement(BassEnhancement::Q(BassQ(0.8))),
                Message::BassEnhancement(BassEnhancement::LowerCutoff(BassCutoff(30.0))),
                Message::BassEnhancement(BassEnhancement::LowerQ(BassQ(0.7))),
            ],
            Preset4 => vec![
                Message::BassEnhancement(BassEnhancement::Preset(Preset4)),
                Message::BassEnhancement(BassEnhancement::Attack(TimeFrame(10.0))),
                Message::BassEnhancement(BassEnhancement::Release(TimeFrame(250.0))),
                Message::BassEnhancement(BassEnhancement::Threshold(BassThreshold(-30.0))),
                Message::BassEnhancement(BassEnhancement::Knee(BassKnee(3.0))),
                Message::BassEnhancement(BassEnhancement::MakeupGain(MakeUpGain(0.0))),
                Message::BassEnhancement(BassEnhancement::Ratio(BassRatio(8.0))),
                Message::BassEnhancement(BassEnhancement::Cutoff(BassCutoff(150.0))),
                Message::BassEnhancement(BassEnhancement::Q(BassQ(0.7))),
                Message::BassEnhancement(BassEnhancement::LowerCutoff(BassCutoff(30.0))),
                Message::BassEnhancement(BassEnhancement::LowerQ(BassQ(0.7))),
            ],
        }
    }

    pub fn get_amount(amount: f32) -> Vec<Message> {
        vec![
            Message::BassEnhancement(BassEnhancement::Amount(BassAmount(amount))),
            Message::BassEnhancement(BassEnhancement::Drive(BassDrive(3.2 * amount))),
            Message::BassEnhancement(BassEnhancement::Mix(Percent(amount * 10.0))),
        ]
    }
}

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum BassPreset {
    #[default]
    Preset1 = 0x00,
    Preset2 = 0x01,
    Preset3 = 0x02,
    Preset4 = 0x03,
}

impl crate::types::sealed::Sealed for BassPreset {}
impl WriteBeacn for BassPreset {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_f32(&mut buf, *self as u8 as f32);
        buf
    }
}

impl ReadBeacn for BassPreset {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_f32(buf);
        for var in Self::iter() {
            if var as u8 as f32 == value {
                return var;
            }
        }
        panic!("Unable to Find Mode")
    }
}

generate_range!(BassDrive, f32, 0.0..=32.0);
generate_range!(BassAmount, f32, 0.0..=10.0);
generate_range!(BassThreshold, f32, -50.0..=0.0);
generate_range!(BassKnee, f32, 0.0..=5.0);
generate_range!(BassRatio, f32, 0.0..=16.0);
generate_range!(BassCutoff, f32, 0.0..=160.0);
generate_range!(BassQ, f32, 0.0..=16.0);
