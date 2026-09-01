use crate::audio::data::BulkMessage;
use crate::audio::messages::{DeviceMessageType, Message};
use crate::audio::{BeacnAudioDevice, DeviceDefinition, LinkChannel, LinkedApp};
use crate::common::BeacnDeviceInfo;
use crate::manager::DeviceType;
use crate::sealed::Sealed;
use crate::sync::AsyncMutex as Mutex;
use crate::transfer::{EndpointHandle, transfer};
use crate::{BResult, beacn_bail};
use async_trait::async_trait;
use byteorder::{ByteOrder, LittleEndian};
use log::{error, warn};
use nusb::transfer::{Bulk, In, Out};
use web_time::Duration;

/// This is a bulk endpoint pair. These are mutexed together to prevent
/// the potential of different threads (or async tasks) attempting to interact with
/// the device at the same time; access is treated one at a time.
pub struct AudioEndpoints {
    pub(crate) out_ep: EndpointHandle<Bulk, Out>,
    pub(crate) in_ep: EndpointHandle<Bulk, In>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub(crate) trait BeacnAudioDeviceInternal: Sealed {
    // We're specifically allowing the DeviceDefinition to be a private interface, as it's
    // simply used internally for connection up a device, and shouldn't have any visibility
    // from the outside. This also prevents external code from attempting to call connect.

    async fn connect(definition: DeviceDefinition) -> BResult<Box<dyn BeacnAudioDevice>>
    where
        Self: Sized;

    fn get_device_type(&self) -> DeviceType;
    fn get_endpoints(&self) -> &Mutex<AudioEndpoints>;
}

// Trait for Sending and Receiving Messages
#[allow(private_bounds)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait BeacnAudioAPI: BeacnAudioDeviceInternal + BeacnAudioMessageLocal + Sealed {
    async fn handle_message(&self, message: Message) -> BResult<Message> {
        if message.is_device_message_set() {
            self.set_value(message).await
        } else {
            self.fetch_value(message).await
        }
    }

    async fn handle_bulk_message(&self, message: BulkMessage) -> BResult<BulkMessage> {
        self.fetch_bulk(message).await
    }

    async fn get_linked_apps(&self) -> BResult<Option<Vec<LinkedApp>>> {
        self.get_app_links().await
    }
    async fn set_linked_app(&self, app: LinkedApp) -> BResult<()> {
        self.set_app_link(app).await
    }
}

// Stuff that is local to this instance
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub(crate) trait BeacnAudioMessageLocal:
    BeacnAudioDeviceInternal + BeacnDeviceInfo + Sealed
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

    async fn fetch_value(&self, message: Message) -> BResult<Message> {
        // Before we do anything, we need to make sure this message is valid on our device
        if !self.is_command_valid(&message) {
            warn!("Cannot Fetch, Message not valid for this device:");
            warn!("{:?}", message);
            beacn_bail!("Cannot Fetch, Message not valid for this device.");
        }

        if !self.is_command_firmware_valid(&message) {
            beacn_bail!("Command is not valid for this firmware version");
        }

        // Ok, first we need to deconstruct this message into something more useful
        let key = message.to_beacn_key(self.get_version());

        // Lookup the Parameter on the Mic
        let param = self.param_lookup(key).await?;

        Ok(Message::from_beacn_message(
            param,
            self.get_device_type(),
            self.get_version(),
        ))
    }

    async fn set_value(&self, message: Message) -> BResult<Message> {
        if !self.is_command_valid(&message) {
            warn!("Command Sent, Message not valid for this device:");
            warn!("{:?}", message);
            beacn_bail!("Command Sent, Message not valid for this device");
        }

        if !self.is_command_firmware_valid(&message) {
            beacn_bail!("Command is not valid for this firmware version");
        }

        let key = message.to_beacn_key(self.get_version());
        let value = message.to_beacn_value(self.get_version());
        let validate = message.should_validate_response();
        let result = self.param_set(key, value, validate).await?;

        // This can generally be ignored, because in most cases it'll be identical to the
        // original request (except fed from the Mic), but passing back anyway just in case.
        Ok(Message::from_beacn_message(
            result,
            self.get_device_type(),
            self.get_version(),
        ))
    }

    async fn param_lookup(&self, key: [u8; 3]) -> BResult<[u8; 8]> {
        let timeout = Duration::from_secs(3);

        let mut request = [0; 4];
        request[0..3].copy_from_slice(&key);
        request[3] = 0xa3;

        let mut ep = self.get_endpoints().lock().await;

        // Write out the command request
        transfer(&mut ep.out_ep, request.into(), timeout).await?;

        // Grab the response into a buffer
        let max_packet_size = ep.in_ep.get_mut()?.max_packet_size();
        let buffer = Vec::with_capacity(max_packet_size);
        let completion = transfer(&mut ep.in_ep, buffer, timeout).await?;

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

    async fn param_set(&self, key: [u8; 3], value: [u8; 4], validate: bool) -> BResult<[u8; 8]> {
        let timeout = Duration::from_millis(200);

        // Build the Set Request
        let mut request = [0; 8];
        request[0..3].copy_from_slice(&key);
        request[3] = 0xa4;
        request[4..].copy_from_slice(&value);

        {
            let mut endpoints = self.get_endpoints().lock().await;
            transfer(&mut endpoints.out_ep, request.into(), timeout).await?;
        }

        // Check whether the value has changed
        let new_value = self.param_lookup(key).await?;

        let old = &request[4..8];
        let new = &new_value[4..8];

        // Compare the new response
        if validate && old != new {
            // If we're validating this, we should reject this because the value that was returned
            // was different from the value which was sent, however, there are some minor cases
            // where that behaviour is actually expected (Controls->Balance, sends an i32, returns
            // a f32)
            error!("Send Failed: Expecting: {:?} != Received: {:?}", old, new);
            beacn_bail!("Value was not changed on the device!");
        }

        Ok(new_value)
    }

    async fn fetch_bulk(&self, message: BulkMessage) -> BResult<BulkMessage> {
        let timeout = Duration::from_secs(3);

        if !message.is_valid_fetch() {
            warn!("Cannot Fetch, Message not valid for this device:");
            beacn_bail!("Message is not a valid request");
        }

        let mut request = [0; 4];
        let key = message.to_beacn_key();
        request[0..3].copy_from_slice(&key);
        request[3] = 0xa5;

        let mut ep = self.get_endpoints().lock().await;

        // Write out the command request
        transfer(&mut ep.out_ep, request.into(), timeout).await?;

        // Grab the response into a buffer
        let max_packet_size = ep.in_ep.get_mut()?.max_packet_size();
        let buffer = Vec::with_capacity(max_packet_size);
        let completion = transfer(&mut ep.in_ep, buffer, timeout).await?;

        // Ok, we need to convert this into a BulkMessage
        Ok(BulkMessage::handle_response(
            &message,
            &completion.into_vec(),
            self.get_device_type(),
        )?)
    }

    /// Returns the Apps and their link configuration from PC2
    async fn get_app_links(&self) -> BResult<Option<Vec<LinkedApp>>> {
        let mut apps = vec![];

        if self.get_device_type() != DeviceType::BeacnStudio {
            beacn_bail!("This can only be executed on a Beacn Studio")
        }

        let timeout = Duration::from_secs(3);

        // Build the request
        let request = [0x00, 0x00, 0x01, 0xAC];

        let mut endpoints = self.get_endpoints().lock().await;
        transfer(&mut endpoints.out_ep, request.into(), timeout).await?;

        // TODO: Assuming max length of 1024, it might be higher
        let completion = transfer(&mut endpoints.in_ep, Vec::with_capacity(1024), timeout).await?;
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

    async fn set_app_link(&self, link: LinkedApp) -> BResult<()> {
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
        let mut endpoints = self.get_endpoints().lock().await;
        transfer(&mut endpoints.out_ep, message, timeout).await?;

        Ok(())
    }
}
