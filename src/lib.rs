//pub mod device;
pub mod audio;
mod common;
pub mod controller;
pub mod manager;
pub mod types;
pub mod version;

pub use flume;
use log::debug;
pub use nusb::ErrorKind as UsbError;
pub use nusb::transfer::TransferError as UsbTransferError;

use crate::version::VersionNumber;
use thiserror::Error;

// These are some helper versions, which can be used to determine feature availability
const MIC_CLASS_COMPLIANT_VERSION: VersionNumber = VersionNumber(1, 2, 0, 188);

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
