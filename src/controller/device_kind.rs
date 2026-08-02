use crate::BResult;
use crate::common::{BeacnDeviceInfo, BeacnDeviceKind, DeviceDefinition, open_device};
use crate::controller::common::{
    BeacnControlAPI, BeacnControlDeviceInfo, BeacnControlDeviceInternal,
};
use crate::controller::device::runner::BeacnControlDeviceRunner;
use crate::controller::{BeacnControlDevice, ControlThreadSender, Interactions};
use crate::manager::DeviceType;
use crate::sealed::Sealed;
use crate::timers::sleep;
use crate::version::VersionNumber;
use anyhow::Result;
use anyhow::bail;
use async_trait::async_trait;
use flume::{Sender, bounded};
use log::debug;
use nusb::transfer::Interrupt;
use std::marker::PhantomData;
use std::panic::RefUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use web_time::Duration;

#[derive(Debug)]
pub(crate) struct BeacnDevice<K: BeacnDeviceKind> {
    pid: u16,
    serial: String,
    fw_version: VersionNumber,

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
        self.fw_version
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
        let handle = open_device::<Interrupt>(K::PID, definition, 0, &[0x00, 0x01]).await?;
        let (sender, receiver) = bounded(64);

        let control_attach = Self {
            pid: handle.descriptor.product_id(),
            serial: handle.serial.clone(),
            fw_version: handle.fw_version,
            sender: sender.clone(),
            sender_enabled: AtomicBool::new(false),
            _kind: PhantomData,
        };
        let control: Arc<Box<dyn BeacnControlDevice>> = Arc::new(Box::new(control_attach));
        let control_inner = control.clone();

        spawn_background(K::TYPE, async move {
            debug!("Starting {} control thread", K::TYPE);
            Self::spawn_event_handler(control_inner, receiver, handle, interaction).await;
            debug!("{} control thread exited", K::TYPE);
            sleep(Duration::from_millis(500)).await;
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
        debug!("Dropping {}", K::TYPE);
        let _ = self.sender.send(ControlThreadSender::Stop);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_background<F>(kind: DeviceType, future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    #[cfg(feature = "tokio")]
    {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(future);
            return;
        }
    }

    let device_type = match kind {
        DeviceType::BeacnMixCreate => "mix-create",
        DeviceType::BeacnMix => "mix",
        _ => unreachable!(),
    };


    // If we're not already inside a supported runtime, create an async-io context.
    #[cfg(not(target_arch = "wasm32"))]
    {
        debug!("Spawning background thread for {}", device_type);
        let name = format!("{}-task", device_type);
        thread::Builder::new()
            .name(name)
            .spawn(move || {
                async_io::block_on(future);
            })
            .expect("failed to spawn background thread");
    }
}

// Split wasm off completely as it has a different, incompatible, return type
#[cfg(target_arch = "wasm32")]
fn spawn_background<F>(_: DeviceType, future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}
