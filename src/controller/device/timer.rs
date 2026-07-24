use std::thread;
use std::time::Duration;
use flume::{bounded, Receiver, Sender};

// Replacement for crossbeam::channel::after
pub struct Timer {
    cancel: Sender<()>,
    rx: Receiver<()>,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        let (cancel_tx, cancel_rx) = bounded(1);
        let (tx, rx) = bounded(1);

        thread::spawn(move || {
            loop {
                let event = flume::Selector::new()
                    .recv(&cancel_rx, |_| false)
                    .wait_timeout(duration);

                match event {
                    Ok(false) => break,
                    Ok(true) => {
                        let _ = tx.send(());
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            cancel: cancel_tx,
            rx,
        }
    }

    pub fn reset(&mut self, duration: Duration) {
        let _ = self.cancel.send(());
        *self = Self::new(duration);
    }

    pub fn receiver(&self) -> &Receiver<()> {
        &self.rx
    }
}