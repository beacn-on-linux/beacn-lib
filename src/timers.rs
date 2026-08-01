use std::time::{Duration, Instant};

pub struct Timer {
    deadline: Instant,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Self {
            deadline: Instant::now() + duration,
        }
    }

    /// Push the deadline out to `duration` from now.
    pub fn reset(&mut self, duration: Duration) {
        self.deadline = Instant::now() + duration;
    }

    /// Resolves once the current deadline elapses.
    pub async fn wait(&mut self) {
        // This is basically for if wait gets called a small amount of time after reset / new
        sleep(self.deadline.saturating_duration_since(Instant::now())).await;
    }
}

pub struct Ticker {
    duration: Duration,
    deadline: Instant,
    fixed_rate: bool,
}

impl Ticker {
    pub fn new(duration: Duration, fixed_rate: bool) -> Self {
        Self {
            duration,
            deadline: Instant::now() + duration,
            fixed_rate,
        }
    }

    /// Resolves once every `duration`, forever.
    pub async fn tick(&mut self) {
        let wait = self.deadline.saturating_duration_since(Instant::now());
        sleep(wait).await;

        if self.fixed_rate {
            self.deadline += self.duration;
        } else {
            self.deadline = Instant::now() + self.duration;
        }
    }
}

/// This is a runtime agnostic sleep function, it'll use tokio if inside a tokio runtime, otherwise
/// it'll fall back to asyncio
pub(crate) async fn sleep(duration: Duration) {
    #[cfg(feature = "tokio")]
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::time::sleep(duration).await;
            return;
        }
    }

    async_io::Timer::after(duration).await;
}