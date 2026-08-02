use crate::common::BeacnDeviceHandle;
use crate::controller::ButtonState::{Press, Release};
use crate::controller::ControlThreadSender::{
    KeepAlive, SetActiveBrightness, SetButtonBrightness, SetButtonColour, SetDimTimeout,
    SetEnabled, SetImage,
};
use crate::controller::device::messenger::Messenger;
use crate::controller::{BeacnControlDevice, Buttons, ControlThreadSender, Dials, Interactions};
use crate::sealed::Sealed;
use crate::timers::{Ticker, Timer, sleep};
use crate::transfer::transfer;
use crate::version::VersionNumber;
use byteorder::{BigEndian, ByteOrder};
use flume::{Receiver, Sender};
use futures_lite::future::or;
use futures_lite::stream::{self, Stream, StreamExt};
use log::{debug, error, warn};
use nusb::transfer::{Buffer, In, Interrupt, TransferError};
use std::future::pending;
use std::pin::Pin;
use std::sync::Arc;
use strum::IntoEnumIterator;
use web_time::Duration;

// Default Display 'Active' and 'Dimmed' brightness, and the default dim time
static DISPLAY_DEFAULT_FULL_BRIGHTNESS: u8 = 40;
static DISPLAY_DIM_BRIGHTNESS: u8 = 1;
static DISPLAY_DIM_TIME: u64 = 180;

// Default button brightness
static BUTTONS_DEFAULT_BRIGHTNESS: u8 = 8;

// Internal Event Manager
enum Event {
    Command(Result<ControlThreadSender, flume::RecvError>),
    DimTimeout,
    Input([u8; 64]),
    InputEnded,
    Poll,
    ProcessInputs([u8; 64]),
}

pub(crate) trait BeacnControlDeviceRunner: Sealed {
    async fn spawn_event_handler(
        control: Arc<Box<dyn BeacnControlDevice>>,
        rx: Receiver<ControlThreadSender>,
        handler: BeacnDeviceHandle,
        interaction: Option<Sender<Interactions>>,
    ) where
        Self: Sized,
    {
        control.set_sender_enabled(true);

        // In 1.2.0 build 81+ the Beacn Mix and Mix Create shifted to a 'polling' method
        // of interaction checks. For versions older we need to listen for a notification
        let notify_version = VersionNumber(1, 2, 0, 80);
        let is_notify = handler.fw_version <= notify_version;

        // Timeout Handlers
        let timeout = Duration::from_millis(2000);

        // This is an internal event loop, it's designed to allow Events to trigger other Events
        // when needed (for example, the two different interaction methods can feed into a single
        // handler with the same data).
        let (event_tx, event_rx) = flume::unbounded::<Event>();
        let in_ep = match handler.interface.endpoint::<Interrupt, In>(0x83) {
            Ok(ep) => ep,
            Err(e) => {
                error!("Failed to open Interrupt IN endpoint: {}", e);
                return;
            }
        };

        let serial = handler.serial.clone();
        let mut messenger = match Messenger::new(handler, timeout) {
            Ok(messenger) => messenger,
            Err(e) => {
                error!("Failed to create Messenger: {}", e);
                return;
            }
        };

        #[cfg(target_arch = "wasm32")]
        type NotifyType = Pin<Box<dyn Stream<Item = [u8; 64]>>>;

        #[cfg(not(target_arch = "wasm32"))]
        type NotifyType = Pin<Box<dyn Stream<Item = [u8; 64]> + Send>>;

        let mut polled_in_ep: Option<nusb::Endpoint<Interrupt, In>> = None;
        let mut notify_reads: NotifyType = if is_notify {
            Box::pin(build_notify_read_stream(in_ep))
        } else {
            polled_in_ep = Some(in_ep);
            Box::pin(stream::pending())
        };

        let mut poll_tick = match is_notify {
            true => PollTick::Disabled,
            false => PollTick::Interval(Ticker::new(Duration::from_millis(50), false)),
        };

        // This tracks the button states (so we can message on Send / Receive)
        let mut old_state = 0;

        let mut is_dimmed = false;
        let mut brightness = DISPLAY_DEFAULT_FULL_BRIGHTNESS;

        if let Err(e) = messenger.ensure_enabled().await {
            error!("Failed to Enable Device: {}", e);
            return;
        }

        if let Err(e) = messenger.set_brightness(brightness).await {
            error!("Failed to Set Default Brightness: {}", e);
            return;
        }

        if let Err(e) = messenger
            .set_button_brightness(BUTTONS_DEFAULT_BRIGHTNESS)
            .await
        {
            error!("Failed to Set Default Button Brightness: {}", e);
            return;
        }

        if let Err(e) = messenger.ping().await {
            error!("Failed to Wake Device: {}", e);
            return;
        }

        sleep(Duration::from_millis(250)).await;

        let mut dim_duration = Duration::from_secs(DISPLAY_DIM_TIME);

        // Create some timers for processing
        let mut dim_timeout = Timer::new(dim_duration);

        // TODO: I should probably use a Macro or a closure to handle the recv
        // In all cases, if a channel has closed, we should abort.
        debug!("Spawning Event Handler for {}", serial);

        'primary: loop {
            // We're using this because we have futures-lite available as an agnostic handler,
            // it's not pretty, but should get the job done. Wait for one of our tasks to fire.
            let event = or(
                or(async { Event::Command(rx.recv_async().await) }, async {
                    dim_timeout.wait().await;
                    Event::DimTimeout
                }),
                or(
                    async {
                        match notify_reads.next().await {
                            Some(input) => Event::Input(input),
                            None => Event::InputEnded,
                        }
                    },
                    or(async { event_rx.recv_async().await.unwrap() }, async {
                        poll_tick.wait().await;
                        Event::Poll
                    }),
                ),
            )
            .await;

            // Now handle the fired task.
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
                                    if let Err(e) = messenger.ping().await {
                                        error!("Failed to Send Keep-Alive Request: {}", e);
                                        break;
                                    }
                                    let _ = tx.send(());
                                }
                                SetEnabled(enabled, tx) => {
                                    if let Err(e) = messenger.enable(enabled).await {
                                        error!("Failed to Enable Device: {}", e);
                                        break;
                                    }
                                    let _ = tx.send(());
                                }
                                SetImage(x, y, img, tx) => {
                                    if let Err(e) = messenger.ensure_enabled().await {
                                        error!("Failed to Enable Device, dropping Frame: {}", e);
                                        continue 'primary;
                                    }

                                    if let Err(e) = messenger.send_image(x, y, &img).await {
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
                                    if let Err(e) = messenger.set_brightness(brightness).await {
                                        error!("Failed to Set Brightness: {}", e);
                                        break;
                                    }
                                    let _ = tx.send(());
                                }
                                SetButtonBrightness(value, tx) => {
                                    if let Err(e) = messenger.set_button_brightness(value).await {
                                        error!("Failed to Set Button Brightness: {}", e);
                                        break;
                                    }
                                    let _ = tx.send(());
                                }
                                SetButtonColour(b, c, tx) => {
                                    if let Err(e) = messenger.set_button_colour(b, c).await {
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
                    if let Err(e) = messenger.set_brightness(DISPLAY_DIM_BRIGHTNESS).await {
                        error!("Failed to Set DIM brightness: {}", e);
                        break;
                    }
                }
                Event::Input(input) => {
                    let _ = event_tx.send(Event::ProcessInputs(input));
                }
                Event::InputEnded => {
                    error!("Input Receiver Terminated");
                    break;
                }
                Event::Poll => {
                    // Ok, we're at a poll interval, we need to fetch changes to inputs
                    if let Err(e) = messenger.poll_inputs().await {
                        error!("Failed to Poll Inputs: {}", e);
                        break;
                    }

                    let Some(in_ep) = polled_in_ep.as_mut() else {
                        error!("polled_in_ep is None when Event::Poll can fire");
                        break;
                    };

                    match transfer(in_ep, Buffer::new(64), timeout).await {
                        Err(e) => {
                            debug!("Error Reading Poll Response: {}", e);
                            break;
                        }

                        Ok(buf) => {
                            let mut input = [0u8; 64];
                            let n = buf.len().min(64);
                            input[..n].copy_from_slice(&buf[..n]);

                            // Fire off to the event queue
                            let _ = event_tx.send(Event::ProcessInputs(input));
                        }
                    }
                }
                Event::ProcessInputs(input) => {
                    let (changed, state) = Self::on_interaction(input, old_state, &interaction);
                    old_state = state;

                    if !changed {
                        continue;
                    }

                    if is_dimmed {
                        is_dimmed = false;
                        if let Err(e) = messenger.set_brightness(brightness).await {
                            error!("Failed to Set Brightness: {}", e);
                            break;
                        }
                    }
                    dim_timeout.reset(dim_duration);
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

    fn on_interaction(
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

/// Builds an async-friendly background read for the "notify" firmware path, we pass back a stream
/// which can be awaited and will handle message reading, and will trigger when a message arrives.
///
/// We maintain ownership of the endpoint for the run, but when a message arrives we return it
/// back to the caller.
fn build_notify_read_stream(in_ep: nusb::Endpoint<Interrupt, In>) -> impl Stream<Item = [u8; 64]> {
    // Defensive check, how many consecutive "device gone" reads we'll tolerate before assuming
    // the device is actually dead.
    const MAX_DEVICE_RETRIES: u32 = 10;

    let read_timeout = Duration::from_millis(100);
    stream::unfold(
        (in_ep, 0u32),
        move |(mut ep, mut device_retries)| async move {
            loop {
                match transfer(&mut ep, Buffer::new(64), read_timeout).await {
                    Ok(buf) => {
                        device_retries = 0;
                        let mut input = [0u8; 64];
                        let n = buf.len().min(64);
                        input[..n].copy_from_slice(&buf[..n]);
                        return Some((input, (ep, device_retries)));
                    }
                    Err(TransferError::Cancelled) => {
                        // Just a read timeout with nothing to report -- the user hasn't moved a
                        // dial or pressed a button recently. Loop and wait again.
                        device_retries = 0;
                    }
                    Err(TransferError::Disconnected) => {
                        device_retries += 1;
                        if device_retries > MAX_DEVICE_RETRIES {
                            warn!(
                                "Device not recovering after {} retries, assuming dead",
                                MAX_DEVICE_RETRIES
                            );

                            return None;
                        }

                        // The assumption here is that when waking from sleep, the interrupt on
                        // the read has been cancelled, and we can safely retry.
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(usb_error) => {
                        warn!("USB Error while receiving inputs: {}", usb_error);
                        return None;
                    }
                }
            }
        },
    )
}

// Handles the firmware behaviour paths, newer firmwares need polling, older ones don't, so this
// enum lets us cleanly define both cases.
pub enum PollTick {
    Disabled,
    Interval(Ticker),
}

impl PollTick {
    pub async fn wait(&mut self) {
        match self {
            PollTick::Disabled => pending().await,
            PollTick::Interval(ticker) => ticker.tick().await,
        }
    }
}
