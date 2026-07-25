use crate::transfer::transfer_with_timeout;
use log::warn;
use nusb::transfer::{Buffer, Interrupt, Out, TransferError};
use std::time::Duration;

pub(crate) struct UsbWriter<'a> {
    endpoint: &'a mut nusb::Endpoint<Interrupt, Out>,
    timeout: Duration,
}

impl<'a> UsbWriter<'a> {
    pub(crate) fn new(endpoint: &'a mut nusb::Endpoint<Interrupt, Out>, timeout: Duration) -> Self {
        Self { endpoint, timeout }
    }

    pub(crate) async fn send(&mut self, data: &[u8]) -> Result<(), TransferError> {
        self.send_timeout(data, self.timeout).await
    }

    /// Send a USB interrupt OUT transfer.
    pub(crate) async fn send_timeout(
        &mut self,
        data: &[u8],
        timeout: Duration,
    ) -> Result<(), TransferError> {
        match self.send_once(data, timeout).await {
            Ok(()) => Ok(()),

            Err(TransferError::Stall) => {
                warn!("USB endpoint stalled, clearing halt");
                crate::setup::clear_halt(self.endpoint)
                    .await
                    .map_err(|_| TransferError::Disconnected)?;
                self.send_once(data, timeout).await
            }

            Err(e) => Err(e),
        }
    }

    /// Perform the actual transfer.
    ///
    /// This deliberately does not handle recovery. Recovery belongs in send()
    /// so every caller gets identical behaviour.
    async fn send_once(&mut self, data: &[u8], timeout: Duration) -> Result<(), TransferError> {
        transfer_with_timeout(self.endpoint, Buffer::from(data.to_vec()), timeout)
            .await
            .into_result()
            .map(|_| ())
    }
}
