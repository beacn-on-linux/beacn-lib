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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;

use beacn_lib::flume;
use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, RgbaImage};

use neurodoom::{Button as DoomButton, Buttons as DoomButtons};
use neurodoom::{ClassicEngine, PeerId, PlayerAction};

use beacn_lib::controller::messages::Message;
use image::imageops::{FilterType, resize};
use log::{debug, info, warn};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::flag::register;
use std::time::{Duration, Instant};

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
    let shutdown = install_shutdown_handler();

    let mut input = InputState {
        dial_turn: 0,
        forward: false,
        shoot: false,
        use_key: false,
    };

    let wad = std::fs::read("DOOM1.WAD").expect("missing DOOM1.WAD");
    let mut doom = ClassicEngine::new(&wad, "E1M1").expect("failed to initialise Doom");

    'main: loop {
        if shutdown.load(Ordering::Relaxed) {
            info!("Shutdown signal received, terminating..");
            break;
        }

        info!("Attempting to Connect to Beacn Mix Create Device..");

        let devices = get_beacn_mix_create_device().wait();
        if devices.is_empty() {
            warn!("No BEACN Mix Create found, waiting 5 seconds and trying again..");
            interruptible_sleep(Duration::from_secs(5), &shutdown);
            continue;
        }

        let (interaction_tx, interaction_rx) = flume::unbounded();
        let (health_tx, _health_rx) = flume::unbounded();

        let device = devices[0].clone();
        let device = match open_control_device(device, Some(interaction_tx), health_tx).wait() {
            Ok(device) => device,
            Err(e) => {
                warn!(
                    "Failed to connect to Beacn Mix Create: {e}, waiting 5 seconds and trying again.."
                );
                interruptible_sleep(Duration::from_secs(5), &shutdown);
                continue;
            }
        };

        let mut last_display = Instant::now();

        let mut tick_accumulator = Duration::ZERO;
        let mut last_time = Instant::now();

        loop {
            if shutdown.load(Ordering::Relaxed) {
                info!("Shutdown signal received, terminating..");
                break 'main;
            }

            if _health_rx.try_recv().is_ok() {
                warn!("Device Lost, waiting 5 seconds then trying to get it back..");
                interruptible_sleep(Duration::from_secs(5), &shutdown);
                continue 'main;
            }

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
                        if dial == Dials::Dial4 {
                            input.dial_turn = -(delta as i16 * 120);
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
                let frame = encode_frame(doom.framebuffer());

                let message = Message::KeepAlive;
                match device.handle_message(message).wait() {
                    Ok(()) => {
                        let image = Message::SetImage(0, 0, frame);
                        if let Err(e) = device.handle_message(image).wait() {
                            debug!("Image send failed: {e}");
                            sleep(Duration::from_millis(200));
                        }
                    }
                    Err(e) => {
                        debug!("Keepalive failed: {e}");
                        sleep(Duration::from_millis(200));
                    }
                }

                // We update the timestamp now, so maintain consistency between frames
                last_display = now;
            }

            // Lets not pure CPU spin here :D
            sleep(Duration::from_millis(5));
        }
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

fn install_shutdown_handler() -> Arc<AtomicBool> {
    let shutdown = Arc::new(AtomicBool::new(false));
    register(SIGINT, shutdown.clone()).unwrap();
    register(SIGTERM, shutdown.clone()).unwrap();
    shutdown
}

fn interruptible_sleep(duration: Duration, interrupt: &AtomicBool) {
    let start = Instant::now();

    while start.elapsed() < duration {
        if interrupt.load(Ordering::SeqCst) {
            return;
        }

        // If we're near the end, sleep until we're done
        let remaining = duration - start.elapsed();
        if duration - remaining < Duration::from_millis(50) {
            sleep(remaining);
            return;
        }

        // Wait 50ms, then check again..
        sleep(Duration::from_millis(50));
    }
}
