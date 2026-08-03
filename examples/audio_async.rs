use env_logger::Env;
use log::{error, info};
use beacn_lib::audio::messages::Message;
use beacn_lib::audio::messages::lighting::{Lighting, LightingBrightness};
use beacn_lib::audio::{BeacnAudioDevice, open_audio_device};
use beacn_lib::manager::{DeviceType, get_beacn_mic_devices, get_beacn_studio_devices};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Firstly, find any Mix and Mix Create devices
    let mics = get_beacn_mic_devices().await;
    let studios = get_beacn_studio_devices().await;

    let mut device_maps = vec![];

    for (devices, device_type) in [
        (mics, DeviceType::BeacnMic),
        (studios, DeviceType::BeacnStudio),
    ] {
        for device in devices {
            match open_audio_device(device).await {
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
        let messages = Message::generate_fetch_message(dev.device_type);
        for message in messages {
            info!("Request {:?}", message);

            let result = dev.device.handle_message(message).await;
            match result {
                Ok(msg) => {
                    // This response actually works as a setter on the device as well, you can
                    // handle_message(msg).await to set the value back to the device.
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
        let result = dev.device.handle_message(message).await;
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
