use crate::transfer::{EndpointHandle, transfer};
use anyhow::{Result, bail};
use log::warn;
use nusb::Interface;
use nusb::transfer::{Interrupt, Out, TransferError};
use web_time::Duration;

#[allow(dead_code)]
pub(crate) struct UsbWriter {
    endpoint: EndpointHandle<Interrupt, Out>,
    timeout: Duration,
}

impl UsbWriter {
    pub(crate) fn new(interface: Interface, timeout: Duration) -> Result<Self> {
        let endpoint = EndpointHandle::<Interrupt, Out>::new(interface, 0x03);
        let endpoint = match endpoint {
            Ok(ep) => ep,
            Err(e) => {
                bail!("Failed to open Interrupt OUT endpoint: {}", e);
            }
        };

        Ok(Self { endpoint, timeout })
    }

    pub(crate) async fn clear_halt(&mut self) -> Result<(), TransferError> {
        self.endpoint
            .clear_halt()
            .await
            .map_err(|_| TransferError::Disconnected)
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
                self.clear_halt().await?;
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
        transfer(&mut self.endpoint, data.to_vec(), timeout)
            .await
            .map(|_| ())
    }
}
