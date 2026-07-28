use crate::BResult;
use crate::common::{BeacnDeviceInfo, DeviceDefinition};
use crate::controller::common::{
    BeacnControlAPI, BeacnControlDeviceInfo, BeacnControlDeviceInternal, open_beacn,
};
use crate::controller::device::runner::BeacnControlDeviceRunner;
use crate::controller::{BeacnControlDevice, ControlThreadSender, Interactions};
use crate::sealed::Sealed;
use crate::version::VersionNumber;
use anyhow::Result;
use anyhow::bail;
use async_trait::async_trait;
use flume::{Sender, bounded};
use log::debug;
use std::marker::PhantomData;
use std::panic::RefUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

/// Supplies the bits that differ between physical device types.
pub trait BeacnDeviceKind: Send + Sync + 'static {
    const PID: &[u16];
    const NAME: &'static str;
}

#[derive(Debug)]
pub(crate) struct BeacnDevice<K: BeacnDeviceKind> {
    pid: u16,
    serial: String,
    version: VersionNumber,

    sender: Sender<ControlThreadSender>,
    sender_enabled: AtomicBool,

    _kind: PhantomData<K>,
}

impl<K: BeacnDeviceKind + RefUnwindSafe> Sealed for BeacnDevice<K> {}

#[async_trait]
impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnControlDeviceInfo for BeacnDevice<K> {
    fn get_display_size(&self) -> (u32, u32) {
        (800, 480)
    }
}

impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnDeviceInfo for BeacnDevice<K> {
    fn get_product_id(&self) -> u16 {
        self.pid
    }
    fn get_serial(&self) -> String {
        self.serial.clone()
    }
    fn get_version(&self) -> VersionNumber {
        self.version
    }
}

impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnControlDeviceInternal for BeacnDevice<K> {
    async fn connect(
        definition: DeviceDefinition,
        interaction: Option<Sender<Interactions>>,
        health_tx: Sender<()>,
    ) -> BResult<Arc<Box<dyn BeacnControlDevice>>>
    where
        Self: Sized,
    {
        let handle = open_beacn(definition, K::PID).await?;
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
            _kind: PhantomData,
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

    fn get_sender(&self) -> Result<&Sender<ControlThreadSender>> {
        if self.sender.is_disconnected() || !self.sender_enabled.load(Ordering::Relaxed) {
            bail!("Sender is disconnected!");
        }
        Ok(&self.sender)
    }

    fn set_sender_enabled(&self, enabled: bool) {
        self.sender_enabled.store(enabled, Ordering::Relaxed);
    }
}

impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnControlDevice for BeacnDevice<K> {}
impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnControlDeviceRunner for BeacnDevice<K> {}
impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnControlAPI for BeacnDevice<K> {}

impl<K: BeacnDeviceKind> Drop for BeacnDevice<K> {
    fn drop(&mut self) {
        debug!("Dropping {}", K::NAME);
        let _ = self.sender.send(ControlThreadSender::Stop);
    }
}
