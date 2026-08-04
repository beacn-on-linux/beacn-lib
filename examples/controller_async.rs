use crate::common::controller::{test_buttons, test_pattern};
use beacn_lib::controller::{BeacnControlDevice, Interactions, open_control_device};
use beacn_lib::manager::{DeviceLocation, get_beacn_mix_create_device, get_beacn_mix_device};

use crate::common::logging::configure_logging;
use crate::common::{interval, spawn_local};
use flume::Receiver;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::mpsc;
use web_time::Duration;

#[path = "common/mod.rs"]
mod common;

beacn_main!(flavor = "local", {
    app_main().await;
});

async fn app_main() {
    configure_logging();

    // Firstly, find any Mix and Mix Create devices
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
                error!("Failed to open device: {:?}", e);
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
        error!("No usable devices found!");
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

        spawn_local(async move {
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

    let mut ticker = interval::Interval::new(Duration::from_secs(1));
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
                        info!("[{}] {:?}", location, msg);
                    }

                    DeviceEvent::Health(location) => {
                        error!("[{}] Error on Device Handler!", location);
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
