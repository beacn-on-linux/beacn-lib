use crate::common::{BeacnDeviceHandle, DeviceDefinition, get_device_info};
use crate::controller::ButtonState::{Press, Release};
use crate::controller::ControlThreadSender::{
    KeepAlive, SetActiveBrightness, SetButtonBrightness, SetButtonColour, SetDimTimeout,
    SetEnabled, SetImage,
};
use crate::controller::device::messenger::Messenger;
use crate::controller::{
    BeacnControlDevice, ButtonLighting, Buttons, ControlThreadSender, Dials, Interactions,
};
use crate::types::RGBA;
use crate::version::VersionNumber;
use crate::{BResult, beacn_bail};
use anyhow::Error;
use byteorder::{BigEndian, ByteOrder};
use flume::{Receiver, Sender, bounded};
use jpeg_decoder::Decoder;
use log::{debug, error, warn};
use nusb::MaybeFuture;
use nusb::transfer::{Buffer, In, Interrupt, Out, TransferError};
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;

// Default Display 'Active' and 'Dimmed' brightness, and the default dim time
static DISPLAY_DEFAULT_FULL_BRIGHTNESS: u8 = 40;
static DISPLAY_DIM_BRIGHTNESS: u8 = 1;
static DISPLAY_DIM_TIME: u64 = 180;

// Default button brightness
static BUTTONS_DEFAULT_BRIGHTNESS: u8 = 8;

pub trait BeacnControlDeviceAttach {
    // We're specifically allowing the DeviceDefinition to be a private interface, as it's
    // simply used internally for connection up a device, and shouldn't have any visibility
    // from the outside. This also prevents external code from attempting to call connect.
    #[allow(private_interfaces)]
    fn connect(
        definition: DeviceDefinition,
        interaction: Option<Sender<Interactions>>,
        health_tx: Sender<()>,
    ) -> BResult<Box<dyn BeacnControlDevice>>
    where
        Self: Sized;

    fn get_product_id(&self) -> u16;
    fn get_serial(&self) -> String;
    fn get_version(&self) -> String;

    #[allow(private_interfaces)]
    fn get_sender(&self) -> &Sender<ControlThreadSender>;
    fn get_display_size(&self) -> (u32, u32);
}

// For the most part, the Mix and Mix Create handle interactions identically, obviously the
// mix has fewer buttons, but the firmware seems to do a decent job of handling that, so we
// can simply use the same behaviour between the
pub trait BeacnControlInteraction: BeacnControlDeviceAttach {
    #[allow(private_interfaces)]
    fn spawn_event_handler(
        rx: Receiver<ControlThreadSender>,
        handler: BeacnDeviceHandle,
        interaction: Option<Sender<Interactions>>,
    ) where
        Self: Sized,
    {
        // In 1.2.0 build 81+ the Beacn Mix and Mix Create shifted to a 'polling' method
        // of interaction checks. For versions older we need to use the original notify
        let notify_version = VersionNumber(1, 2, 0, 80);
        let is_notify = handler.version <= notify_version;

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

        if let Err(e) = messenger.ensure_enabled() {
            error!("Failed to Enable Device: {}", e);
            return;
        }

        if let Err(e) = messenger.set_screen_brightness(brightness) {
            error!("Failed to Set Default Brightness: {}", e);
            return;
        }

        if let Err(e) = messenger.set_button_brightness(BUTTONS_DEFAULT_BRIGHTNESS) {
            error!("Failed to Set Default Button Brightness: {}", e);
            return;
        }

        if let Err(e) = messenger.ping() {
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
                                KeepAlive => {
                                    if let Err(e) = messenger.ping() {
                                        error!("Failed to Send Keep-Alive Request: {}", e);
                                        break;
                                    }
                                }
                                SetEnabled(enabled) => {
                                    if let Err(e) = messenger.enable(enabled) {
                                        error!("Failed to Enable Device: {}", e);
                                        break;
                                    }
                                }
                                SetImage(x, y, img) => {
                                    if let Err(e) = messenger.ensure_enabled() {
                                        error!("Failed to Enable Device, dropping Frame: {}", e);
                                        continue 'primary;
                                    }

                                    if let Err(e) = messenger.send_image(x, y, &img) {
                                        error!("Failed to Send Image, dropping Frame: {}", e);
                                        continue 'primary;
                                    }
                                }
                                SetDimTimeout(timeout) => {
                                    dim_duration = timeout;
                                    if !is_dimmed {
                                        // If we're not already dimmed, reset the timer
                                        dim_timeout.reset(timeout);
                                    }
                                }
                                SetActiveBrightness(percent) => {
                                    if is_dimmed {
                                        is_dimmed = false;
                                        dim_timeout.reset(timeout);
                                    }
                                    brightness = percent;
                                    if let Err(e) = messenger.set_screen_brightness(brightness) {
                                        error!("Failed to Set Brightness: {}", e);
                                        break;
                                    }
                                }
                                SetButtonBrightness(value) => {
                                    if let Err(e) = messenger.set_button_brightness(value) {
                                        error!("Failed to Set Button Brightness: {}", e);
                                        break;
                                    }
                                }
                                SetButtonColour(b, c) => {
                                    if let Err(e) = messenger.set_button_colour(b, c) {
                                        error!("Failed to Set Button Colour: {}", e);
                                        break;
                                    }
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
                    if let Err(e) = messenger.set_screen_brightness(DISPLAY_DIM_BRIGHTNESS) {
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

                                    if let Err(e) = messenger.set_screen_brightness(brightness) {
                                        error!("Failed to Set Brightness: {}", e);
                                        break;
                                    }
                                }

                                // Set a new Dim timeout
                                dim_timeout.reset(timeout);
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
                    if let Err(e) = messenger.poll_inputs() {
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

        dim_timeout.cancel();

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

    fn set_enabled(&self, enabled: bool) -> BResult<()> {
        self.get_sender()
            .send(SetEnabled(enabled))
            .map_err(Error::from)?;
        Ok(())
    }

    fn send_keepalive(&self) -> BResult<()> {
        self.get_sender().send(KeepAlive).map_err(Error::from)?;
        Ok(())
    }

    fn set_image(&self, x: u32, y: u32, jpeg_image: &[u8]) -> BResult<()> {
        // TODO: This might be too heavy for a frequent update check (for example, metering)

        // All we do here is validate the image and make sure it fits inside the window
        // Firstly, make sure we're rendering to the actual screen
        let display_size = self.get_display_size();
        if x > display_size.0 || y > display_size.1 {
            beacn_bail!(
                "Position should be between 0..{}, 0..{}",
                display_size.0,
                display_size.1
            );
        }

        // Load out the image, and get the width + height
        let mut decoder = Decoder::new(jpeg_image);
        decoder.read_info().map_err(Error::from)?;

        if let Some(info) = decoder.info() {
            if (x + info.width as u32) > display_size.0 {
                beacn_bail!(
                    "Image overflows display width, {}>{}",
                    x + info.width as u32,
                    display_size.0
                );
            }
            if (y + info.height as u32) > display_size.1 {
                beacn_bail!(
                    "Image overflows display height, {}>{}",
                    y + info.height as u32,
                    display_size.1
                );
            }
        } else {
            beacn_bail!("Unable to Fetch Image Info");
        }

        self.get_sender()
            .send(SetImage(x, y, Vec::from(jpeg_image)))
            .map_err(Error::from)?;
        Ok(())
    }

    fn set_display_brightness(&self, brightness: u8) -> BResult<()> {
        if !(1..=100).contains(&brightness) {
            beacn_bail!("Display Brightness must be a percentage");
        }

        self.get_sender()
            .send(SetActiveBrightness(brightness))
            .map_err(Error::from)?;
        Ok(())
    }

    fn set_button_brightness(&self, brightness: u8) -> BResult<()> {
        if !(0..=10).contains(&brightness) {
            beacn_bail!("Button Brightness must be between 0 and 10");
        }
        self.get_sender()
            .send(SetButtonBrightness(brightness))
            .map_err(Error::from)?;
        Ok(())
    }

    fn set_dim_timeout(&self, timeout: Duration) -> BResult<()> {
        if timeout > Duration::from_secs(300) || timeout < Duration::from_secs(30) {
            beacn_bail!(
                "For display safety, dim timeout must be lower than 5 minutes, and greater than 30 seconds"
            );
        }

        self.get_sender()
            .send(SetDimTimeout(timeout))
            .map_err(Error::from)?;
        Ok(())
    }

    fn set_button_colour(&self, button: ButtonLighting, colour: RGBA) -> BResult<()> {
        let button = button as u8;
        self.get_sender()
            .send(SetButtonColour(button, colour))
            .map_err(Error::from)?;
        Ok(())
    }
}

/// Simple function to Open a USB connection to a Beacn Audio device, do initial setup, and
/// grab the firmware version from the device.
pub(crate) fn open_beacn(def: DeviceDefinition, product_id: &[u16]) -> BResult<BeacnDeviceHandle> {
    if !product_id.contains(&def.descriptor.product_id()) {
        beacn_bail!(
            "Expecting PIDs {:?} but got {}",
            product_id,
            def.descriptor.product_id()
        );
    }

    let device = def.descriptor.open().wait()?;
    let interface = device.claim_interface(0).wait()?;
    interface.set_alt_setting(1).wait()?;

    let mut out_ep = interface.endpoint::<Interrupt, Out>(0x03)?;
    let mut in_ep = interface.endpoint::<Interrupt, In>(0x83)?;
    in_ep.clear_halt().wait()?;

    let setup_timeout = Duration::from_millis(2000);

    // Unlike the Mic and Studio, we use an interrupt, rather a bulk read
    out_ep
        .transfer_blocking([00u8, 00, 00, 1].into(), setup_timeout)
        .into_result()?;
    let completion = in_ep
        .transfer_blocking(Buffer::new(64), setup_timeout)
        .into_result()?;

    let (version, serial) = get_device_info(&completion[..])?;

    debug!(
        "Loaded Device, Location: {}.{}, Serial: {}, Version: {}",
        def.descriptor.bus_id(),
        def.descriptor.device_address(),
        serial.clone(),
        version
    );

    // out_ep / in_ep are dropped here, releasing their claim on the endpoints. They get
    // claimed again in spawn_event_handler, once we know whether the IN endpoint needs to
    // live on a dedicated reader thread (older "notify" firmware) or stay on the event loop
    // thread (newer, polling firmware).
    Ok(BeacnDeviceHandle {
        descriptor: def.descriptor,
        device,
        interface,
        version,
        serial,
    })
}

enum Event {
    Command(Result<ControlThreadSender, flume::RecvError>),
    DimTimeout,
    Input(Result<[u8; 64], flume::RecvError>),
    Poll,
}

pub fn tick(duration: Duration) -> Receiver<()> {
    let (tx, rx) = flume::unbounded();

    thread::spawn(move || {
        loop {
            sleep(duration);
            if tx.send(()).is_err() {
                break;
            }
        }
    });

    rx
}

pub fn never<T>() -> Receiver<T> {
    let (_tx, rx) = flume::bounded(0);
    rx
}

// Replacement for crossbeam::channel::after
pub struct Timer {
    cancel: Sender<()>,
    rx: Receiver<()>,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        let (cancel_tx, cancel_rx) = bounded(1);
        let (tx, rx) = bounded(1);

        thread::spawn(move || {
            loop {
                let event = flume::Selector::new()
                    .recv(&cancel_rx, |_| false)
                    .wait_timeout(duration);

                match event {
                    Ok(false) => break,
                    Ok(true) => {
                        let _ = tx.send(());
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            cancel: cancel_tx,
            rx,
        }
    }

    pub fn cancel(&self) {
        let _ = self.cancel.send(());
    }

    pub fn reset(&mut self, duration: Duration) {
        let _ = self.cancel.send(());
        *self = Self::new(duration);
    }

    pub fn receiver(&self) -> &Receiver<()> {
        &self.rx
    }
}
