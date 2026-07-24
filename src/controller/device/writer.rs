use log::warn;
use nusb::MaybeFuture;
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

    /// Send a USB interrupt OUT transfer.
    pub(crate) fn send(&mut self, data: &[u8]) -> Result<(), TransferError> {
        match self.send_once(data) {
            Ok(()) => Ok(()),

            Err(TransferError::Stall) => {
                warn!("USB endpoint stalled, clearing halt");

                self.endpoint
                    .clear_halt()
                    .wait()
                    .map_err(|_| TransferError::Disconnected)?;

                self.send_once(data)
            }

            Err(e) => Err(e),
        }
    }

    /// Perform the actual transfer.
    ///
    /// This deliberately does not handle recovery. Recovery belongs in send()
    /// so every caller gets identical behaviour.
    fn send_once(&mut self, data: &[u8]) -> Result<(), TransferError> {
        self.endpoint
            .transfer_blocking(Buffer::from(data.to_vec()), self.timeout)
            .into_result()
            .map(|_| ())
    }
}
