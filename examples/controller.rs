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

fn main() {
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
