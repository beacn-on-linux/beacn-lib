use std::time::Duration;
use anyhow::{bail, Result};
use log::debug;
use rusb::{Device, DeviceDescriptor, DeviceHandle, GlobalContext};
use crate::messages::Message;

const VID_BEACN_MIC: u16 = 0x33ae;
const PID_BEACN_MIC: u16 = 0x0001;

pub struct BeacnMic {
    handle: DeviceHandle<GlobalContext>,
    device: Device<GlobalContext>,
    descriptor: DeviceDescriptor,

    serial: String,
    firmware_version: String,
}

impl BeacnMic {
    pub fn open() -> Result<Self> {
        // Attempt to Locate a Beacn Mic
        let (device, descriptor) = Self::find_devices()?;

        let handle = device.open()?;
        handle.claim_interface(3)?;
        handle.set_alternate_setting(3, 1)?;

        Ok(Self {
            handle,
            device,
            descriptor,

            serial: String::from("NotImplemented"),
            firmware_version: String::from("0.0.0 build 0")
        })
    }

    fn fetch_value(&self, message: Message) -> Result<Message> {
        // Ok, first we need to deconstruct this message into something more useful
        let key = message.to_beacn_key();

        // Lookup the Parameter on the Mic
        let param = self.param_lookup(key)?;

        Ok(Message::from_beacn_message(param))
    }

    fn set_value(&self, message: Message) -> Result<Message> {
        let key = message.to_beacn_key();
        let value = message.to_beacn_value();

        let result = self.param_set(key, value)?;

        // This can generally be ignored, because in most cases it'll be identical to the
        // original request (except fed from the Mic), but passing back anyway just in case.
        Ok(Message::from_beacn_message(result))
    }

    fn find_devices() -> Result<(Device<GlobalContext>, DeviceDescriptor)> {
        if let Ok(devices) = rusb::devices() {
            for device in devices.iter() {
                if let Ok(descriptor) = device.device_descriptor() {
                    let bus_number = device.bus_number();
                    let address = device.address();

                    if descriptor.vendor_id() == VID_BEACN_MIC && descriptor.product_id() == PID_BEACN_MIC {
                        debug!("Found Beacn Mic at address {}.{}", bus_number, address);
                        return Ok((device, descriptor));
                    }
                }
            }
        }
        bail!("Unable to Locate Device")
    }

    fn param_lookup(&self, key: [u8; 3]) -> Result<[u8; 8]> {
        let timeout = Duration::from_secs(3);

        let mut request = [0;4];
        request[0..3].copy_from_slice(&key);
        request[3] = 0xa3;

        // Write out the command request
        self.handle.write_bulk(0x03, &request, timeout)?;

        // Grab the response into a buffer
        let mut buf = [0; 8];
        self.handle.read_bulk(0x83, &mut buf, timeout)?;

        // Validate the header...
        if buf[0..2] != request[0..2] || buf[3] != 0xa4 {
            bail!("Invalid Response Received");
        }

        Ok(buf)
    }

    fn param_set(&self, key: [u8; 3], value: [u8; 4]) -> Result<[u8; 8]> {
        let timeout = Duration::from_millis(200);

        // Build the Set Request
        let mut request = [0; 8];
        request[0..3].copy_from_slice(&key);
        request[3] = 0xa4;
        request[4..].copy_from_slice(&value);

        // Write out the command request
        self.handle.write_bulk(0x03, &request, timeout)?;

        // Check whether the value has changed
        let new_value = self.param_lookup(key)?;

        // Compare the new response
        if new_value != request[4..8] {
            bail!("Value was not changed on the device!");
        }
        Ok(new_value)
    }
}