use beacn_lib::MaybeFuture;
use beacn_lib::controller::{BeacnControlDevice, Interactions, open_control_device};
use beacn_lib::manager::{DeviceLocation, get_beacn_mix_create_device, get_beacn_mix_device};
use flume::Receiver;
use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, Rgb};
use std::time::{Duration, Instant};

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

    // We're going to track an instant on our loop and quit after 10 seconds
    let deadline = Instant::now() + Duration::from_secs(10);

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
    while Instant::now() < deadline {
        let mut selector = flume::Selector::new();

        for device in &device_maps {
            let location = device.location.clone();
            selector = selector.recv(&device.interactions, move |msg| {
                println!("[{}] {:?}", location, msg);
            });

            let location = device.location.clone();
            selector = selector.recv(&device.health, move |_| {
                println!("[{}] Error on Device Handler!", location);
            });
        }

        selector = selector.recv(&tick_rx, |_| {
            for device in &device_maps {
                let (x, y, image) = test_pattern(step);

                // The display's screen will switch off after 30 seconds of inactivity, we send
                // keepalives to keep the screen on.
                let _ = device.device.send_keepalive();
                let _ = device.device.set_image(x, y, &image);

                step += 1;
            }
        });

        let remaining = deadline.saturating_duration_since(Instant::now());
        if selector.wait_timeout(remaining).is_err() {
            break;
        }
    }
    for device in device_maps {
        let _ = device.device.set_enabled(false);
    }
}

struct Device {
    device: Box<dyn BeacnControlDevice>,
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
