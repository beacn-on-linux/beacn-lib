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
use tokio::sync::mpsc;

#[tokio::main(flavor = "current_thread")]
async fn main() {
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
                        let _ = device.device.set_button_colour(button, colour);
                    }

                    let _ = device.device.send_keepalive();
                    let _ = device.device.set_image(x, y, &image);

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
        let _ = device.device.set_enabled(false);
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

/// This test pattern is a simple 4 colour stepper, that demonstrates how to create images that
/// can be used on the devices. It specifically uses overlays, and not full draws.
fn test_pattern(step: usize) -> (u32, u32, Vec<u8>) {
    let width = 800;
    let height = 480;

    let band = width / 4;

    let (x, colour, w) = match step {
        0 => (0, [0u8, 0, 0], width),
        1..=4 => (
            ((step - 1) as u32) * band,
            [[255u8, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]][step - 1],
            band,
        ),
        5..=8 => (((step - 5) as u32) * band, [0u8, 0, 0], band),
        9 => (0, [0u8, 0, 0], width),
        _ => unreachable!(),
    };

    let image = ImageBuffer::from_fn(w, height, |_x, _y| Rgb(colour));

    // The higher the quality, the larger the file size, and thus the longer it'll take to send
    // and render. Keep this in mind when creating your own images!
    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 50);

    encoder.encode_image(&image).unwrap();
    (x, 0, jpeg)
}

fn test_buttons(step: usize) -> Vec<(ButtonLighting, RGBA)> {
    let black = RGBA::from([0, 0, 0, 255]);
    let red = RGBA::from([255, 0, 0, 255]);
    let green = RGBA::from([0, 255, 0, 255]);
    let blue = RGBA::from([0, 0, 255, 255]);
    let white = RGBA::from([255, 255, 255, 255]);

    let clear = vec![
        (ButtonLighting::Dial1, black),
        (ButtonLighting::Dial2, black),
        (ButtonLighting::Dial3, black),
        (ButtonLighting::Dial4, black),
    ];

    match step {
        0 => clear,
        1 => vec![(ButtonLighting::Dial1, red)],
        2 => vec![(ButtonLighting::Dial2, green)],
        3 => vec![(ButtonLighting::Dial3, blue)],
        4 => vec![(ButtonLighting::Dial4, white)],
        5 => vec![(ButtonLighting::Dial1, black)],
        6 => vec![(ButtonLighting::Dial2, black)],
        7 => vec![(ButtonLighting::Dial3, black)],
        8 => vec![(ButtonLighting::Dial4, black)],
        9 => clear,
        _ => unreachable!(),
    }
}
