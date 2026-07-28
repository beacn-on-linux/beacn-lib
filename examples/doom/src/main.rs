//! This is a meme, it works, but is probably not recommended.
//!
//! Controls:
//! AudienceMix - Walk Forward
//! Dial 4 Turn: Turn Left / Right
//! Dial 4 Press: Shoot
//! Dial 4 Button: Use

use beacn_lib::MaybeFuture;
use beacn_lib::controller::{ButtonState, Buttons, Dials, Interactions, open_control_device};
use beacn_lib::manager::get_beacn_mix_create_device;

use beacn_lib::flume;
use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, RgbaImage};

use neurodoom::{Button as DoomButton, Buttons as DoomButtons};
use neurodoom::{ClassicEngine, PeerId, PlayerAction};

use std::time::{Duration, Instant};
use image::imageops::{resize, FilterType};
use log::debug;

const DOOM_TICK: Duration = Duration::from_millis(1000 / 35); // ~35Hz
const DISPLAY_TICK: Duration = Duration::from_millis(1000 / 20); // 20Hz

struct InputState {
    dial_turn: i16,

    forward: bool,
    shoot: bool,

    use_key: bool,
}

impl InputState {
    fn to_player_action(&self) -> PlayerAction {
        let mut buttons = DoomButtons::empty();

        if self.shoot {
            buttons.insert(DoomButton::Attack);
        }

        if self.use_key {
            buttons.insert(DoomButton::Use);
        }

        PlayerAction {
            forward_move: if self.forward { 50 } else { 0 },
            side_move: 0,

            angle_turn: self.dial_turn,

            buttons,
            weapon_select: 0,
        }
    }
}

fn main() {
    env_logger::init();

    let devices = get_beacn_mix_create_device().wait();
    if devices.is_empty() {
        println!("No BEACN Mix Create found");
        return;
    }

    let (interaction_tx, interaction_rx) = flume::unbounded();
    let (health_tx, _health_rx) = flume::unbounded();

    let device = open_control_device(devices[0].clone(), Some(interaction_tx), health_tx)
        .wait()
        .unwrap();

    let mut input = InputState {
        dial_turn: 0,
        forward: false,
        shoot: false,
        use_key: false,
    };

    let wad = std::fs::read("DOOM1.WAD").expect("missing DOOM1.WAD");
    let mut doom = ClassicEngine::new(&wad, "E1M1").expect("failed to initialise Doom");

    let mut last_display = Instant::now();

    let mut tick_accumulator = Duration::ZERO;
    let mut last_time = Instant::now();

    loop {
        let now = Instant::now();

        tick_accumulator += now - last_time;
        last_time = now;

        input.dial_turn = 0;

        // Drain BEACN events
        while let Ok(event) = interaction_rx.try_recv() {
            match event {
                Interactions::ButtonPress(button, state) => {
                    let pressed = matches!(state, ButtonState::Press);

                    match button {
                        Buttons::AudienceMix => {
                            input.forward = pressed;
                        }

                        Buttons::Dial4 => {
                            input.shoot = pressed;
                        }

                        Buttons::Audience4 => {
                            input.use_key = pressed;
                        }

                        _ => {}
                    }
                }

                Interactions::DialChanged(dial, delta) => {
                    match dial {
                        Dials::Dial4 => input.dial_turn = (delta as i16 * 120) * -1,
                        _ => {}
                    }
                }
            }
        }

        while tick_accumulator >= DOOM_TICK {
            let action = input.to_player_action();

            doom.tick_single(PeerId(0), action);
            tick_accumulator -= DOOM_TICK;
        }

        if now.duration_since(last_display) >= DISPLAY_TICK {
            debug!("Rendering Frame..");
            let framebuffer = doom.framebuffer();

            let jpeg = encode_frame(framebuffer);
            let _ = device.send_keepalive();

            if let Err(e) = device.set_image(0, 0, &jpeg) {
                println!("display error: {:?}", e);
            }

            // We update the timestamp now, so maintain consistency between frames
            last_display = now;
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}

fn encode_frame(framebuffer: &[u8]) -> Vec<u8> {
    // neurodoom returns 320x200 RGBA
    let img: RgbaImage = ImageBuffer::from_raw(320, 200, framebuffer.to_vec()).unwrap();

    // Make Beacn Size (800x480)
    let scaled = resize(&img, 800, 480, FilterType::Nearest);

    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 60);

    encoder.encode_image(&scaled).unwrap();
    jpeg
}
