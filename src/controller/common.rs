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
use anyhow::Result;
use anyhow::bail;
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use crossbeam::channel::{Receiver, Sender, after, bounded, never, tick};
use crossbeam::select;
use jpeg_decoder::Decoder;
use log::{debug, error, warn};
use rusb::Error::Timeout;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;
use strum::IntoEnumIterator;

// Default Display 'Active' and 'Dimmed' brightness, and the default dim time
static DISPLAY_DEFAULT_FULL_BRIGHTNESS: u8 = 40;
static DISPLAY_DEFAULT_DIM_BRIGHTNESS: u8 = 1;
static DISPLAY_DEFAULT_DIM_TIME: u64 = 180;

// Default button brightness
static BUTTONS_DEFAULT_BRIGHTNESS: u8 = 8;

pub trait BeacnControlDeviceAttach {
    // We're specifically allowing the DeviceDefinition to be a private interface, as it's
    // simply used internally for connection up a device, and shouldn't have any visibility
    // from the outside. This also prevents external code from attempting to call connect.
    #[allow(private_interfaces)]
    fn connect(
        definition: DeviceDefinition,
        interaction: Option<mpsc::Sender<Interactions>>,
    ) -> Result<Box<dyn BeacnControlDevice>>
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
        interaction: Option<mpsc::Sender<Interactions>>,
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
        let mut input_buffer = [0; 64];

        // Timeout Handlers
        let timeout = Duration::from_millis(2000);

        // At this point, we need to pull out the USB handler and wrap it up
        let handle = Arc::new(handler.handle);
        let poll = if is_notify {
            let handler_clone = handle.clone();
            let tx_clone = input_tx.clone();
            thread::spawn(move || {
                debug!("Spawning Event Listener");

                // Input buffer for messages
                let mut input = [0; 64];

                let handle = handler_clone;
                let input_tx = tx_clone;
                let read = Duration::from_secs(60);
                loop {
                    // Firstly, we need to fire off a message saying we're ready for buttons
                    match handle.read_interrupt(0x83, &mut input, read) {
                        Ok(_) => {
                            if input_tx.send(input).is_err() {
                                // Our channel is gone or closed, bail.
                                warn!("Message Channel Closed, Terminating");
                                break;
                            }
                        }
                        Err(usb_error) => {
                            // Timeout is a completely acceptable error to have, it just means
                            // the user hasn't moved a dial or pressed a button in the last
                            // `read` seconds, and we're good to wait again.
                            if usb_error != Timeout {
                                // Other errors means that something's gone horribly wrong, and
                                // we should straight up abort our efforts.
                                warn!("USB Error while receiving inputs: {}", usb_error);
                                break;
                            }
                        }
                    }
                }

                debug!("Event Listener Terminated");
            });
            never()
        } else {
            tick(Duration::from_millis(50))
        };

        // Force the device into a 'wake' state if it's currently sleeping
        let wake = [00, 00, 00, 0xf1];
        if handle.write_interrupt(0x03, &wake, timeout).is_err() {
            error!("Unable to Wake Device");
            return;
        }

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

        let mut dim_duration = Duration::from_secs(DISPLAY_DEFAULT_DIM_TIME);

        // Create some timers for processing
        let mut dim_timeout = after(dim_duration);

        // TODO: I should probably use a Macro or a closure to handle the recv
        // In all cases, if a channel has closed, we should abort.
        debug!("Spawning Event Handler for {}", handler.serial);
        'primary: loop {
            select! {
                recv(rx) -> msg => {
                    match msg {
                        Ok(msg) => {
                            match msg {
                                ControlThreadSender::Stop => {
                                    debug!("Stopping Event Handler");
                                    break;
                                }
                                KeepAlive => {
                                    if handle.write_interrupt(0x03, &[00, 00, 00, 0xf1], timeout).is_err() {
                                        error!("Error Sending Keep-Alive Request");
                                        break;
                                    }
                                }
                                SetEnabled(enabled) => {
                                    let byte = if enabled { 0 } else { 1 };
                                    let message = [0, 1, 0, 4, byte, 0, 0, 0];

                                    if handle.write_interrupt(0x03, &message, timeout).is_err() {
                                        error!("Failed to Send Enabled Message");
                                        break 'primary;
                                    }
                                }
                                SetImage(x, y, img) => {
                                    // Ok, lets try sending TUX :D
                                    let mut iter = img.chunks(1020).enumerate().peekable();
                                    let mut output = [0; 1024];

                                    while let Some((index, value)) = iter.next() {
                                        LittleEndian::write_u24(&mut output[0..3], index as u32);
                                        output[3] = 0x50;

                                        // Write this chunk to the USB stream
                                        output[4..value.len() + 4].copy_from_slice(value);
                                        if handle.write_interrupt(0x03, &output, timeout).is_err() {
                                            error!("Failed to Write Image");
                                            break 'primary;
                                        }

                                        // Check if we're the last packet...
                                        if iter.peek().is_none() {
                                            // Flag the message as complete
                                            output[0] = 0xff;
                                            output[1] = 0xff;
                                            output[2] = 0xff;
                                            output[3] = 0x50;

                                            // Send the Total size of the image
                                            LittleEndian::write_u32(&mut output[4..8], img.len() as u32 - 1);

                                            // Set the X and Y coordinates..
                                            LittleEndian::write_u32(&mut output[8..12], x);
                                            LittleEndian::write_u32(&mut output[12..16], y);

                                            // Send this out via USB.
                                            if handle.write_interrupt(0x03, &output, timeout).is_err() {
                                                error!("Failed to write final message");
                                                break 'primary;
                                            }
                                        }
                                    }
                                }
                                SetDimTimeout(timeout) => {
                                    dim_duration = timeout;
                                    if !is_dimmed {
                                        // If we're not already dimmed, reset the timer
                                        dim_timeout = after(timeout);
                                    }
                                }
                                SetActiveBrightness(percent) => {
                                    if is_dimmed {
                                        is_dimmed = false;
                                        dim_timeout = after(dim_duration);
                                    }
                                    active_brightness = percent;
                                    if handle.write_interrupt(0x03, &[0, 0, 0, 4, active_brightness, 0, 0, 0], timeout).is_err() {
                                        error!("Failed to Set Brightness");
                                        break;
                                    }
                                }
                                SetButtonBrightness(value) => {
                                    button_brightness = value;
                                    if handle.write_interrupt(0x03, &[1, 7, 0, 4, button_brightness, 0, 0, 0], timeout).is_err() {
                                        error!("Failed to Set Button Brightness");
                                        break;
                                    }
                                }
                                SetButtonColour(button, colour) => {
                                    let message = [1, button, 0, 4, colour.blue, colour.green, colour.red, colour.alpha];
                                    if handle.write_interrupt(0x03,&message,timeout).is_err() {
                                        error!("Failed to Set Button Colour");
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
                recv(dim_timeout) -> msg => {
                    match msg {
                        Ok(_) => {
                            is_dimmed = true;
                            if handle.write_interrupt(0x03, &[0, 0, 0, 4, DISPLAY_DEFAULT_DIM_BRIGHTNESS, 0, 0, 0], timeout).is_err() {
                                error!("Failed to Set DIM brightness");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("DIM Timeout Receiver broken {}", e);
                            break;
                        }
                    }
                }
                recv(input_rx) -> msg => {
                    match msg {
                        Ok(input) => {
                            let (changed, button_state) = Self::handle_interaction(input, last_button_state, &interaction);
                            last_button_state = button_state;

                            if changed {
                                if is_dimmed {
                                    // We need to wake up screen
                                    is_dimmed = false;
                                    if handle.write_interrupt(0x03, &[0, 0, 0, 4, active_brightness, 0, 0, 0], timeout).is_err() {
                                        error!("Failed to Set DIM brightness");
                                        break;
                                    }
                                }

                                // Set a new Dim timeout
                                dim_timeout = after(dim_duration);
                            }
                        },
                        Err(e) => {
                            error!("Input Receiver Terminated: {:?}", e);
                            break;
                        }
                    }
                }
                recv(poll) -> msg => {
                    // Ok, we're at a poll interval, we need to fetch changes to inputs
                    match msg {
                        Ok(_) => {
                            if handle.write_interrupt(0x03, &[0, 0, 0, 5], timeout).is_err() {
                                debug!("Error Sending Poll Request");
                                break;
                            }
                            if handle.read_interrupt(0x83, &mut input_buffer, timeout).is_ok() {
                                if input_tx.send(input_buffer).is_err() {
                                    debug!("Failed to Send Poll Response Data");
                                    break;
                                };
                            } else {
                                debug!("Error Reading Poll Response");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Poll Receiver Terminated: {:?}", e);
                            break;
                        }
                    }
                }
            }
        }

        debug!("Event Handler Terminated");
    }

    fn handle_interaction(
        message: [u8; 64],
        last: u16,
        tx: &Option<mpsc::Sender<Interactions>>,
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

    fn set_enabled(&self, enabled: bool) -> Result<()> {
        self.get_sender().send(SetEnabled(enabled))?;
        Ok(())
    }

    fn send_keepalive(&self) -> Result<()> {
        self.get_sender().send(KeepAlive)?;
        Ok(())
    }

    fn set_image(&self, x: u32, y: u32, jpeg_image: &[u8]) -> Result<()> {
        // TODO: This might be too heavy for a frequent update check (for example, metering)

        // All we do here is validate the image and make sure it fits inside the window
        // Firstly, make sure we're rendering to the actual screen
        let display_size = self.get_display_size();
        if x > display_size.0 || y > display_size.1 {
            bail!(
                "Position should be between 0..{}, 0..{}",
                display_size.0,
                display_size.1
            );
        }

        // Load out the image, and get the width + height
        let mut decoder = Decoder::new(jpeg_image);
        decoder.read_info()?;

        if let Some(info) = decoder.info() {
            if (x + info.width as u32) > display_size.0 {
                bail!(
                    "Image overflows display width, {}>{}",
                    x + info.width as u32,
                    display_size.0
                );
            }
            if (y + info.height as u32) > display_size.1 {
                bail!(
                    "Image overflows display height, {}>{}",
                    x + info.height as u32,
                    display_size.1
                );
            }
        } else {
            bail!("Unable to Fetch Image Info");
        }

        self.get_sender()
            .send(SetImage(x, y, Vec::from(jpeg_image)))?;
        Ok(())
    }

    fn set_display_brightness(&self, brightness: u8) -> Result<()> {
        if !(1..=100).contains(&brightness) {
            bail!("Display Brightness must be a percentage");
        }

        self.get_sender().send(SetActiveBrightness(brightness))?;
        Ok(())
    }

    fn set_button_brightness(&self, brightness: u8) -> Result<()> {
        if !(0..=10).contains(&brightness) {
            bail!("Button Brightness must be between 0 and 10");
        }
        self.get_sender().send(SetButtonBrightness(brightness))?;
        Ok(())
    }

    fn set_dim_timeout(&self, timeout: Duration) -> Result<()> {
        if timeout > Duration::from_secs(300) || timeout < Duration::from_secs(30) {
            bail!("For display safety, dim timeout must be lower than 5 minutes, and greater than 30 seconds");
        }

        self.get_sender().send(SetDimTimeout(timeout))?;
        Ok(())
    }

    fn set_button_colour(&self, button: ButtonLighting, colour: RGBA) -> Result<()> {
        let button = button as u8;
        self.get_sender().send(SetButtonColour(button, colour))?;
        Ok(())
    }
}

/// Simple function to Open a libusb connection to a Beacn Audio device, do initial setup and
/// grab the firmware version from the device.
pub(crate) fn open_beacn(def: DeviceDefinition, product_id: u16) -> Result<BeacnDeviceHandle> {
    if def.descriptor.product_id() != product_id {
        bail!(
            "Expecting PID {} but got {}",
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
