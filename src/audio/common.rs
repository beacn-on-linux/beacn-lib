use crate::audio::messages::{DeviceMessageType, Message};
use crate::audio::{BeacnAudioDevice, DeviceDefinition};
use crate::common::{BeacnDeviceHandle, get_device_info};
use crate::manager::DeviceType;
use anyhow::{Result, bail};
use log::{debug, warn};
use rusb::{DeviceHandle, GlobalContext};
use std::time::Duration;

// This defines the code needed for connecting to a Beacn Audio Device, it's currently consistent
// between the Mic and Studio, so we'll have a common base implementation for open()
pub trait BeacnAudioDeviceAttach {
    // We're specifically allowing the DeviceDefinition to be a private interface, as it's
    // simply used internally for connection up a device, and shouldn't have any visibility
    // from the outside. This also prevents external code from attempting to call connect.
    #[allow(private_interfaces)]
    fn connect(device: DeviceDefinition) -> Result<Box<dyn BeacnAudioDevice>>
    where
        Self: Sized;

    fn get_product_id(&self) -> u16;
    fn get_serial(&self) -> String;
    fn get_version(&self) -> String;
}

pub trait BeacnAudioMessageExecute {
    fn get_device_type(&self) -> DeviceType;
    fn get_usb_handle(&self) -> &DeviceHandle<GlobalContext>;
}

// Trait for Sending and Receiving Messages
pub trait BeacnAudioMessaging: BeacnAudioMessageExecute {
    fn is_command_valid(&self, message: Message) -> bool {
        // TODO: We need to somehow cleanly map message_type to device_type
        let message_type = message.get_device_message_type();
        let device_type = self.get_device_type();
        match message_type {
            DeviceMessageType::Common => true,
            DeviceMessageType::BeacnMic => device_type == DeviceType::BeacnMic,
            DeviceMessageType::BeacnStudio => device_type == DeviceType::BeacnStudio,
        }
    }

    fn fetch_value(&self, message: Message) -> Result<Message> {
        // Before we do anything, we need to make sure this message is valid on our device
        if !self.is_command_valid(message) {
            warn!("Command Sent not valid for this device:");
            warn!("{:?}", &message);
            bail!("Command is not valid for this device");
        }

        // Ok, first we need to deconstruct this message into something more useful
        let key = message.to_beacn_key();

        // Lookup the Parameter on the Mic
        let param = self.param_lookup(key)?;

        Ok(Message::from_beacn_message(param, self.get_device_type()))
    }

    fn set_value(&self, message: Message) -> anyhow::Result<Message> {
        if !self.is_command_valid(message) {
            warn!("Command Sent not valid for this device:");
            warn!("{:?}", message);
            bail!("Command is not valid for this device");
        }

        let key = message.to_beacn_key();
        let value = message.to_beacn_value();

        let result = self.param_set(key, value)?;

        // This can generally be ignored, because in most cases it'll be identical to the
        // original request (except fed from the Mic), but passing back anyway just in case.
        Ok(Message::from_beacn_message(result, self.get_device_type()))
    }

    fn param_lookup(&self, key: [u8; 3]) -> Result<[u8; 8]> {
        let timeout = Duration::from_secs(3);

        let mut request = [0; 4];
        request[0..3].copy_from_slice(&key);
        request[3] = 0xa3;

        // Write out the command request
        self.get_usb_handle().write_bulk(0x03, &request, timeout)?;

        // Grab the response into a buffer
        let mut buf = [0; 8];
        self.get_usb_handle().read_bulk(0x83, &mut buf, timeout)?;

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
        self.get_usb_handle().write_bulk(0x03, &request, timeout)?;

        // Check whether the value has changed
        let new_value = self.param_lookup(key)?;

        let old = &request[4..8];
        let new = &new_value[4..8];

        // Compare the new response
        if old != new {
            warn!(
                "Value Set: {:?} does not match value on Device: {:?}",
                &old, &new
            );
            bail!("Value was not changed on the device!");
        }
        Ok(new_value)
    }
}

/// Simple function to Open a libusb connection to a Beacn Audio device, do initial setup and
/// grab the firmware version from the device.
pub(crate) fn open_beacn(def: DeviceDefinition, product_id: u16) -> Result<BeacnDeviceHandle> {
    if def.descriptor.product_id() != product_id {
        bail!(
            "Expecting PID {} but got {}",
            product_id,
            def.descriptor.product_id()
        );
    }

    let handle = def.device.open()?;
    handle.claim_interface(3)?;
    handle.set_alternate_setting(3, 1)?;
    handle.clear_halt(0x83)?;

    let setup_timeout = Duration::from_millis(2000);

    let request = [0x00, 0x00, 0x00, 0xa0];
    handle.write_bulk(0x03, &request, setup_timeout)?;

    // Mic and Studio use bulk reads to get this data
    let mut input = [0; 512];
    let request = [0x00, 0x00, 0x00, 0xa1];
    handle.write_bulk(0x03, &request, setup_timeout)?;
    handle.read_bulk(0x83, &mut input, setup_timeout)?;

    // So, this is consistent between the Mix Create and the Mic :D
    let (version, serial) = get_device_info(&input)?;

    debug!(
        "Loaded Device, Location: {}.{}, Serial: {}, Version: {}",
        def.device.bus_number(),
        def.device.address(),
        serial.clone(),
        version
    );

    Ok(BeacnDeviceHandle {
        descriptor: def.descriptor,
        device: def.device,
        handle,
        version,
        serial,
    })
}
