use crate::manager::{DeviceLocation, DeviceType, VENDOR_BEACN};
use crate::sealed::Sealed;
use crate::transfer::{EndpointHandle, transfer};
use crate::version::VersionNumber;
use crate::{BResult, beacn_bail, setup};
use anyhow::Result;
use async_trait::async_trait;
use byteorder::{LittleEndian, ReadBytesExt};
use log::debug;
use nusb::transfer::{BulkOrInterrupt, EndpointType, In, Out};
use nusb::{Device, DeviceInfo, Interface};
use std::io::{Cursor, Read, Seek};
use web_time::Duration;

pub struct DeviceDefinition {
    pub(crate) descriptor: DeviceInfo,
}

/// Define the Device Kinds
pub trait BeacnDeviceKind: Send + Sync + 'static {
    const PID: &[u16];
    const TYPE: DeviceType;
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct BeacnDeviceHandle {
    pub(crate) descriptor: DeviceInfo,
    pub(crate) device: Device,
    pub(crate) interface: Interface,
    pub(crate) fw_version: VersionNumber,
    pub(crate) serial: String,
}

// This trait gets attached to devices and returns information about them.
#[async_trait]
pub trait BeacnDeviceInfo: Sealed {
    fn get_product_id(&self) -> u16;
    fn get_serial(&self) -> String;
    fn get_version(&self) -> VersionNumber;
}

/// A function to open a Beacn Device
pub(crate) async fn open_device<T>(
    product_id: &[u16],
    definition: DeviceDefinition,
    interface_num: u8,
    firmware_bytes: &[u8],
) -> BResult<BeacnDeviceHandle>
where
    T: EndpointType + BulkOrInterrupt,
{
    if !product_id.contains(&definition.descriptor.product_id()) {
        beacn_bail!(
            "Expecting PIDs {:?} but got {}",
            product_id,
            definition.descriptor.product_id()
        );
    }

    let device = setup::open(&definition.descriptor).await?;
    let interface = setup::claim_interface(&device, interface_num).await?;
    setup::set_alt_setting(&interface, 1).await?;

    // Create some endpoints, caller tells us the type
    let mut out_ep = EndpointHandle::<T, Out>::new(interface.clone(), 0x03)?;
    let mut in_ep = EndpointHandle::<T, In>::new(interface.clone(), 0x83)?;

    let setup_timeout = Duration::from_millis(2000);
    let read_len = in_ep.get_mut()?.max_packet_size().max(64);

    for byte in firmware_bytes {
        transfer(&mut out_ep, [0, 0, 0, *byte].into(), setup_timeout).await?;
    }

    let completion = transfer(&mut in_ep, Vec::with_capacity(read_len), setup_timeout).await?;
    let (version, serial) = get_device_info(&completion[..])?;

    #[cfg(not(target_arch = "wasm32"))]
    debug!(
        "Loaded Device, Location: {}.{}, Serial: {}, Version: {}",
        definition.descriptor.bus_id(),
        definition.descriptor.device_address(),
        serial.clone(),
        version
    );

    #[cfg(target_arch = "wasm32")]
    debug!(
        "Loaded Device, Serial: {}, Version: {}",
        serial.clone(),
        version
    );

    Ok(BeacnDeviceHandle {
        descriptor: definition.descriptor,
        device,
        interface,
        fw_version: version,
        serial,
    })
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
