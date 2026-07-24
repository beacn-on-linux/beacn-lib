use flume::{Receiver, Sender, bounded};
use std::thread;
use std::time::Duration;
use flume::select::SelectError;

// Replacement for crossbeam::channel::after
pub struct Timer {
    reset: Sender<Duration>,
    rx: Receiver<()>,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        let (reset_tx, reset_rx) = bounded(1);
        let (tx, rx) = bounded(1);

        thread::spawn(move || {
            // How long this current duration is
            let mut duration = duration;

            loop {
                // We're going to receive a reset, or a timeout, behaviour will depend on which
                let event = flume::Selector::new()
                    .recv(&reset_rx, |duration| duration)
                    .wait_timeout(duration);

                match event {
                    // Got a reset, let's set the new duration and restart the loop.
                    Ok(Ok(new_duration)) => duration = new_duration,
                    Ok(Err(_)) => break,

                    Err(SelectError::Timeout) => {
                        let _ = tx.send(());

                        // We shouldn't trigger again until we've been reset
                        match reset_rx.recv() {
                            Ok(new_duration) => duration = new_duration,
                            Err(_) => break,
                        }
                    }
                }
            }
        });

        Self {
            reset: reset_tx,
            rx,
        }
    }

    pub fn reset(&mut self, duration: Duration) {
        let _ = self.reset.send(duration);
    }

    pub fn receiver(&self) -> &Receiver<()> {
        &self.rx
    }
}