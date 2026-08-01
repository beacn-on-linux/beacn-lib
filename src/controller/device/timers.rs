use async_io::Timer as IoTimer;
use std::time::Duration;


pub struct Timer {
    inner: IoTimer,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Self {
            inner: IoTimer::after(duration),
        }
    }

    /// Push the deadline out to `duration` from now.
    pub fn reset(&mut self, duration: Duration) {
        self.inner = IoTimer::after(duration);
    }

    /// Resolves once the current deadline elapses.
    pub async fn wait(&mut self) {
        (&mut self.inner).await;
    }
}


pub struct Ticker {
    duration: Duration,
    inner: IoTimer,
}

impl Ticker {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            inner: IoTimer::after(duration),
        }
    }

    /// Resolves once every `duration`, forever.
    pub async fn tick(&mut self) {
        (&mut self.inner).await;
        self.inner = IoTimer::after(self.duration);
    }
}

/// Sleep is the easiest, we just call after() directly.
pub(crate) async fn sleep(duration: Duration) {
    async_io::Timer::after(duration).await;
}