use crate::common::{BeacnDeviceHandle, DeviceDefinition, get_device_info};
use crate::controller::ControlThreadSender::{
    KeepAlive, SetActiveBrightness, SetButtonBrightness, SetButtonColour, SetDimTimeout,
    SetEnabled, SetImage,
};
use crate::controller::device::runner::BeacnControlDeviceRunner;
use crate::controller::{BeacnControlDevice, ButtonLighting, ControlThreadSender, Interactions};
use crate::transfer::transfer;
use crate::types::RGBA;

use crate::sealed::Sealed;
use crate::{BResult, beacn_bail};
use anyhow::Error;
use anyhow::Result;
use async_trait::async_trait;
use flume::{Receiver, Sender, bounded};
use jpeg_decoder::Decoder;
use log::debug;
use nusb::transfer::{Buffer, In, Interrupt, Out};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use std::{mem, thread};

#[async_trait]
pub trait BeacnControlDeviceAttach: Sealed {
    // We're specifically allowing the DeviceDefinition to be a private interface, as it's
    // simply used internally for connection up a device, and shouldn't have any visibility
    // from the outside. This also prevents external code from attempting to call connect.
    fn get_product_id(&self) -> u16;
    fn get_serial(&self) -> String;
    fn get_version(&self) -> String;

    fn get_display_size(&self) -> (u32, u32);
}

pub(crate) trait BeacnControlDeviceInternal: Sealed {
    async fn connect(
        definition: DeviceDefinition,
        interaction: Option<Sender<Interactions>>,
        health_tx: Sender<()>,
    ) -> BResult<Arc<Box<dyn BeacnControlDevice>>>
    where
        Self: Sized;

    fn get_sender(&self) -> Result<&Sender<ControlThreadSender>>;
    fn set_sender_enabled(&self, enabled: bool);
}

// For the most part, the Mix and Mix Create handle interactions identically, obviously the
// mix has fewer buttons, but the firmware seems to do a decent job of handling that, so we
// can simply use the same behaviour between the two
#[allow(private_bounds)]
pub trait BeacnControlAPI:
    BeacnControlDeviceAttach + BeacnControlDeviceInternal + BeacnControlDeviceRunner + Sealed
{
    fn set_enabled(&self, enabled: bool) -> BResult<()> {
        let (tx, rx) = oneshot::channel();

        self.get_sender()?
            .send(SetEnabled(enabled, tx))
            .map_err(Error::from)?;

        rx.recv().map_err(Error::from)?;
        Ok(())
    }

    fn send_keepalive(&self) -> BResult<()> {
        let (tx, rx) = oneshot::channel();
        self.get_sender()?
            .send(KeepAlive(tx))
            .map_err(Error::from)?;
        rx.recv().map_err(Error::from)?;
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

        let (tx, rx) = oneshot::channel();

        self.get_sender()?
            .send(SetImage(x, y, Vec::from(jpeg_image), tx))
            .map_err(Error::from)?;

        rx.recv().map_err(Error::from)?;
        Ok(())
    }

    fn set_display_brightness(&self, brightness: u8) -> BResult<()> {
        if !(1..=100).contains(&brightness) {
            beacn_bail!("Display Brightness must be a percentage");
        }

        let (tx, rx) = oneshot::channel();
        self.get_sender()?
            .send(SetActiveBrightness(brightness, tx))
            .map_err(Error::from)?;

        rx.recv().map_err(Error::from)?;
        Ok(())
    }

    fn set_button_brightness(&self, brightness: u8) -> BResult<()> {
        if !(0..=10).contains(&brightness) {
            beacn_bail!("Button Brightness must be between 0 and 10");
        }

        let (tx, rx) = oneshot::channel();
        self.get_sender()?
            .send(SetButtonBrightness(brightness, tx))
            .map_err(Error::from)?;

        rx.recv().map_err(Error::from)?;
        Ok(())
    }

    fn set_dim_timeout(&self, timeout: Duration) -> BResult<()> {
        if timeout > Duration::from_secs(300) || timeout < Duration::from_secs(30) {
            beacn_bail!(
                "For display safety, dim timeout must be lower than 5 minutes, and greater than 30 seconds"
            );
        }

        let (tx, rx) = oneshot::channel();
        self.get_sender()?
            .send(SetDimTimeout(timeout, tx))
            .map_err(Error::from)?;

        rx.recv().map_err(Error::from)?;
        Ok(())
    }

    fn set_button_colour(&self, button: ButtonLighting, colour: RGBA) -> BResult<()> {
        let button = button as u8;

        let (tx, rx) = oneshot::channel();
        self.get_sender()?
            .send(SetButtonColour(button, colour, tx))
            .map_err(Error::from)?;

        rx.recv().map_err(Error::from)?;
        Ok(())
    }
}

/// Simple function to Open a USB connection to a Beacn Audio device, do initial setup, and
/// grab the firmware version from the device.
pub(crate) async fn open_beacn(
    def: DeviceDefinition,
    product_id: &[u16],
) -> BResult<BeacnDeviceHandle> {
    if !product_id.contains(&def.descriptor.product_id()) {
        beacn_bail!(
            "Expecting PIDs {:?} but got {}",
            product_id,
            def.descriptor.product_id()
        );
    }

    let device = crate::setup::open(&def.descriptor).await?;
    let interface = crate::setup::claim_interface(&device, 0).await?;
    crate::setup::set_alt_setting(&interface, 1).await?;

    // Unlike the Mic and Studio, we use an interrupt, rather a bulk read
    let mut out_ep = interface.endpoint::<Interrupt, Out>(0x03)?;
    let mut in_ep = interface.endpoint::<Interrupt, In>(0x83)?;

    let setup_timeout = Duration::from_millis(2000);
    transfer(&mut out_ep, [0, 0, 0, 0].into(), setup_timeout).await?;
    transfer(&mut out_ep, [0, 0, 0, 1].into(), setup_timeout).await?;
    let completion = transfer(&mut in_ep, Buffer::new(64), setup_timeout).await?;

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

pub fn tick(duration: Duration) -> Receiver<()> {
    let (tx, rx) = bounded(1);

    thread::spawn(move || {
        loop {
            sleep(duration);

            // Use try_send to avoid blocking the thread if the channel is full
            match tx.try_send(()) {
                Ok(_) => {}
                Err(flume::TrySendError::Full(_)) => {}
                Err(flume::TrySendError::Disconnected(_)) => break,
            }
        }
    });

    rx
}

pub fn never<T>() -> Receiver<T> {
    let (tx, rx) = bounded(0);

    // This *TECHNICALLY* leaks memory, but the number of occurrences will be tiny.
    mem::forget(tx);
    rx
}
