use crate::timers::sleep;
use futures_lite::future::or;
use nusb::transfer::{Buffer, BulkOrInterrupt, Completion, EndpointDirection, TransferError};
use nusb::{Endpoint, Interface};
use web_time::Duration;

// Ok, this is a wrapper around Interface where we can keep and manage the lifecycle
pub(crate) struct EndpointHandle<EpType, Dir> {
    interface: Interface,
    address: u8,

    // None would be transient if a transfer in normal operation we'd have something here,
    // in wasm this may become None if a transfer is cancelled
    endpoint: Option<Endpoint<EpType, Dir>>,
}

impl<EpType, Dir> EndpointHandle<EpType, Dir>
where
    EpType: BulkOrInterrupt,
    Dir: EndpointDirection,
{
    pub(crate) fn new(interface: Interface, address: u8) -> Result<Self, nusb::Error> {
        // While we strictly don't need to open the endpoint here (it'll open on first use), this
        // allows us to immediately report to the caller if the endpoint is unusable.
        let endpoint = interface.endpoint::<EpType, Dir>(address)?;
        Ok(Self {
            interface,
            address,
            endpoint: Some(endpoint),
        })
    }

    pub(crate) fn get_mut(&mut self) -> Result<&mut Endpoint<EpType, Dir>, nusb::Error> {
        if self.endpoint.is_none() {
            self.endpoint = Some(self.interface.endpoint::<EpType, Dir>(self.address)?);
        }
        Ok(self.endpoint.as_mut().expect("Endpoint should be set"))
    }

    pub(crate) async fn clear_halt(&mut self) -> Result<(), nusb::Error> {
        if let Some(ep) = self.endpoint.as_mut() {
            #[cfg(any(feature = "tokio", feature = "smol", target_arch = "wasm32"))]
            {
                #[cfg(not(target_arch = "wasm32"))]
                ep.cancel_all();

                ep.clear_halt().await
            }

            #[cfg(not(any(feature = "tokio", feature = "smol", target_arch = "wasm32")))]
            {
                use nusb::MaybeFuture;
                ep.cancel_all();
                ep.clear_halt().wait()
            }
        } else {
            Ok(())
        }
    }

    // Forcibly drop the endpoint, it'll be recreated on next use. Note this is only currently
    // used in wasm, but is available for other cases if needed.
    #[allow(unused)]
    pub(crate) fn drop_endpoint(&mut self) {
        self.endpoint = None;
    }
}

/// This is basically a drop-in replacement for nusb::transfer::transfer_with_timeout designed
/// to be run in a non-blocking async environment. For now, it's functionally identical, but
/// that may end up changing depending on internal use cases :D
pub(crate) async fn transfer_with_timeout<EpType, Dir>(
    endpoint: &mut EndpointHandle<EpType, Dir>,
    buf: Buffer,
    timeout: Duration,
) -> Completion
where
    EpType: BulkOrInterrupt,
    Dir: EndpointDirection,
{
    // Get the Endpoint
    let Ok(ep) = endpoint.get_mut() else {
        // We can't get or open the endpoint. Report it the same was as it would if the
        // endpoint went missing mis-stream.
        return Completion {
            buffer: Buffer::new(0),
            status: Err(TransferError::Disconnected),
            actual_len: 0,
        };
    };

    ep.submit(buf);

    // Race the transfer with the timeout, whichever completes first wins.
    let outcome = or(async { Some(ep.next_complete().await) }, async {
        sleep(timeout).await;
        None
    })
    .await;

    match outcome {
        Some(completion) => completion,
        None => {
            // The sleep won the race, we need to cancel the request and return.

            #[cfg(not(target_arch = "wasm32"))]
            {
                // Use the built-in cancel functionality
                ep.cancel_all();

                // Want for the cancel to complete and return the completion
                ep.next_complete().await
            }

            #[cfg(target_arch = "wasm32")]
            {
                use nusb::transfer::TransferError;

                // The only way to cancel a transfer in wasm is to drop the endpoint, so we'll
                // do that here, and return a cancelled completion.
                endpoint.drop_endpoint();

                Completion {
                    buffer: Buffer::new(0),
                    status: Err(TransferError::Cancelled),
                    actual_len: 0,
                }
            }
        }
    }
}

/// Same as transfer_with_timeout, but returns a Result<Buffer, TransferError> instead of a
/// Completion. This is useful for when you want to handle the error case separately from the
/// success case, and you don't want to have to deal with the Completion type.
pub(crate) async fn transfer<EpType, Dir>(
    endpoint: &mut EndpointHandle<EpType, Dir>,
    buf: Buffer,
    timeout: Duration,
) -> Result<Buffer, TransferError>
where
    EpType: BulkOrInterrupt,
    Dir: EndpointDirection,
{
    transfer_with_timeout(endpoint, buf, timeout)
        .await
        .into_result()
}
