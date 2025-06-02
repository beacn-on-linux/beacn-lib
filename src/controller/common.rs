use crate::common::{BeacnDeviceHandle, DeviceDefinition, get_device_info};
use crate::controller::ButtonState::{Press, Release};
use crate::controller::{BeacnControlDevice, Buttons, ControlThreadManager, Dials, Interactions};
use crate::version::VersionNumber;
use anyhow::Result;
use anyhow::bail;
use byteorder::{BigEndian, ByteOrder};
use crossbeam::channel::{bounded, never, tick, Receiver};
use crossbeam::select;
use log::{debug, error, warn};
use rusb::Error::Timeout;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;
use strum::IntoEnumIterator;

pub trait BeacnControlDeviceAttach {
    // We're specifically allowing the DeviceDefinition to be a private interface, as it's
    // simply used internally for connection up a device, and shouldn't have any visibility
    // from the outside. This also prevents external code from attempting to call connect.
    #[allow(private_interfaces)]
    fn connect(
        definition: DeviceDefinition,
        interaction: mpsc::Sender<Interactions>,
    ) -> Result<Box<dyn BeacnControlDevice>>
    where
        Self: Sized;

    fn get_product_id(&self) -> u16;
    fn get_serial(&self) -> String;
    fn get_version(&self) -> String;
}

// For the most part, the Mix and Mix Create handle interactions identically, obviously the
// mix has fewer buttons, but the firmware seems to do a decent job of handling that, so we
// can simply use the same behaviour between the
pub trait BeacnControlInteraction: BeacnControlDeviceAttach {
    #[allow(private_interfaces)]
    fn spawn_event_handler(
        rx: Receiver<ControlThreadManager>,
        handler: BeacnDeviceHandle,
        interaction: mpsc::Sender<Interactions>,
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

        // This tracks the button states (so we can message on Send / Receive)
        let mut last_button_state = 0;

        // TODO: I should probably use a Macro or a closure to handle the recv
        // In all cases, if a channel has closed, we should abort.
        debug!("Spawning Event Handler for {}", handler.serial);
        loop {
            select! {
                recv(rx) -> msg => {
                    match msg {
                        Ok(msg) => {
                            match msg {
                                ControlThreadManager::STOP => {
                                    debug!("Stopping Event Handler");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Main Event Receiver Error: {}", e);
                            break;
                        }
                    }
                }
                recv(input_rx) -> msg => {
                    match msg {
                        Ok(input) => {
                            last_button_state = Self::handle_interaction(input, last_button_state, &interaction);
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

    fn handle_interaction(message: [u8; 64], last: u16, tx: &mpsc::Sender<Interactions>) -> u16
    where
        Self: Sized,
    {
        let dials = &message[4..8];
        for dial in Dials::iter() {
            if dials[dial as usize] != 0 {
                let change = dials[dial as usize] as i8;
                let _ = tx.send(Interactions::DialChanged(dial, change));
                debug!("Dial Moved: {} - {}", dial, change);
            }
        }

        let buttons = BigEndian::read_u16(&message[8..10]);
        for button in Buttons::iter() {
            let button_pressed = (buttons >> button as u8) & 1;
            if ((last >> button as u8) & 1) != button_pressed {
                if (buttons >> button as u8) & 1 == 1 {
                    let _ = tx.send(Interactions::ButtonPress(button, Press));
                    debug!("Button Pressed: {}", button);
                } else {
                    let _ = tx.send(Interactions::ButtonPress(button, Release));
                    debug!("Button Released: {}", button);
                }
            }
        }
        buttons
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
