use crate::common::logging::configure_logging;
use beacn_lib::MaybeFuture;
use beacn_lib::audio::messages::Message;
use beacn_lib::audio::messages::lighting::{Lighting, LightingBrightness};
use beacn_lib::audio::{BeacnAudioDevice, open_audio_device};
use beacn_lib::manager::{DeviceType, get_beacn_mic_devices, get_beacn_studio_devices};
use log::{error, info};

#[path = "common/mod.rs"]
mod common;

#[cfg(target_arch = "wasm32")]
compile_error!("Sync Examples are not supported under WASM, use the async variant instead.");

fn main() {
    configure_logging();

    // Firstly, find any Mix and Mix Create devices
    let mics = get_beacn_mic_devices().wait();
    let studios = get_beacn_studio_devices().wait();

    let mut device_maps = vec![];

    for (devices, device_type) in [
        (mics, DeviceType::BeacnMic),
        (studios, DeviceType::BeacnStudio),
    ] {
        for device in devices {
            match open_audio_device(device).wait() {
                Ok(dev) => device_maps.push(Device {
                    device: dev,
                    device_type,
                }),
                Err(e) => error!("Failed to open device: {:?}", e),
            }
        }
    }

    // Ok, lets fetch all the current device configs..
    for dev in &device_maps {
        let messages = Message::generate_fetch_message(dev.device_type, dev.device.get_version());
        for message in messages {
            info!("Request {:?}", message);

            let result = dev.device.handle_message(message).wait();
            match result {
                Ok(msg) => {
                    // This response actually works as a setter on the device as well, you can
                    // handle_message(msg) to set the value back to the device.
                    info!("Response: {:?}", msg);
                    info!("---");
                }
                Err(e) => error!("Failed to send message: {:?}", e),
            }
        }
    }

    // Lets send a lighting brightness message to the device
    let message = Message::Lighting(Lighting::Brightness(LightingBrightness(50)));
    for dev in &device_maps {
        info!("Request {:?}", message);
        let result = dev.device.handle_message(message).wait();
        match result {
            Ok(msg) => info!("Response: {:?}", msg),
            Err(e) => error!("Failed to send message: {:?}", e),
        }
    }
}

struct Device {
    device: Box<dyn BeacnAudioDevice>,
    device_type: DeviceType,
}
