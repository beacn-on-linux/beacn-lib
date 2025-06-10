use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message};
use crate::generate_range;
use crate::manager::DeviceType;
use crate::types::sealed::Sealed;
use crate::types::{BeacnValue, RGBA, ReadBeacn, WriteBeacn, read_value, write_value};
use byteorder::{ByteOrder, LittleEndian};
use enum_map::Enum;
use strum::{EnumIter, IntoEnumIterator};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Lighting {
    GetMode,
    Mode(LightingMode),

    GetStudioMode,
    StudioMode(StudioLightingMode),

    GetColour1,
    Colour1(RGBA),

    GetColour2,
    Colour2(RGBA),

    GetSpeed,
    Speed(LightingSpeed),

    GetBrightness,
    Brightness(LightingBrightness),

    GetMeterSource,
    MeterSource(LightingMeterSource),

    GetMeterSensitivity,
    MeterSensitivity(LightingMeterSensitivty),

    GetMuteMode,
    MuteMode(LightingMuteMode),

    GetMuteColour,
    MuteColour(RGBA),

    GetSuspendMode,
    SuspendMode(LightingSuspendMode),

    GetSuspendBrightness,
    SuspendBrightness(LightingSuspendBrightness),
}

impl BeacnSubMessage for Lighting {
    fn get_device_message_type(&self) -> DeviceMessageType {
        match self {
            Lighting::GetMode | Lighting::Mode(_) => DeviceMessageType::BeacnMic,
            Lighting::GetStudioMode | Lighting::StudioMode(_) => DeviceMessageType::BeacnStudio,
            _ => DeviceMessageType::Common,
        }
    }

    fn is_device_message_set(&self) -> bool {
        matches!(
            self,
            Lighting::Mode(_)
                | Lighting::StudioMode(_)
                | Lighting::Colour1(_)
                | Lighting::Colour2(_)
                | Lighting::Speed(_)
                | Lighting::Brightness(_)
                | Lighting::MeterSource(_)
                | Lighting::MeterSensitivity(_)
                | Lighting::MuteMode(_)
                | Lighting::MuteColour(_)
                | Lighting::SuspendMode(_)
                | Lighting::SuspendBrightness(_)
        )
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Lighting::GetMode | Lighting::Mode(_) => [0x00, 0x00],
            Lighting::GetStudioMode | Lighting::StudioMode(_) => [0x00, 0x00],
            Lighting::GetColour1 | Lighting::Colour1(_) => [0x01, 0x00],
            Lighting::GetColour2 | Lighting::Colour2(_) => [0x02, 0x00],
            Lighting::GetSpeed | Lighting::Speed(_) => [0x04, 0x00],
            Lighting::GetBrightness | Lighting::Brightness(_) => [0x05, 0x00],
            Lighting::GetMeterSource | Lighting::MeterSource(_) => [0x06, 0x00],
            Lighting::GetMeterSensitivity | Lighting::MeterSensitivity(_) => [0x07, 0x00],
            Lighting::GetMuteMode | Lighting::MuteMode(_) => [0x08, 0x00],
            Lighting::GetMuteColour | Lighting::MuteColour(_) => [0x09, 0x00],
            Lighting::GetSuspendMode | Lighting::SuspendMode(_) => [0x0b, 0x00],
            Lighting::GetSuspendBrightness | Lighting::SuspendBrightness(_) => [0x0c, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Lighting::Mode(v) => v.write_beacn(),
            Lighting::StudioMode(v) => v.write_beacn(),
            Lighting::Colour1(v) => v.write_beacn(),
            Lighting::Colour2(v) => v.write_beacn(),
            Lighting::Speed(v) => write_value(v),
            Lighting::Brightness(v) => write_value(v),
            Lighting::MeterSource(v) => v.write_beacn(),
            Lighting::MeterSensitivity(v) => write_value(v),
            Lighting::MuteMode(v) => v.write_beacn(),
            Lighting::MuteColour(v) => v.write_beacn(),
            Lighting::SuspendMode(v) => v.write_beacn(),
            Lighting::SuspendBrightness(v) => write_value(v),
            _ => panic!("Attempting to Set a Get"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, device_type: DeviceType) -> Self {
        match key[0] {
            0x00 => match device_type {
                DeviceType::BeacnMic => Self::Mode(LightingMode::read_beacn(&value)),
                DeviceType::BeacnStudio => Self::StudioMode(StudioLightingMode::read_beacn(&value)),
                _ => panic!("This isn't an Audio Device!"),
            },
            0x01 => Self::Colour1(RGBA::read_beacn(&value)),
            0x02 => Self::Colour2(RGBA::read_beacn(&value)),
            0x04 => Self::Speed(read_value(&value)),
            0x05 => Self::Brightness(read_value(&value)),
            0x06 => Self::MeterSource(LightingMeterSource::read_beacn(&value)),
            0x07 => Self::MeterSensitivity(read_value(&value)),
            0x08 => Self::MuteMode(LightingMuteMode::read_beacn(&value)),
            0x09 => Self::MuteColour(RGBA::read_beacn(&value)),
            0x0b => Self::SuspendMode(LightingSuspendMode::read_beacn(&value)),
            0x0c => Self::SuspendBrightness(read_value(&value)),
            _ => panic!("Unexpected Key: {}", key[0]),
        }
    }

    fn generate_fetch_message(device_type: DeviceType) -> Vec<Message> {
        let mode = match device_type {
            DeviceType::BeacnMic => Message::Lighting(Lighting::GetMode),
            DeviceType::BeacnStudio => Message::Lighting(Lighting::GetStudioMode),
            _ => panic!("This isn't an Audio Device!"),
        };

        vec![
            mode,
            Message::Lighting(Lighting::GetColour1),
            Message::Lighting(Lighting::GetColour2),
            Message::Lighting(Lighting::GetSpeed),
            Message::Lighting(Lighting::GetBrightness),
            Message::Lighting(Lighting::GetMeterSource),
            Message::Lighting(Lighting::GetMeterSensitivity),
            Message::Lighting(Lighting::GetMuteMode),
            Message::Lighting(Lighting::GetMuteColour),
            Message::Lighting(Lighting::GetSuspendMode),
            Message::Lighting(Lighting::GetSuspendBrightness),
        ]
    }
}

generate_range!(LightingSpeed, i32, -10..=10);
generate_range!(LightingBrightness, i32, 0..=100);
generate_range!(LightingMeterSensitivty, f32, 0.0..=10.);
generate_range!(LightingSuspendBrightness, u32, 0..=100);

// enum LightingK {
//     Mode = 0x00,              // LightingMode
//     Colour1 = 0x01,           // BGRA
//     Colour2 = 0x02,           // BGRA
//     Speed = 0x04,             // i32 (-10..=10)
//     Brightness = 0x05,        // i32 (0..=100)
//     MeterSource = 0x06,       // LightingMeterSource
//     MeterSensitivity = 0x07,  // f32 (0..=10)
//     MuteMode = 0x08,          // LightingMuteMode
//     MuteColour = 0x09,        // BGRA
//     SuspendMode = 0x0b,       // LightingSuspendMode
//     SuspendBrightness = 0x0c, // u32 (0..=100)    // VERIFY THIS, SHOULD MATCH Brightness
// }

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum LightingMode {
    #[default]
    Solid = 0x00,
    Spectrum = 0x01,
    Gradient = 0x02,
    ReactiveRing = 0x05,
    ReactiveMeterUp = 0x06,
    ReactiveMeterDown = 0x07,
    SparkleRandom = 0x0a,
    SparkleMeter = 0x0b,
}
impl Sealed for LightingMode {}
impl ReadBeacn for LightingMode {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for mode in Self::iter() {
            if mode as u32 == value {
                return mode;
            }
        }
        panic!("Unable to Find Mode")
    }
}
impl WriteBeacn for LightingMode {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum StudioLightingMode {
    #[default]
    Solid = 0x00,
    PeakMeter = 0x05,
    SolidSpectrum = 0x0d,
}
impl Sealed for StudioLightingMode {}
impl ReadBeacn for StudioLightingMode {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for mode in Self::iter() {
            if mode as u32 == value {
                return mode;
            }
        }
        panic!("Unable to Find Mode")
    }
}
impl WriteBeacn for StudioLightingMode {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum LightingMuteMode {
    #[default]
    Nothing = 0x00,
    Solid = 0x01,
    Off = 0x02,
}

impl Sealed for LightingMuteMode {}
impl ReadBeacn for LightingMuteMode {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for mode in Self::iter() {
            if mode as u32 == value {
                return mode;
            }
        }
        panic!("Unable to Find Mode")
    }
}
impl WriteBeacn for LightingMuteMode {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum LightingSuspendMode {
    #[default]
    Nothing = 0x00,
    Off = 0x01,
    Brightness = 0x02,
}
impl Sealed for LightingSuspendMode {}
impl ReadBeacn for LightingSuspendMode {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for mode in Self::iter() {
            if mode as u32 == value {
                return mode;
            }
        }
        panic!("Unable to Find Mode")
    }
}
impl WriteBeacn for LightingSuspendMode {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum LightingMeterSource {
    #[default]
    Microphone = 0x00,
    Headphones = 0x01,
}
impl Sealed for LightingMeterSource {}
impl ReadBeacn for LightingMeterSource {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for mode in Self::iter() {
            if mode as u32 == value {
                return mode;
            }
        }
        panic!("Unable to Find Mode")
    }
}
impl WriteBeacn for LightingMeterSource {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}
