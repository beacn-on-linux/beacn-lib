use crate::audio::messages::{DeviceMessageType, Message};
use crate::audio::{BeacnAudioDevice, DeviceDefinition, LinkChannel, LinkedApp};
use crate::common::{BeacnDeviceHandle, get_device_info};
use crate::manager::DeviceType;
use crate::version::VersionNumber;
use crate::{BResult, beacn_bail};
use byteorder::{ByteOrder, LittleEndian};
use log::{debug, warn};
use nusb::MaybeFuture;
use nusb::transfer::{Buffer, Bulk, In, Out};
use std::sync::Mutex;
use std::time::Duration;

// This defines the code needed for connecting to a Beacn Audio Device, it's currently consistent
// between the Mic and Studio, so we'll have a common base implementation for open()
pub trait BeacnAudioDeviceAttach {
    // We're specifically allowing the DeviceDefinition to be a private interface, as it's
    // simply used internally for connection up a device, and shouldn't have any visibility
    // from the outside. This also prevents external code from attempting to call connect.
    #[allow(private_interfaces)]
    fn connect(device: DeviceDefinition) -> BResult<Box<dyn BeacnAudioDevice>>
    where
        Self: Sized;

    fn get_product_id(&self) -> u16;
    fn get_serial(&self) -> String;
    fn get_version(&self) -> VersionNumber;
}

/// This is a bulk endpoint pair. These are mutexed together to prevent
/// the potential of different threads attempting to interact with the
/// devices at the same time, and access is treated once at a time.
pub(crate) struct AudioEndpoints {
    pub(crate) out_ep: nusb::Endpoint<Bulk, Out>,
    pub(crate) in_ep: nusb::Endpoint<Bulk, In>,
}

#[allow(private_interfaces)]
pub trait BeacnAudioMessageExecute {
    fn get_device_type(&self) -> DeviceType;
    fn get_endpoints(&self) -> &Mutex<AudioEndpoints>;
}

// Trait for Sending and Receiving Messages
#[allow(private_bounds)]
pub trait BeacnAudioMessaging: BeacnAudioMessageExecute + BeacnAudioMessageLocal {
    fn handle_message(&self, message: Message) -> BResult<Message> {
        if message.is_device_message_set() {
            self.set_value(message)
        } else {
            self.fetch_value(message)
        }
    }

    fn get_linked_app_list(&self) -> BResult<Option<Vec<LinkedApp>>> {
        self.get_linked_apps()
    }
    fn set_linked_app(&self, app: LinkedApp) -> BResult<()> {
        self.set_app_link(app)
    }
}

// Stuff that is local to this instance
pub(crate) trait BeacnAudioMessageLocal:
    BeacnAudioMessageExecute + BeacnAudioDeviceAttach
{
    fn is_command_valid(&self, message: &Message) -> bool {
        let message_type = message.get_device_message_type();
        let device_type = self.get_device_type();
        match message_type {
            DeviceMessageType::Common => true,
            DeviceMessageType::BeacnMic => device_type == DeviceType::BeacnMic,
            DeviceMessageType::BeacnStudio => device_type == DeviceType::BeacnStudio,
        }
    }

    fn is_command_firmware_valid(&self, message: &Message) -> bool {
        let min_version = message.get_message_minimum_version();
        let max_version = message.get_message_maximum_version();
        let device_version = self.get_version();
        if device_version < min_version {
            warn!("Command Sent not valid for this firmware version:");
            warn!("Device: {:?} < {:?}", device_version, min_version);
            warn!("{:?}", message);
            false
        } else if device_version > max_version {
            warn!("Command Sent not valid for this firmware version:");
            warn!("Device: {:?} > {:?}", device_version, min_version);
            warn!("{:?}", message);
            false
        } else {
            true
        }
    }

    fn fetch_value(&self, message: Message) -> BResult<Message> {
        // Before we do anything, we need to make sure this message is valid on our device
        if !self.is_command_valid(&message) {
            warn!("Command Sent not valid for this device:");
            warn!("{:?}", message);
            beacn_bail!("Command is not valid for this device");
        }

        if !self.is_command_firmware_valid(&message) {
            beacn_bail!("Command is not valid for this firmware version");
        }

        // Ok, first we need to deconstruct this message into something more useful
        let key = message.to_beacn_key();

        // Lookup the Parameter on the Mic
        let param = self.param_lookup(key)?;

        Ok(Message::from_beacn_message(param, self.get_device_type()))
    }

    fn set_value(&self, message: Message) -> BResult<Message> {
        if !self.is_command_valid(&message) {
            warn!("Command Sent not valid for this device:");
            warn!("{:?}", message);
            beacn_bail!("Command is not valid for this device");
        }

        if !self.is_command_firmware_valid(&message) {
            beacn_bail!("Command is not valid for this firmware version");
        }

        let key = message.to_beacn_key();
        let value = message.to_beacn_value();

        let result = self.param_set(key, value)?;

        // This can generally be ignored, because in most cases it'll be identical to the
        // original request (except fed from the Mic), but passing back anyway just in case.
        Ok(Message::from_beacn_message(result, self.get_device_type()))
    }

    fn param_lookup(&self, key: [u8; 3]) -> BResult<[u8; 8]> {
        let timeout = Duration::from_secs(3);

        let mut request = [0; 4];
        request[0..3].copy_from_slice(&key);
        request[3] = 0xa3;

        let mut ep = self.get_endpoints().lock().unwrap();

        // Write out the command request
        ep.out_ep
            .transfer_blocking(request.into(), timeout)
            .into_result()?;

        // Grab the response into a buffer
        let max_packet_size = ep.in_ep.max_packet_size();
        let completion = ep
            .in_ep
            .transfer_blocking(Buffer::new(max_packet_size), timeout)
            .into_result()?;

        if completion.len() != 8 {
            beacn_bail!("Invalid Response Length Received");
        }

        let mut buf = [0u8; 8];
        buf.copy_from_slice(&completion[0..8]);

        // Validate the header...
        if buf[0..2] != request[0..2] || buf[3] != 0xa4 {
            beacn_bail!("Invalid Response Received");
        }

        Ok(buf)
    }

    fn param_set(&self, key: [u8; 3], value: [u8; 4]) -> BResult<[u8; 8]> {
        let timeout = Duration::from_millis(200);

        // Build the Set Request
        let mut request = [0; 8];
        request[0..3].copy_from_slice(&key);
        request[3] = 0xa4;
        request[4..].copy_from_slice(&value);

        {
            let mut endpoints = self.get_endpoints().lock().unwrap();
            endpoints
                .out_ep
                .transfer_blocking(request.into(), timeout)
                .into_result()?;
        }

        // Check whether the value has changed
        let new_value = self.param_lookup(key)?;

        let old = &request[4..8];
        let new = &new_value[4..8];

        // Compare the new response
        if old != new {
            warn!(
                "Value Set: {:?} does not match value on Device: {:?}",
                old, new
            );
            beacn_bail!("Value was not changed on the device!");
        }
        Ok(new_value)
    }

    /// Returns the Apps and their link configuration from PC2
    fn get_linked_apps(&self) -> BResult<Option<Vec<LinkedApp>>> {
        let mut apps = vec![];

        if self.get_device_type() != DeviceType::BeacnStudio {
            beacn_bail!("This can only be executed on a Beacn Studio")
        }

        let timeout = Duration::from_secs(3);

        // Build the request
        let request = [0x00, 0x00, 0x01, 0xAC];

        let mut endpoints = self.get_endpoints().lock().unwrap();
        endpoints
            .out_ep
            .transfer_blocking(request.into(), timeout)
            .into_result()?;

        // TODO: Assuming max length of 1024, it might be higher
        let completion = endpoints
            .in_ep
            .transfer_blocking(Buffer::new(1024), timeout)
            .into_result()?;
        let buf = &completion[..];

        // Extract the header
        let data_length = LittleEndian::read_u24(&buf[0..3]) as usize;
        if data_length == 0xFFFFFF {
            // No PC2 Connection
            return Ok(None);
        }

        let data = &buf[4..4 + data_length];
        let mut position = 0;
        loop {
            if position >= data.len() {
                break;
            }

            let len = data[position] as usize;
            if len == 0 {
                break;
            }

            if position + 2 + len > data.len() {
                beacn_bail!("Truncated Entry, aborting");
            }

            let channel = data[position + 1];
            let name = str::from_utf8(&data[position + 2..position + 2 + len])
                .map_err(anyhow::Error::from)?;
            apps.push(LinkedApp {
                channel: LinkChannel::from_u8(channel),
                name: name.to_string(),
            });
            position += 2 + len;
        }

        // Sort alphabetically
        apps.sort_by_key(|app| app.name.to_lowercase());
        Ok(Some(apps))
    }

    fn set_app_link(&self, link: LinkedApp) -> BResult<()> {
        if self.get_device_type() != DeviceType::BeacnStudio {
            beacn_bail!("This can only be executed on a Beacn Studio")
        }

        // Build the packet
        let name_bytes = link.name.as_bytes();

        // I'm honestly unsure about this, it seems to appear with every packet when moving
        // apps between channels, so I'll include it.
        let extra = [0x00, 0xcd, 0xcd, 0xcd, 0xcd, 0x00];
        let length: u8 = (name_bytes.len() + extra.len()) as u8;

        let mut packet: Vec<u8> = Vec::with_capacity(2 + name_bytes.len() + 1 + extra.len());
        packet.push(length);
        packet.push(link.channel as u8);
        packet.extend_from_slice(name_bytes);
        packet.extend_from_slice(&extra);

        let mut message = vec![0x00, 0x00, 0x00, 0xac];
        LittleEndian::write_u24(&mut message[0..3], packet.len() as u32);
        message.extend_from_slice(&packet);

        let timeout = Duration::from_secs(3);
        let mut endpoints = self.get_endpoints().lock().unwrap();
        endpoints
            .out_ep
            .transfer_blocking(message.into(), timeout)
            .into_result()?;

        Ok(())
    }
}

/// Simple function to Open a USB connection to a Beacn Audio device, do initial setup and
/// grab the firmware version from the device.
pub(crate) fn open_beacn(
    def: DeviceDefinition,
    product_id: &[u16],
) -> BResult<(BeacnDeviceHandle, AudioEndpoints)> {
    if !product_id.contains(&def.descriptor.product_id()) {
        beacn_bail!(
            "Expecting PIDs {:?} but got {}",
            product_id,
            def.descriptor.product_id()
        );
    }

    let device = def.descriptor.open().wait()?;
    let interface = device.claim_interface(3).wait()?;
    interface.set_alt_setting(1).wait()?;

    let mut out_ep = interface.endpoint::<Bulk, Out>(0x03)?;
    let mut in_ep = interface.endpoint::<Bulk, In>(0x83)?;
    in_ep.clear_halt().wait()?;

    let setup_timeout = Duration::from_millis(2000);

    let request = [0x00, 0x00, 0x00, 0xa0];
    out_ep
        .transfer_blocking(request.into(), setup_timeout)
        .into_result()?;

    // Mic and Studio use bulk reads to get this data
    let request = [0x00, 0x00, 0x00, 0xa1];
    out_ep
        .transfer_blocking(request.into(), setup_timeout)
        .into_result()?;

    let read_len = in_ep.max_packet_size().max(512);
    let completion = in_ep
        .transfer_blocking(Buffer::new(read_len), setup_timeout)
        .into_result()?;

    // So, this is consistent between the Mix Create and the Mic :D
    let (version, serial) = get_device_info(&completion[..])?;

    debug!(
        "Loaded Device, Location: {}.{}, Serial: {}, Version: {}",
        def.descriptor.bus_id(),
        def.descriptor.device_address(),
        serial.clone(),
        version
    );

    let handle = BeacnDeviceHandle {
        descriptor: def.descriptor,
        device,
        interface,
        version,
        serial,
    };

    Ok((
        handle,
        AudioEndpoints {
            out_ep: out_ep,
            in_ep,
        },
    ))
}
