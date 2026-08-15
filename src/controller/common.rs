use crate::common::DeviceDefinition;
use crate::controller::device::runner::BeacnControlDeviceRunner;
use crate::controller::messages::Message;
use crate::controller::{BeacnControlDevice, ControlThreadSender, Interactions};
use crate::sealed::Sealed;
use crate::{BResult, beacn_bail};
use anyhow::Result;
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use flume::Sender;
use jpeg_decoder::Decoder;
use std::sync::Arc;

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
#[async_trait]
pub trait BeacnControlAPI:
    BeacnControlDeviceInfo + BeacnControlDeviceInternal + BeacnControlDeviceRunner + Sealed
{
    async fn handle_message(&self, message: Message) -> BResult<()> {
        // Firstly, do any validation that's needed on the messages
        match &message {
            Message::SetImage(x, y, i) => {
                let display_size = self.get_display_size();
                if *x > display_size.0 || *y > display_size.1 {
                    beacn_bail!(
                        "Position should be between 0..{}, 0..{}",
                        display_size.0,
                        display_size.1
                    );
                }

                // Load out the image, and get the width + height
                let mut decoder = Decoder::new(&**i);
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
            }
            Message::SetActiveBrightness(b) => {
                if !(1..=100).contains(b) {
                    beacn_bail!("Display Brightness must be a percentage");
                }
            }
            Message::SetButtonBrightness(b) => {
                if !(0..=10).contains(b) {
                    beacn_bail!("Button Brightness must be between 0 and 10");
                }
            }

            #[allow(clippy::collapsible_match)]
            Message::SetDimTimeout(t) => {
                if !(30..=300).contains(&t.as_secs()) {
                    let err = "Dim timeout must be between 30 and 300 seconds";
                    beacn_bail!(anyhow!("{err}"));
                }
            }
            _ => {}
        };

        // Wrap the message up in a oneshot, then send it to the control thread.
        let (tx, rx) = oneshot::channel();

        self.get_sender()?
            .send_async(ControlThreadSender::SendMessage(message, tx))
            .await
            .map_err(Error::from)?;

        rx.await.map_err(Error::from)?;
        Ok(())
    }
}
