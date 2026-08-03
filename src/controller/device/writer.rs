use crate::transfer::{EndpointHandle, transfer};
use anyhow::{Result, bail};
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

    pub(crate) async fn send(&mut self, data: &[u8]) -> Result<(), TransferError> {
        self.send_timeout(data, self.timeout).await
    }

    /// Send a USB interrupt OUT transfer.
    pub(crate) async fn send_timeout(
        &mut self,
        data: &[u8],
        timeout: Duration,
    ) -> Result<(), TransferError> {
        transfer(&mut self.endpoint, data.to_vec(), timeout)
            .await
            .map(|_| ())
    }
}
