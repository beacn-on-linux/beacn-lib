use crate::audio::messages::{BeacnSubMessage, DeviceMessageType, Message, VERSION_ALL};

use crate::generate_range;
use crate::manager::DeviceType;
use crate::types::sealed::Sealed;
use crate::types::{BeacnValue, ReadBeacn, WriteBeacn, read_value, write_value};
use crate::version::VersionNumber;
use byteorder::{ByteOrder, LittleEndian};
use enum_map::Enum;
use strum::{EnumIter, IntoEnumIterator};

const MIC_CLASS_COMPLIANT_VERSION: VersionNumber = VersionNumber(1, 2, 0, 188);

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Headphones {
    GetHeadphoneLevel,
    HeadphoneLevel(HPLevel),

    GetMicMonitor,
    MicMonitor(HPMicMonitorLevel),

    GetStudioMicMonitor,
    StudioMicMonitor(HPMicMonitorLevel),

    GetMicChannelsLinked,
    MicChannelsLinked(bool),

    GetStudioChannelsLinked,
    StudioChannelsLinked(bool),

    GetMicOutputGain,
    MicOutputGain(HPMicOutputGain),

    GetHeadphoneType,
    HeadphoneType(HeadphoneTypes),

    GetFXEnabled,
    FXEnabled(bool),

    GetStudioDriverless,
    StudioDriverless(bool),

    GetMicClassCompliant,
    MicClassCompliant(bool),
}

impl BeacnSubMessage for Headphones {
    fn get_device_message_type(&self) -> DeviceMessageType {
        match self {
            Headphones::GetMicMonitor => DeviceMessageType::BeacnMic,
            Headphones::MicMonitor(_) => DeviceMessageType::BeacnMic,
            Headphones::GetStudioMicMonitor => DeviceMessageType::BeacnStudio,
            Headphones::StudioMicMonitor(_) => DeviceMessageType::BeacnStudio,
            Headphones::GetMicChannelsLinked => DeviceMessageType::BeacnMic,
            Headphones::MicChannelsLinked(_) => DeviceMessageType::BeacnMic,
            Headphones::GetStudioChannelsLinked => DeviceMessageType::BeacnStudio,
            Headphones::StudioChannelsLinked(_) => DeviceMessageType::BeacnStudio,
            Headphones::GetStudioDriverless => DeviceMessageType::BeacnStudio,
            Headphones::StudioDriverless(_) => DeviceMessageType::BeacnStudio,
            Headphones::GetMicClassCompliant => DeviceMessageType::BeacnMic,
            Headphones::MicClassCompliant(_) => DeviceMessageType::BeacnMic,
            _ => DeviceMessageType::Common,
        }
    }

    fn get_message_minimum_version(&self) -> VersionNumber {
        match self {
            Headphones::GetMicClassCompliant | Headphones::MicClassCompliant(_) => {
                MIC_CLASS_COMPLIANT_VERSION
            }
            _ => VERSION_ALL,
        }
    }

    fn is_device_message_set(&self) -> bool {
        matches!(
            self,
            Headphones::HeadphoneLevel(_)
                | Headphones::MicMonitor(_)
                | Headphones::StudioMicMonitor(_)
                | Headphones::MicChannelsLinked(_)
                | Headphones::StudioChannelsLinked(_)
                | Headphones::MicOutputGain(_)
                | Headphones::HeadphoneType(_)
                | Headphones::FXEnabled(_)
                | Headphones::StudioDriverless(_)
                | Headphones::MicClassCompliant(_)
        )
    }

    fn to_beacn_key(&self) -> [u8; 2] {
        match self {
            Headphones::HeadphoneLevel(_) | Headphones::GetHeadphoneLevel => [0x04, 0x00],
            Headphones::MicMonitor(_) | Headphones::GetMicMonitor => [0x06, 0x00],
            Headphones::StudioMicMonitor(_) | Headphones::GetStudioMicMonitor => [0x07, 0x00],
            Headphones::MicChannelsLinked(_) | Headphones::GetMicChannelsLinked => [0x07, 0x00],
            Headphones::StudioChannelsLinked(_) | Headphones::GetStudioChannelsLinked => {
                [0x08, 0x00]
            }
            Headphones::MicOutputGain(_) | Headphones::GetMicOutputGain => [0x10, 0x00],
            Headphones::HeadphoneType(_) | Headphones::GetHeadphoneType => [0x11, 0x00],
            Headphones::FXEnabled(_) | Headphones::GetFXEnabled => [0x12, 0x00],
            Headphones::StudioDriverless(_)
            | Headphones::GetStudioDriverless
            | Headphones::MicClassCompliant(_)
            | Headphones::GetMicClassCompliant => [0x14, 0x00],
        }
    }

    fn to_beacn_value(&self) -> BeacnValue {
        match self {
            Headphones::HeadphoneLevel(v) => write_value(v),
            Headphones::MicMonitor(v) => write_value(v),
            Headphones::StudioMicMonitor(v) => write_value(v),
            Headphones::MicChannelsLinked(v) => v.write_beacn(),
            Headphones::StudioChannelsLinked(v) => v.write_beacn(),
            Headphones::MicOutputGain(v) => write_value(v),
            Headphones::HeadphoneType(v) => v.write_beacn(),
            Headphones::FXEnabled(v) => v.write_beacn(),
            Headphones::StudioDriverless(v) => {
                if v == &true {
                    DeviceMode::Compliancy.write_beacn()
                } else {
                    DeviceMode::StudioDefault.write_beacn()
                }
            }
            Headphones::MicClassCompliant(v) => {
                if v == &true {
                    DeviceMode::Compliancy.write_beacn()
                } else {
                    DeviceMode::MicDefault.write_beacn()
                }
            }
            _ => panic!("Attempted to get Value on Setter"),
        }
    }

    fn from_beacn(key: [u8; 2], value: BeacnValue, device_type: DeviceType) -> Self {
        match key[0] {
            0x04 => Self::HeadphoneLevel(read_value(&value)),
            0x06 => Self::MicMonitor(read_value(&value)),
            0x07 => match device_type {
                DeviceType::BeacnMic => Self::MicChannelsLinked(bool::read_beacn(&value)),
                DeviceType::BeacnStudio => Self::StudioMicMonitor(read_value(&value)),
                _ => panic!("This isn't an Audio Device!"),
            },
            0x08 => Self::StudioChannelsLinked(bool::read_beacn(&value)),
            0x10 => Self::MicOutputGain(read_value(&value)),
            0x11 => Self::HeadphoneType(HeadphoneTypes::read_beacn(&value)),
            0x12 => Self::FXEnabled(bool::read_beacn(&value)),
            0x14 => {
                // The values on this are a little ominous, it's technically an enum, but it's
                // also a boolean,
                let mode = DeviceMode::read_beacn(&value);
                match device_type {
                    DeviceType::BeacnMic => {
                        if mode == DeviceMode::MicDefault {
                            Self::MicClassCompliant(false)
                        } else {
                            Self::MicClassCompliant(true)
                        }
                    }
                    DeviceType::BeacnStudio => {
                        if mode == DeviceMode::StudioDefault {
                            Self::StudioDriverless(false)
                        } else {
                            Self::StudioDriverless(true)
                        }
                    }
                    _ => panic!("This isn't an Audio Device!"),
                }
            }
            _ => panic!("Unexpected Key: {}", key[0]),
        }
    }

    fn generate_fetch_message(device_type: DeviceType) -> Vec<Message> {
        let mut messages = vec![
            Message::Headphones(Headphones::GetHeadphoneLevel),
            Message::Headphones(Headphones::GetMicOutputGain),
            Message::Headphones(Headphones::GetHeadphoneType),
            Message::Headphones(Headphones::GetFXEnabled),
        ];
        match device_type {
            DeviceType::BeacnMic => {
                messages.push(Message::Headphones(Headphones::GetMicMonitor));
                messages.push(Message::Headphones(Headphones::GetMicChannelsLinked));
                messages.push(Message::Headphones(Headphones::GetMicClassCompliant));
            }
            DeviceType::BeacnStudio => {
                messages.push(Message::Headphones(Headphones::GetStudioMicMonitor));
                messages.push(Message::Headphones(Headphones::GetStudioChannelsLinked));
                messages.push(Message::Headphones(Headphones::GetStudioDriverless));
            }
            _ => panic!("This isn't an Audio Device!"),
        }

        messages
    }
}

generate_range!(HPLevel, f32, -70.0..=-0.0);
generate_range!(HPMicMonitorLevel, f32, -100.0..=6.0);
generate_range!(HPMicOutputGain, f32, 0.0..=12.0);

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum HeadphoneTypes {
    #[default]
    LineLevel = 0x00,
    NormalPower = 0x01,
    HighImpedance = 0x02,
    InEarMonitors = 0x03,
}

impl Sealed for HeadphoneTypes {}
impl WriteBeacn for HeadphoneTypes {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

impl ReadBeacn for HeadphoneTypes {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for var in Self::iter() {
            if var as u32 == value {
                return var;
            }
        }
        panic!("Could not Find Value");
    }
}

#[derive(Default, Copy, Clone, Hash, Enum, EnumIter, Debug, Eq, PartialEq)]
pub enum DeviceMode {
    #[default]
    Compliancy = 0x00,
    StudioDefault = 0x01,
    MicDefault = 0x02,
}
impl Sealed for DeviceMode {}
impl WriteBeacn for DeviceMode {
    fn write_beacn(&self) -> BeacnValue {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, *self as u8 as u32);
        buf
    }
}

impl ReadBeacn for DeviceMode {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        for var in Self::iter() {
            if var as u32 == value {
                return var;
            }
        }
        panic!("Could not Find Value");
    }
}
