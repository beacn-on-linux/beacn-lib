use std::ops::Deref;
use std::panic::RefUnwindSafe;

/// Thin wrapper around `async_lock::Mutex` that restores the same *unconditional*
/// `RefUnwindSafe` guarantee `std::sync::Mutex<T>` provides regardless of `T`.
pub(crate) struct AsyncMutex<T>(async_lock::Mutex<T>);
impl<T> AsyncMutex<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(async_lock::Mutex::new(value))
    }
}

impl<T> Deref for AsyncMutex<T> {
    type Target = async_lock::Mutex<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> RefUnwindSafe for AsyncMutex<T> {}
