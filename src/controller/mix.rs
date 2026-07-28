use crate::BResult;
use crate::common::DeviceDefinition;
use crate::controller::common::{BeacnControlDeviceAttach, BeacnControlInteraction, open_beacn};
use crate::controller::{BeacnControlDevice, ControlThreadSender, Interactions};
use crate::manager::PID_BEACN_MIX;
use crate::version::VersionNumber;
use anyhow::{Result, bail};
use async_trait::async_trait;
use flume::{Sender, bounded};
use log::debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

pub struct BeacnMix {
    pid: u16,

    serial: String,
    version: VersionNumber,

    sender: Sender<ControlThreadSender>,
    sender_enabled: AtomicBool,
}

#[async_trait]
impl BeacnControlDeviceAttach for BeacnMix {
    async fn connect(
        definition: DeviceDefinition,
        interaction: Option<Sender<Interactions>>,
        health_tx: Sender<()>,
    ) -> BResult<Arc<Box<dyn BeacnControlDevice>>>
    where
        Self: Sized,
    {
        // This handle will get sent into the main processing thread which will monitor for
        // interactions, and handle commands.
        let handle = open_beacn(definition, PID_BEACN_MIX).await?;
        let serial = handle.serial.clone();
        let version = handle.version;
        let pid = handle.descriptor.product_id();

        let (sender, receiver) = bounded(64);

        let control_attach = Self {
            pid,
            serial,
            version,
            sender,

            sender_enabled: AtomicBool::new(false),
        };

        let control: Arc<Box<dyn BeacnControlDevice>> = Arc::new(Box::new(control_attach));
        let control_inner = control.clone();

        thread::spawn(move || {
            Self::spawn_event_handler(control_inner, receiver, handle, interaction);
            sleep(Duration::from_millis(500));
            let _ = health_tx.send(());
        });

        Ok(control)
    }

    fn get_product_id(&self) -> u16 {
        self.pid
    }

    fn get_serial(&self) -> String {
        self.serial.clone()
    }

    fn get_version(&self) -> String {
        self.version.to_string()
    }

    fn get_sender(&self) -> Result<&Sender<ControlThreadSender>> {
        if self.sender.is_disconnected() || !self.sender_enabled.load(Ordering::Relaxed) {
            bail!("Sender is disconnected!");
        }
        Ok(&self.sender)
    }

    fn set_sender_enabled(&self, enabled: bool) {
        self.sender_enabled.store(enabled, Ordering::Relaxed);
    }

    fn get_display_size(&self) -> (u32, u32) {
        (800, 480)
    }
}

impl BeacnControlDevice for BeacnMix {}
impl BeacnControlInteraction for BeacnMix {}

impl Drop for BeacnMix {
    fn drop(&mut self) {
        debug!("Dropping BeacnMix");
        let _ = self.sender.send(ControlThreadSender::Stop);
    }
}
