use beacn_lib::MaybeFuture;
use beacn_lib::controller::{BeacnControlDevice, Interactions, open_control_device};
use beacn_lib::manager::{DeviceLocation, get_beacn_mix_create_device, get_beacn_mix_device};

use crate::common::controller::{test_buttons, test_pattern};
use crate::common::logging::configure_logging;
use beacn_lib::controller::messages::Message;
use flume::Receiver;
use log::{error, info, warn};
use std::sync::Arc;
use web_time::Duration;

#[path = "common/mod.rs"]
mod common;

#[cfg(target_arch = "wasm32")]
compile_error!("Sync Examples are not supported under WASM, use the async variant instead.");

fn main() {
    configure_logging();

    // Firstly, find any Mix and Mix Create devices
    let mut devices = get_beacn_mix_device().wait();
    devices.extend(get_beacn_mix_create_device().wait());

    let mut device_maps = vec![];

    for device in devices {
        let (interaction_tx, interaction_rx) = flume::unbounded();
        let (health_tx, health_rx) = flume::unbounded();

        let dev = open_control_device(device.clone(), Some(interaction_tx), health_tx).wait();
        let dev = match dev {
            Ok(dev) => dev,
            Err(e) => {
                error!("Failed to open device: {:?}", e);
                continue;
            }
        };

        device_maps.push(Device {
            device: dev,
            location: device.clone(),
            health: health_rx,
            interactions: interaction_rx,
        })
    }

    if device_maps.is_empty() {
        warn!("No usable devices found!");
        return;
    }

    // Spawn up a ticker..
    let mut step = 0;
    let (tick_tx, tick_rx) = flume::unbounded();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            if tick_tx.send(()).is_err() {
                break;
            }
        }
    });

    // Ok, we're built up now, lets listen for messages from the devices
    'primary: loop {
        let mut selector = flume::Selector::new();

        for device in &device_maps {
            let location = device.location.clone();
            selector = selector.recv(&device.interactions, move |msg| {
                info!("[{}] {:?}", location, msg);
                false
            });

            let location = device.location.clone();
            selector = selector.recv(&device.health, move |_| {
                error!("[{}] Error on Device Handler!", location);
                false
            });
        }
        selector = selector.recv(&tick_rx, |_| true);

        let tick = selector.wait();
        if tick {
            for device in &device_maps {
                let (x, y, image) = test_pattern(step);
                for (button, colour) in test_buttons(step) {
                    let msg = Message::SetButtonColour(button, colour);
                    let _ = device.device.handle_message(msg).wait();
                }

                let _ = device.device.handle_message(Message::KeepAlive).wait();

                // Send the Image
                let msg = Message::SetImage(x, y, image);
                let _ = device.device.handle_message(msg).wait();

                step += 1;
            }

            if step == 10 {
                break 'primary;
            }
        }
    }
    for device in device_maps {
        let msg = Message::SetEnabled(false);
        let _ = device.device.handle_message(msg).wait();
    }
}

struct Device {
    device: Arc<Box<dyn BeacnControlDevice>>,
    location: DeviceLocation,

    health: Receiver<()>,
    interactions: Receiver<Interactions>,
}
