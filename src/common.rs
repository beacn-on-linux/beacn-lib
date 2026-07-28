use crate::manager::{DeviceLocation, VENDOR_BEACN};
use crate::version::VersionNumber;
use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use nusb::{Device, DeviceInfo, Interface};
use std::io::{Cursor, Read, Seek};

pub struct DeviceDefinition {
    pub(crate) descriptor: DeviceInfo,
}

#[derive(Debug)]
#[allow(unused)]
pub struct BeacnDeviceHandle {
    pub(crate) descriptor: DeviceInfo,
    pub(crate) device: Device,
    pub(crate) interface: Interface,
    pub(crate) version: VersionNumber,
    pub(crate) serial: String,
}

pub(crate) async fn find_device(location: DeviceLocation) -> Option<DeviceDefinition> {
    // We need to iterate through the devices and find the one at this location
    if let Ok(devices) = crate::setup::list_devices().await {
        for info in devices {
            if info.vendor_id() == VENDOR_BEACN && DeviceLocation::from(&info) == location {
                return Some(DeviceDefinition { descriptor: info });
            }
        }
    }
    None
}

pub(crate) fn get_device_info(input: &[u8]) -> Result<(VersionNumber, String)> {
    let mut cursor = Cursor::new(input);
    cursor.seek_relative(4)?;

    let version = cursor.read_u32::<LittleEndian>()?;

    // Break it down
    let major = version >> 0x1c;
    let minor = (version >> 0x18) & 0xf;
    let patch = (version >> 0x10) & 0xff;
    let build = version & 0xffff;

    let version = VersionNumber(major, minor, patch, build);

    // Now grab the Serial...
    let mut serial_bytes = vec![];
    for byte in cursor.bytes() {
        let byte = byte?;

        // Check for Null Termination
        if byte == 0 {
            break;
        }
        serial_bytes.push(byte);
    }
    let serial = String::from_utf8_lossy(&serial_bytes)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();

    Ok((version, serial))
}
