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

    /// Send a USB interrupt OUT transfer.
    pub(crate) async fn send(&mut self, data: &[u8]) -> Result<(), TransferError> {
        match self.send_once(data).await {
            Ok(()) => Ok(()),

            Err(TransferError::Stall) => {
                warn!("USB endpoint stalled, clearing halt");

                // clear_halt() is one of the small set of nusb operations that need a
                // blocking syscall under the hood. Awaiting it only works if the
                // `tokio` or `smol` crate feature is enabled (so nusb has somewhere to
                // hand the blocking call off to); otherwise it panics. Stalls are rare
                // (this is error-recovery, not the hot path), so falling back to a
                // direct blocking `.wait()` here when no runtime feature is enabled is
                // an acceptable trade-off rather than forcing every caller to pick a
                // runtime just to compile.
                #[cfg(any(feature = "tokio", feature = "smol"))]
                {
                    self.endpoint
                        .clear_halt()
                        .await
                        .map_err(|_| TransferError::Disconnected)?;
                }
                #[cfg(not(any(feature = "tokio", feature = "smol")))]
                {
                    use nusb::MaybeFuture;
                    self.endpoint
                        .clear_halt()
                        .wait()
                        .map_err(|_| TransferError::Disconnected)?;
                }

                self.send_once(data).await
            }

            Err(e) => Err(e),
        }
    }

    /// Perform the actual transfer.
    ///
    /// This deliberately does not handle recovery. Recovery belongs in send()
    /// so every caller gets identical behaviour.
    async fn send_once(&mut self, data: &[u8]) -> Result<(), TransferError> {
        transfer_with_timeout(self.endpoint, Buffer::from(data.to_vec()), self.timeout)
            .await
            .into_result()
            .map(|_| ())
    }
}
