use async_io::Timer;
use futures_lite::future::or;
use nusb::Endpoint;
use nusb::transfer::{Buffer, BulkOrInterrupt, Completion, EndpointDirection};
use std::time::Duration;

/// This is basically a drop-in replacement for nusb::transfer::transfer_with_timeout designed
/// to be run in a non-blocking async environment. For now, it's functionally identical, but
/// that may end up changing depending on internal use cases :D
pub(crate) async fn transfer_with_timeout<EpType, Dir>(
    endpoint: &mut Endpoint<EpType, Dir>,
    buf: Buffer,
    timeout: Duration,
) -> Completion
where
    EpType: BulkOrInterrupt,
    Dir: EndpointDirection,
{
    endpoint.submit(buf);

    // Race the transfer completion against a timer. `next_complete()` is documented as
    // cancel-safe, so dropping it (when the timer wins the race) is fine.
    let outcome = or(async { Some(endpoint.next_complete().await) }, async {
        Timer::after(timeout).await;
        None
    })
    .await;

    match outcome {
        Some(completion) => completion,
        None => {
            // Timed out. Request cancellation, then wait so we never leave a dangling pending
            // transfer on the endpoint for the next caller to trip over.
            endpoint.cancel_all();
            endpoint.next_complete().await
        }
    }
}

/// Same as transfer_with_timeout, but returns a Result<Buffer, TransferError> instead of a
/// Completion. This is useful for when you want to handle the error case separately from the
/// success case, and you don't want to have to deal with the Completion type.
#[allow(unused)]
pub(crate) async fn transfer<EpType, Dir>(
    endpoint: &mut Endpoint<EpType, Dir>,
    buf: Buffer,
    timeout: Duration,
) -> Result<Buffer, nusb::transfer::TransferError>
where
    EpType: BulkOrInterrupt,
    Dir: EndpointDirection,
{
    endpoint.submit(buf);

    // Race the transfer completion against a timer. `next_complete()` is documented as
    // cancel-safe, so dropping it (when the timer wins the race) is fine.
    let outcome = or(async { Some(endpoint.next_complete().await) }, async {
        Timer::after(timeout).await;
        None
    })
    .await;

    match outcome {
        Some(completion) => completion.into_result(),
        None => {
            // Timed out. Request cancellation, then wait so we never leave a dangling pending
            // transfer on the endpoint for the next caller to trip over.
            endpoint.cancel_all();
            endpoint.next_complete().await.into_result()
        }
    }
}
