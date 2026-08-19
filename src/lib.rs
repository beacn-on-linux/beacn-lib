//pub mod device;
pub mod audio;
mod common;
pub mod controller;
pub mod manager;
mod setup;
mod sync;
mod timers;
mod transfer;
pub mod types;
pub mod version;

pub use flume;
use log::debug;
pub use nusb::ErrorKind as UsbError;
pub use nusb::transfer::TransferError as UsbTransferError;

use crate::version::VersionNumber;
use std::future::Future;
use thiserror::Error;

// We kinda need this for sanity reasons, it helps IDEs expand the macros for message groups
#[allow(clippy::single_component_path_imports)]
use paste;

#[cfg(all(target_arch = "wasm32", feature = "tokio"))]
compile_error!("Cannot use the tokio feature on wasm, use async instead");

#[cfg(all(target_arch = "wasm32", feature = "async-rt"))]
compile_error!("Cannot use the smol feature on wasm, use async instead");

#[cfg(all(target_arch = "wasm32", not(feature = "async")))]
compile_error!("The async feature is required for wasm");

#[cfg(all(
    feature = "async",
    not(target_arch = "wasm32"),
    not(any(feature = "tokio", feature = "async-rt"))
))]
compile_error!(
    "The `async` feature on non-WASM targets requires either the `tokio` or `async-rt` feature."
);

/// We try to support async everywhere, but for blocking environments this trait uses futures-lite
/// to allow calling .wait() instead of .await as a blocking call inside a non-async context.
pub trait MaybeFuture: Future + Sized {
    /// Block the current thread until this operation completes.
    #[cfg(not(target_arch = "wasm32"))]
    fn wait(self) -> Self::Output {
        async_io::block_on(self)
    }

    // Per the error, this is not supported in wasm.
    #[cfg(target_arch = "wasm32")]
    fn wait(self) -> Self::Output {
        unimplemented!("Cannot block thread in wasm, .await the future instead")
    }
}

impl<F: Future> MaybeFuture for F {}

// These are some helper versions, which can be used to determine feature availability
const MIC_CLASS_COMPLIANT_VERSION: VersionNumber = VersionNumber(1, 2, 0, 188);
const EQ_HEADPHONES_VERSION: VersionNumber = VersionNumber(1, 3, 0, 0);

pub type BResult<T> = Result<T, BeacnError>;

// This is a general error handler for the entire library, we might need to reexport rusb::Error
#[derive(Debug, Error)]
pub enum BeacnError {
    #[error("USB error: {0:?}")]
    Usb(UsbError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<nusb::Error> for BeacnError {
    fn from(err: nusb::Error) -> Self {
        debug!("Received nusb Error: {}", err);
        BeacnError::Usb(err.kind())
    }
}

// Convert a nusb::transfer::TransferError into an anyhow::Error
impl From<UsbTransferError> for BeacnError {
    fn from(err: UsbTransferError) -> Self {
        BeacnError::Other(err.into())
    }
}

#[macro_export]
macro_rules! beacn_bail {
    // formatted string form
    ($msg:literal $(, $args:expr)* $(,)?) => {
        return Err($crate::BeacnError::Other(anyhow::anyhow!($msg $(, $args)*)))
    };
    // error expression form (like passing an existing error)
    ($err:expr) => {
        return Err($crate::BeacnError::Other(anyhow::Error::from($err)))
    };
}

mod sealed {
    pub trait Sealed {}
}
