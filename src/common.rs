use crate::manager::{DeviceLocation, VENDOR_BEACN};
use crate::version::VersionNumber;
use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use rusb::{Device, DeviceDescriptor, DeviceHandle, GlobalContext};
use std::io::{Cursor, Read, Seek};

pub(crate) struct DeviceDefinition {
    pub(crate) device: Device<GlobalContext>,
    pub(crate) descriptor: DeviceDescriptor,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct BeacnDeviceHandle {
    pub(crate) descriptor: DeviceDescriptor,
    pub(crate) device: Device<GlobalContext>,
    pub(crate) handle: DeviceHandle<GlobalContext>,
    pub(crate) version: VersionNumber,
    pub(crate) serial: String,
}

pub(crate) fn find_device(location: DeviceLocation) -> Option<DeviceDefinition> {
    // We need to iterate through the devices and find the one at this location
    if let Ok(devices) = rusb::devices() {
        for device in devices.iter() {
            if let Ok(descriptor) = device.device_descriptor() {
                #[allow(clippy::collapsible_if)]
                if descriptor.vendor_id() == VENDOR_BEACN {
                    if DeviceLocation::from(device.clone()) == location {
                        return Some(DeviceDefinition { device, descriptor });
                    }
                }
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
