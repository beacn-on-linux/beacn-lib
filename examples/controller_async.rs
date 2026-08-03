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
use tokio::sync::mpsc;
use crate::common::controller::{test_buttons, test_pattern};

#[path = "common/mod.rs"]
mod common;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    let mut devices = get_beacn_mix_device().await;
    devices.extend(get_beacn_mix_create_device().await);

    let mut device_maps = vec![];

    for device in devices {
        let (interaction_tx, interaction_rx) = flume::unbounded();
        let (health_tx, health_rx) = flume::unbounded();

        let dev = open_control_device(device.clone(), Some(interaction_tx), health_tx).await;
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
        });
    }

    if device_maps.is_empty() {
        println!("No usable devices found!");
        return;
    }

    // For each of the devices, spawn up a task that handles the events so we can wrap everything
    // in a tokio::select!
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    for device in &device_maps {
        let tx = event_tx.clone();
        let location = device.location.clone();

        let interaction_rx = device.interactions.clone();
        let health_rx = device.health.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = interaction_rx.recv_async() => {
                        match msg {
                            Ok(msg) => {
                                let _ = tx.send(DeviceEvent::Interaction(location.clone(), msg));
                            }
                            Err(_) => break,
                        }
                    }

                    health = health_rx.recv_async() => {
                        match health {
                            Ok(_) => {
                                let _ = tx.send(DeviceEvent::Health(location.clone()));
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        });
    }
    drop(event_tx);

    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    let mut step = 0;

    'primary: loop {
        tokio::select! {
            _ = ticker.tick() => {
                for device in &device_maps {
                    let (x, y, image) = test_pattern(step);
                    for (button, colour) in test_buttons(step) {
                        let _ = device.device.set_button_colour(button, colour).await;
                    }

                    let _ = device.device.send_keepalive().await;
                    let _ = device.device.set_image(x, y, &image).await;

                    step += 1;

                    if step == 10 {
                        break 'primary;
                    }
                }
            }

            Some(event) = event_rx.recv() => {
                match event {
                    DeviceEvent::Interaction(location, msg) => {
                        println!("[{}] {:?}", location, msg);
                    }

                    DeviceEvent::Health(location) => {
                        println!("[{}] Error on Device Handler!", location);
                    }
                }
            }
        }
    }

    for device in device_maps {
        let _ = device.device.set_enabled(false).await;
    }
}

#[derive(Debug)]
enum DeviceEvent {
    Interaction(DeviceLocation, Interactions),
    Health(DeviceLocation),
}

struct Device {
    device: Arc<Box<dyn BeacnControlDevice>>,
    location: DeviceLocation,

    health: Receiver<()>,
    interactions: Receiver<Interactions>,
}