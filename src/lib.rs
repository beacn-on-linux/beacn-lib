//pub mod device;
pub mod audio;
mod common;
pub mod controller;
pub mod manager;
pub mod types;
pub mod version;

pub use crossbeam;
pub use rusb::Error as UsbError;

use thiserror::Error;

pub type BResult<T> = Result<T, BeacnError>;

// This is a general error handler for the entire library, we might need to reexport rusb::Error
#[derive(Debug, Error)]
pub enum BeacnError {
    #[error(transparent)]
    Usb(#[from] UsbError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
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
