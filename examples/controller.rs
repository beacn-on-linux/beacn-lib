use beacn_lib::MaybeFuture;
use beacn_lib::controller::{
    BeacnControlDevice, ButtonLighting, Interactions, open_control_device,
};
use beacn_lib::manager::{DeviceLocation, get_beacn_mix_create_device, get_beacn_mix_device};
use beacn_lib::types::RGBA;
use flume::Receiver;
use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, Rgb};
use std::sync::Arc;
use std::time::Duration;
use env_logger::Env;
use crate::common::controller::{test_buttons, test_pattern};

#[path = "common/mod.rs"]
mod common;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
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
                println!("Failed to open device: {:?}", e);
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
        println!("No usable devices found!");
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
                println!("[{}] {:?}", location, msg);
                false
            });

            let location = device.location.clone();
            selector = selector.recv(&device.health, move |_| {
                println!("[{}] Error on Device Handler!", location);
                false
            });
        }
        selector = selector.recv(&tick_rx, |_| true);

        let tick = selector.wait();
        if tick {
            for device in &device_maps {
                let (x, y, image) = test_pattern(step);
                for (button, colour) in test_buttons(step) {
                    let _ = device.device.set_button_colour(button, colour).wait();
                }

                let _ = device.device.send_keepalive().wait();
                let _ = device.device.set_image(x, y, &image).wait();

                step += 1;
            }

            if step == 10 {
                break 'primary;
            }
        }
    }
    for device in device_maps {
        let _ = device.device.set_enabled(false).wait();
    }
}

struct Device {
    device: Arc<Box<dyn BeacnControlDevice>>,
    location: DeviceLocation,

    health: Receiver<()>,
    interactions: Receiver<Interactions>,
}