use crate::common::DeviceDefinition;
use crate::controller::ControlThreadSender::{
    KeepAlive, SetActiveBrightness, SetButtonBrightness, SetButtonColour, SetDimTimeout,
    SetEnabled, SetImage,
};
use crate::controller::device::runner::BeacnControlDeviceRunner;
use crate::controller::{BeacnControlDevice, ButtonLighting, ControlThreadSender, Interactions};
use crate::sealed::Sealed;
use crate::types::RGBA;
use crate::{BResult, beacn_bail};
use anyhow::Error;
use anyhow::Result;
use async_trait::async_trait;
use flume::{Sender};
use jpeg_decoder::Decoder;
use std::sync::Arc;
use std::time::Duration;

#[async_trait]
pub trait BeacnControlDeviceInfo: Sealed {
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
    BeacnControlDeviceInfo + BeacnControlDeviceInternal + BeacnControlDeviceRunner + Sealed
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