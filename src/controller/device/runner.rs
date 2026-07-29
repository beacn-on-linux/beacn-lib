use crate::MaybeFuture;
use crate::common::BeacnDeviceHandle;
use crate::controller::ButtonState::{Press, Release};
use crate::controller::ControlThreadSender::{
    KeepAlive, SetActiveBrightness, SetButtonBrightness, SetButtonColour, SetDimTimeout,
    SetEnabled, SetImage,
};
use crate::controller::common::{never, tick};
use crate::controller::device::messenger::Messenger;
use crate::controller::device::timer::Timer;
use crate::controller::{BeacnControlDevice, Buttons, ControlThreadSender, Dials, Interactions};
use crate::sealed::Sealed;
use crate::version::VersionNumber;
use byteorder::{BigEndian, ByteOrder};
use flume::{Receiver, Sender, bounded};
use log::{debug, error, warn};
use nusb::transfer::{Buffer, In, Interrupt, Out, TransferError};
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use strum::IntoEnumIterator;

// Default Display 'Active' and 'Dimmed' brightness, and the default dim time
static DISPLAY_DEFAULT_FULL_BRIGHTNESS: u8 = 40;
static DISPLAY_DIM_BRIGHTNESS: u8 = 1;
static DISPLAY_DIM_TIME: u64 = 180;

// Default button brightness
static BUTTONS_DEFAULT_BRIGHTNESS: u8 = 8;

// Internal Event Manager
enum Event {
    Command(anyhow::Result<ControlThreadSender, flume::RecvError>),
    DimTimeout,
    Input(anyhow::Result<[u8; 64], flume::RecvError>),
    Poll,
}

pub(crate) trait BeacnControlDeviceRunner: Sealed {
    fn spawn_event_handler(
        control: Arc<Box<dyn BeacnControlDevice>>,
        rx: Receiver<ControlThreadSender>,
        handler: BeacnDeviceHandle,
        interaction: Option<Sender<Interactions>>,
    ) where
        Self: Sized,
    {
        control.set_sender_enabled(true);

        // In 1.2.0 build 81+ the Beacn Mix and Mix Create shifted to a 'polling' method
        // of interaction checks. For versions older we need to use the original notify
        let notify_version = VersionNumber(1, 2, 0, 80);
        let is_notify = handler.fw_version <= notify_version;

        // We need a message queue for handling when inputs have been received for parsing, given
        // they can come from one of two places, we'll handle them once. 64 might be a little big.
        let (input_tx, input_rx) = bounded(64);
        let mut input_buffer = [0u8; 64];

        // Timeout Handlers
        let timeout = Duration::from_millis(2000);

        // Claim the endpoints we need. The OUT endpoint is always used from this thread.
        // The IN endpoint is used either by a dedicated reader thread (older "notify"
        // firmware) or polled from this thread's event loop (newer firmware) -- never both,
        // so ownership transfers to whichever one needs it below.
        let mut out_ep = match handler.interface.endpoint::<Interrupt, Out>(0x03) {
            Ok(ep) => ep,
            Err(e) => {
                error!("Failed to open Interrupt OUT endpoint: {}", e);
                return;
            }
        };
        let in_ep = match handler.interface.endpoint::<Interrupt, In>(0x83) {
            Ok(ep) => ep,
            Err(e) => {
                error!("Failed to open Interrupt IN endpoint: {}", e);
                return;
            }
        };

        let mut messenger = Messenger::new(&mut out_ep, timeout);
        let mut polled_in_ep: Option<nusb::Endpoint<Interrupt, In>> = None;

        let poll = if is_notify {
            let tx_clone = input_tx.clone();
            thread::spawn(move || {
                debug!("Spawning Event Listener");

                let mut in_ep = in_ep;
                let read = Duration::from_millis(100);

                // These are just defensive checks
                const MAX_NO_DEVICE_RETRIES: u32 = 10;
                let mut no_device_retries = 0;

                loop {
                    // Firstly, we need to fire off a message saying we're ready for buttons
                    match in_ep.transfer_blocking(Buffer::new(64), read).into_result() {
                        Ok(buf) => {
                            no_device_retries = 0;
                            let mut input = [0u8; 64];
                            let n = buf.len().min(64);
                            input[..n].copy_from_slice(&buf[..n]);
                            if let Err(e) = tx_clone.send(input) {
                                // Our channel is gone or closed, bail.
                                warn!("Message Channel Closed, Terminating: {}", e);
                                break;
                            }
                        }
                        Err(TransferError::Disconnected) => {
                            no_device_retries += 1;
                            if no_device_retries > MAX_NO_DEVICE_RETRIES {
                                warn!(
                                    "Device not recovering after {} retries, assuming dead",
                                    MAX_NO_DEVICE_RETRIES
                                );

                                // TODO: We need to actually fully teardown the device
                                // If we get here, then the handle is gone, and that's not been detected
                                // upstream anywhere, which should cause a teardown / reconnect
                                break;
                            }

                            // The assumption here is that when waking from sleep, the interrupt
                            // on the read has been cancelled, and we can safely retry.
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(TransferError::Cancelled) => {
                            // Cancelled here just means our read timed out without anything to
                            // report, which just means the user hasn't moved a dial or pressed
                            // a button in the last `read` seconds, and we're good to wait again.
                            no_device_retries = 0;
                        }
                        Err(usb_error) => {
                            warn!("USB Error while receiving inputs: {}", usb_error);
                            break;
                        }
                    }
                }

                debug!("Event Listener Terminated");
            });
            never()
        } else {
            polled_in_ep = Some(in_ep);
            tick(Duration::from_millis(50))
        };

        // This tracks the button states (so we can message on Send / Receive)
        let mut last_button_state = 0;

        let mut is_dimmed = false;
        let mut brightness = DISPLAY_DEFAULT_FULL_BRIGHTNESS;

        if let Err(e) = messenger.ensure_enabled().wait() {
            error!("Failed to Enable Device: {}", e);
            return;
        }

        if let Err(e) = messenger.set_brightness(brightness).wait() {
            error!("Failed to Set Default Brightness: {}", e);
            return;
        }

        if let Err(e) = messenger
            .set_button_brightness(BUTTONS_DEFAULT_BRIGHTNESS)
            .wait()
        {
            error!("Failed to Set Default Button Brightness: {}", e);
            return;
        }

        if let Err(e) = messenger.ping().wait() {
            error!("Failed to Wake Device: {}", e);
            return;
        }

        sleep(Duration::from_millis(250));

        let mut dim_duration = Duration::from_secs(DISPLAY_DIM_TIME);

        // Create some timers for processing
        let mut dim_timeout = Timer::new(dim_duration);

        // TODO: I should probably use a Macro or a closure to handle the recv
        // In all cases, if a channel has closed, we should abort.
        debug!("Spawning Event Handler for {}", handler.serial);
        'primary: loop {
            let event = flume::Selector::new()
                .recv(&rx, Event::Command)
                .recv(dim_timeout.receiver(), |_| Event::DimTimeout)
                .recv(&input_rx, Event::Input)
                .recv(&poll, |_| Event::Poll)
                .wait();

            match event {
                Event::Command(msg) => {
                    match msg {
                        Ok(msg) => {
                            match msg {
                                ControlThreadSender::Stop => {
                                    debug!("Stopping Event Handler");
                                    break;
                                }
                                KeepAlive(tx) => {
                                    if let Err(e) = messenger.ping().wait() {
                                        error!("Failed to Send Keep-Alive Request: {}", e);
                                        break;
                                    }
                                    let _ = tx.send(());
                                }
                                SetEnabled(enabled, tx) => {
                                    if let Err(e) = messenger.enable(enabled).wait() {
                                        error!("Failed to Enable Device: {}", e);
                                        break;
                                    }
                                    let _ = tx.send(());
                                }
                                SetImage(x, y, img, tx) => {
                                    if let Err(e) = messenger.ensure_enabled().wait() {
                                        error!("Failed to Enable Device, dropping Frame: {}", e);
                                        continue 'primary;
                                    }

                                    if let Err(e) = messenger.send_image(x, y, &img).wait() {
                                        error!("Failed to Send Image, dropping Frame: {}", e);
                                        continue 'primary;
                                    }
                                    let _ = tx.send(());
                                }
                                SetDimTimeout(timeout, tx) => {
                                    dim_duration = timeout;
                                    if !is_dimmed {
                                        // If we're not already dimmed, reset the timer
                                        dim_timeout.reset(dim_duration);
                                    }
                                    let _ = tx.send(());
                                }
                                SetActiveBrightness(percent, tx) => {
                                    if is_dimmed {
                                        is_dimmed = false;
                                        dim_timeout.reset(dim_duration);
                                    }
                                    brightness = percent;
                                    if let Err(e) = messenger.set_brightness(brightness).wait() {
                                        error!("Failed to Set Brightness: {}", e);
                                        break;
                                    }
                                    let _ = tx.send(());
                                }
                                SetButtonBrightness(value, tx) => {
                                    if let Err(e) = messenger.set_button_brightness(value).wait() {
                                        error!("Failed to Set Button Brightness: {}", e);
                                        break;
                                    }
                                    let _ = tx.send(());
                                }
                                SetButtonColour(b, c, tx) => {
                                    if let Err(e) = messenger.set_button_colour(b, c).wait() {
                                        error!("Failed to Set Button Colour: {}", e);
                                        break;
                                    }
                                    let _ = tx.send(());
                                }
                            }
                        }
                        Err(e) => {
                            error!("Main Event Receiver Error: {}", e);
                            break;
                        }
                    }
                }
                Event::DimTimeout => {
                    is_dimmed = true;
                    if let Err(e) = messenger.set_brightness(DISPLAY_DIM_BRIGHTNESS).wait() {
                        error!("Failed to Set DIM brightness: {}", e);
                        break;
                    }
                }
                Event::Input(msg) => {
                    match msg {
                        Ok(input) => {
                            let (changed, button_state) =
                                Self::handle_interaction(input, last_button_state, &interaction);
                            last_button_state = button_state;

                            if changed {
                                if is_dimmed {
                                    // We need to wake up screen
                                    is_dimmed = false;

                                    if let Err(e) = messenger.set_brightness(brightness).wait() {
                                        error!("Failed to Set Brightness: {}", e);
                                        break;
                                    }
                                }

                                // Set a new Dim timeout
                                dim_timeout.reset(dim_duration);
                            }
                        }
                        Err(e) => {
                            error!("Input Receiver Terminated: {:?}", e);
                            break;
                        }
                    }
                }
                Event::Poll => {
                    // Ok, we're at a poll interval, we need to fetch changes to inputs
                    if let Err(e) = messenger.poll_inputs().wait() {
                        error!("Failed to Poll Inputs: {}", e);
                        break;
                    }

                    let in_ep = polled_in_ep
                        .as_mut()
                        .expect("polled_in_ep is always Some() when Event::Poll can fire");
                    match in_ep
                        .transfer_blocking(Buffer::new(64), timeout)
                        .into_result()
                    {
                        Err(e) => {
                            debug!("Error Reading Poll Response: {}", e);
                            break;
                        }
                        Ok(buf) => {
                            let n = buf.len().min(64);
                            input_buffer[..n].copy_from_slice(&buf[..n]);
                            if let Err(e) = input_tx.send(input_buffer) {
                                debug!("Failed to Send Poll Response Data: {}", e);
                                break;
                            };
                        }
                    }
                }
            }
        }

        // Before we exit, we should drain the remaining receiver queue and make sure all
        // senders it contains are also dropped. This prevents code locking up between end
        // and health send.
        control.set_sender_enabled(false);
        while let Ok(msg) = rx.try_recv() {
            drop(msg);
        }

        debug!("Event Handler Terminated");
    }

    fn handle_interaction(
        message: [u8; 64],
        last: u16,
        tx: &Option<Sender<Interactions>>,
    ) -> (bool, u16)
    where
        Self: Sized,
    {
        let mut has_interacted = false;

        let dials = &message[4..8];
        for dial in Dials::iter() {
            if dials[dial as usize] != 0 {
                let change = dials[dial as usize] as i8;
                if let Some(tx) = tx {
                    let _ = tx.send(Interactions::DialChanged(dial, change));
                }
                debug!("Dial Moved: {} - {}", dial, change);
                has_interacted = true;
            }
        }

        let buttons = BigEndian::read_u16(&message[8..10]);
        for button in Buttons::iter() {
            let button_pressed = (buttons >> button as u8) & 1;
            if ((last >> button as u8) & 1) != button_pressed {
                if (buttons >> button as u8) & 1 == 1 {
                    if let Some(tx) = tx {
                        let _ = tx.send(Interactions::ButtonPress(button, Press));
                    }
                    debug!("Button Pressed: {}", button);
                    has_interacted = true;
                } else {
                    if let Some(tx) = tx {
                        let _ = tx.send(Interactions::ButtonPress(button, Release));
                    }
                    debug!("Button Released: {}", button);
                    has_interacted = true;
                }
            }
        }
        (has_interacted, buttons)
    }
}
