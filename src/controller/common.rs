use crate::common::{BeacnDeviceHandle, DeviceDefinition, get_device_info};
use crate::controller::ButtonState::{Press, Release};
use crate::controller::ControlThreadSender::{
    KeepAlive, SetActiveBrightness, SetButtonBrightness, SetButtonColour, SetDimTimeout,
    SetEnabled, SetImage,
};
use crate::controller::{
    BeacnControlDevice, ButtonLighting, Buttons, ControlThreadSender, Dials, Interactions,
};
use crate::types::RGBA;
use crate::version::VersionNumber;
use crate::{BResult, beacn_bail};
use anyhow::Error;
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use flume::{Receiver, Sender, bounded};
use jpeg_decoder::Decoder;
use log::{debug, error, warn};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use strum::IntoEnumIterator;

// Default Display 'Active' and 'Dimmed' brightness, and the default dim time
static DISPLAY_DEFAULT_FULL_BRIGHTNESS: u8 = 40;
static DISPLAY_DEFAULT_DIM_BRIGHTNESS: u8 = 1;
static DISPLAY_DEFAULT_DIM_TIME: u64 = 180;

// Default button brightness
static BUTTONS_DEFAULT_BRIGHTNESS: u8 = 8;

// Trivial enum — closures just tag the value, all logic stays outside
enum Selected {
    Control(Result<ControlThreadSender, flume::RecvError>),
    DimTimeout(Result<(), flume::RecvError>),
    Input(Result<[u8; 64], flume::RecvError>),
    Poll(Result<(), flume::RecvError>),
}

pub trait BeacnControlDeviceAttach {
    // We're specifically allowing the DeviceDefinition to be a private interface, as it's
    // simply used internally for connection up a device, and shouldn't have any visibility
    // from the outside. This also prevents external code from attempting to call connect.
    #[allow(private_interfaces)]
    fn connect(
        definition: DeviceDefinition,
        interaction: Option<Sender<Interactions>>,
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
        let mut input = [0; 64];

        // Timeout Handlers
        let timeout = Duration::from_millis(2000);

        // At this point, we need to pull out the USB handler and wrap it up
        let handle = Arc::new(handler.handle);
        let poll: Receiver<()> = if is_notify {
            let handler_clone = handle.clone();
            let tx_clone = input_tx.clone();
            thread::spawn(move || {
                debug!("Spawning Event Listener");

                // Input buffer for messages
                let mut input = [0; 64];

                let handle = handler_clone;
                let input_tx = tx_clone;
                let read = Duration::from_millis(100);

                // These are just defensive checks
                const MAX_NO_DEVICE_RETRIES: u32 = 10;
                let mut no_device_retries = 0;

                loop {
                    // Firstly, we need to fire off a message saying we're ready for buttons
                    match handle.read_interrupt(0x83, &mut input, read) {
                        Ok(_) => {
                            no_device_retries = 0;
                            if input_tx.send(input).is_err() {
                                // Our channel is gone or closed, bail.
                                warn!("Message Channel Closed, Terminating");
                                break;
                            }
                        }
                        Err(rusb::Error::NoDevice) => {
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
                        Err(rusb::Error::Timeout) => {
                            // Timeout is a completely acceptable error to have, it just means
                            // the user hasn't moved a dial or pressed a button in the last
                            // `read` seconds, and we're good to wait again.
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
            tick(Duration::from_millis(50))
        };

        // This tracks the button states (so we can message on Send / Receive)
        let mut last_button_state = 0;

        let mut is_dimmed = false;
        let mut active_brightness = DISPLAY_DEFAULT_FULL_BRIGHTNESS;
        let mut button_brightness = BUTTONS_DEFAULT_BRIGHTNESS;

        let enable = [0, 1, 0, 4, 0, 0, 0, 0];
        let brightness = [0, 0, 0, 4, active_brightness, 0, 0, 0];
        let buttons = [1, 7, 0, 4, button_brightness, 0, 0, 0];

        // Message to instruct the screen to turn on (default to off after a few seconds)
        if handle.write_interrupt(0x03, &enable, timeout).is_err() {
            error!("Unable to Turn the Screen on");
            return;
        }

        // Set the default display brightness
        if handle.write_interrupt(0x03, &brightness, timeout).is_err() {
            error!("Failed to Set Default Brightness");
            return;
        }

        // Set the default button brightness
        if handle.write_interrupt(0x03, &buttons, timeout).is_err() {
            error!("Unable to Set Default Button Brightness");
            return;
        }

        // Force the device into a 'wake' state if it's currently sleeping
        let wake = [00, 00, 00, 0xf1];
        if handle.write_interrupt(0x03, &wake, timeout).is_err() {
            error!("Unable to Wake Device");
            return;
        }

        let mut dim_duration = Duration::from_secs(DISPLAY_DEFAULT_DIM_TIME);

        // Create some timers for processing
        let mut dim_timeout = after(dim_duration);

        // TODO: I should probably use a Macro or a closure to handle the recv
        // In all cases, if a channel has closed, we should abort.
        debug!("Spawning Event Handler for {}", handler.serial);
        'primary: loop {
            let selected = flume::Selector::new()
                .recv(&rx, Selected::Control)
                .recv(&dim_timeout, Selected::DimTimeout)
                .recv(&input_rx, Selected::Input)
                .recv(&poll, Selected::Poll)
                .wait();

            match selected {
                Selected::Control(msg) => match msg {
                    Ok(msg) => match msg {
                        ControlThreadSender::Stop => {
                            debug!("Stopping Event Handler");
                            break 'primary;
                        }
                        KeepAlive => {
                            let msg = &[00, 00, 00, 0xf1];
                            if handle.write_interrupt(0x03, msg, timeout).is_err() {
                                error!("Error Sending Keep-Alive Request");
                                break;
                            }
                        }
                        SetEnabled(enabled) => {
                            let byte = if enabled { 0 } else { 1 };
                            let msg = &[0, 1, 0, 4, byte, 0, 0, 0];

                            if handle.write_interrupt(0x03, msg, timeout).is_err() {
                                error!("Failed to Send Enabled Message");
                                break 'primary;
                            }
                        }
                        SetImage(x, y, img) => {
                            let max_attempts = 100;
                            let img_time = Duration::from_millis(100);

                            for attempt in 0..=max_attempts {
                                let att = attempt + 1;
                                let mut success = true;
                                let mut iter = img.chunks(1020).enumerate().peekable();
                                let mut out = [0; 1024];

                                if handle.write_interrupt(0x03, &enable, img_time).is_err() {
                                    warn!("Reset Failed during attempt {}", attempt + 1);
                                    continue;
                                }

                                let count = iter.clone().count();

                                debug!("Drawing {count} chunks (attempt {att})",);
                                while let Some((index, value)) = iter.next() {
                                    LittleEndian::write_u24(&mut out[0..3], index as u32);
                                    out[3] = 0x50;

                                    // Write this chunk to the USB stream
                                    out[4..value.len() + 4].copy_from_slice(value);
                                    if handle.write_interrupt(0x03, &out, img_time).is_err() {
                                        warn!("Failed to Send Chunk, attempt {}", attempt + 1);
                                        success = false;
                                        break;
                                    }

                                    // Check if we're the last packet...
                                    if iter.peek().is_none() {
                                        // Flag the message as complete
                                        out[0] = 0xff;
                                        out[1] = 0xff;
                                        out[2] = 0xff;
                                        out[3] = 0x50;

                                        // Send the Total size of the image
                                        let len = img.len() as u32 - 1;
                                        LittleEndian::write_u32(&mut out[4..8], len);

                                        // Set the X and Y coordinates..
                                        LittleEndian::write_u32(&mut out[8..12], x);
                                        LittleEndian::write_u32(&mut out[12..16], y);

                                        // Send this out via USB.
                                        if handle.write_interrupt(0x03, &out, img_time).is_err() {
                                            error!("Failed to Send Final Chunk, attempt {att}",);
                                            success = false;
                                            break;
                                        }
                                    }
                                }

                                if success {
                                    break;
                                } else if attempt == max_attempts {
                                    error!("Failed to send image after {} retries", max_attempts);
                                    break 'primary;
                                }
                            }
                        }
                        SetDimTimeout(new_timeout) => {
                            dim_duration = new_timeout;
                            if !is_dimmed {
                                // If we're not already dimmed, reset the timer
                                dim_timeout = after(new_timeout);
                            }
                        }
                        SetActiveBrightness(percent) => {
                            if is_dimmed {
                                is_dimmed = false;
                                dim_timeout = after(dim_duration);
                            }
                            active_brightness = percent;
                            let msg = &[0, 0, 0, 4, active_brightness, 0, 0, 0];
                            if handle.write_interrupt(0x03, msg, timeout).is_err() {
                                error!("Failed to Set Brightness");
                                break 'primary;
                            }
                        }
                        SetButtonBrightness(value) => {
                            button_brightness = value;
                            let msg = &[1, 7, 0, 4, button_brightness, 0, 0, 0];
                            if handle.write_interrupt(0x03, msg, timeout).is_err() {
                                error!("Failed to Set Button Brightness");
                                break 'primary;
                            }
                        }
                        SetButtonColour(button, colour) => {
                            let message = [
                                1,
                                button,
                                0,
                                4,
                                colour.blue,
                                colour.green,
                                colour.red,
                                colour.alpha,
                            ];
                            if handle.write_interrupt(0x03, &message, timeout).is_err() {
                                error!("Failed to Set Button Colour");
                                break 'primary;
                            }
                        }
                    },
                    Err(e) => {
                        error!("Main Event Receiver Error: {}", e);
                        break 'primary;
                    }
                },
                Selected::DimTimeout(msg) => match msg {
                    Ok(_) => {
                        is_dimmed = true;
                        let msg = &[0, 0, 0, 4, DISPLAY_DEFAULT_DIM_BRIGHTNESS, 0, 0, 0];
                        if handle.write_interrupt(0x03, msg, timeout).is_err() {
                            error!("Failed to Set DIM brightness");
                            break 'primary;
                        }
                    }
                    Err(e) => {
                        error!("DIM Timeout Receiver broken {}", e);
                        break 'primary;
                    }
                },
                Selected::Input(msg) => match msg {
                    Ok(input) => {
                        let (changed, button_state) =
                            Self::handle_interaction(input, last_button_state, &interaction);
                        last_button_state = button_state;
                        if changed {
                            if is_dimmed {
                                // We need to wake up screen
                                is_dimmed = false;
                                let msg = &[0, 0, 0, 4, active_brightness, 0, 0, 0];
                                if handle.write_interrupt(0x03, msg, timeout).is_err() {
                                    error!("Failed to Set DIM brightness");
                                    break 'primary;
                                }
                            }
                            dim_timeout = after(dim_duration);
                        }
                    }
                    Err(e) => {
                        error!("Input Receiver Terminated: {:?}", e);
                        break 'primary;
                    }
                },
                Selected::Poll(msg) => match msg {
                    // Ok, we're at a poll interval, we need to fetch changes to inputs
                    Ok(_) => {
                        let msg = &[0, 0, 0, 5];
                        if handle.write_interrupt(0x03, msg, timeout).is_err() {
                            debug!("Error Sending Poll Request");
                            break 'primary;
                        }
                        if handle.read_interrupt(0x83, &mut input, timeout).is_ok() {
                            if input_tx.send(input).is_err() {
                                debug!("Failed to Send Poll Response Data");
                                break 'primary;
                            }
                        } else {
                            debug!("Error Reading Poll Response");
                            break 'primary;
                        }
                    }
                    Err(e) => {
                        error!("Poll Receiver Terminated: {:?}", e);
                        break 'primary;
                    }
                },
            }
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
                    x + info.height as u32,
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

/// Simple function to Open a libusb connection to a Beacn Audio device, do initial setup and
/// grab the firmware version from the device.
pub(crate) fn open_beacn(def: DeviceDefinition, product_id: &[u16]) -> BResult<BeacnDeviceHandle> {
    if !product_id.contains(&def.descriptor.product_id()) {
        beacn_bail!(
            "Expecting PIDs {:?} but got {}",
            product_id,
            def.descriptor.product_id()
        );
    }

    let handle = def.device.open()?;
    handle.claim_interface(0)?;
    handle.set_alternate_setting(0, 1)?;
    handle.clear_halt(0x83)?;

    let setup_timeout = Duration::from_millis(2000);

    // Unlike the Mic and Studio, we use an interrupt, rather a bulk read
    let mut input = [0; 64];
    handle.write_interrupt(0x03, &[00, 00, 00, 1], setup_timeout)?;
    handle.read_interrupt(0x83, &mut input, setup_timeout)?;

    let (version, serial) = get_device_info(&input)?;

    debug!(
        "Loaded Device, Location: {}.{}, Serial: {}, Version: {}",
        def.device.bus_number(),
        def.device.address(),
        serial.clone(),
        version
    );

    Ok(BeacnDeviceHandle {
        descriptor: def.descriptor,
        device: def.device,
        handle,
        version,
        serial,
    })
}

fn never<T: Send + 'static>() -> Receiver<T> {
    let (tx, rx) = flume::bounded(0);
    // Forget the sender so the channel stays connected but never receives
    std::mem::forget(tx);
    rx
}

fn tick(duration: Duration) -> Receiver<()> {
    let (tx, rx) = flume::unbounded();
    thread::spawn(move || {
        loop {
            thread::sleep(duration);
            if tx.send(()).is_err() {
                break;
            }
        }
    });
    rx
}

fn after(duration: Duration) -> Receiver<()> {
    let (tx, rx) = flume::bounded(1);
    thread::spawn(move || {
        thread::sleep(duration);
        let _ = tx.send(());
    });
    rx
}
