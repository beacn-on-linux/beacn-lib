use beacn_lib::controller::ButtonLighting;
use beacn_lib::types::RGBA;
use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, Rgb};

// Clippy gets weirdly pissy about these, they're used inside examples, but it doesn't seem
// to trace them in any meaningful way, so believes they're not used.

/// This test pattern is a simple 4 colour stepper, that demonstrates how to create images that
/// can be used on the devices. It specifically uses overlays, and not full draws.
#[allow(unused)]
pub(crate) fn test_pattern(step: usize) -> (u32, u32, Vec<u8>) {
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

#[allow(unused)]
pub(crate) fn test_buttons(step: usize) -> Vec<(ButtonLighting, RGBA)> {
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
