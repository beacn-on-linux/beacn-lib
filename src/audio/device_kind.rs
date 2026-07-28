use crate::BResult;
use crate::audio::BeacnAudioDevice;
use crate::audio::common::{
    AudioEndpoints, BeacnAudioDeviceInternal, BeacnAudioMessageExecute, BeacnAudioMessageLocal,
    BeacnAudioMessaging,
};
use crate::common::{BeacnDeviceHandle, BeacnDeviceInfo, DeviceDefinition};
use crate::manager::DeviceType;
use crate::sealed::Sealed;
use crate::sync::AsyncMutex;
use crate::version::VersionNumber;
use async_trait::async_trait;
use log::debug;
use std::marker::PhantomData;
use std::panic::RefUnwindSafe;

/// Supplies the bits that differ between physical device types.
pub trait BeacnDeviceKind: Send + Sync + 'static {
    const PID: &[u16];
    const NAME: &'static str;
}

pub(crate) struct BeacnDevice<K: BeacnDeviceKind> {
    handle: BeacnDeviceHandle,
    endpoints: AsyncMutex<AudioEndpoints>,

    _kind: PhantomData<K>,
}

impl<K: BeacnDeviceKind + RefUnwindSafe> Sealed for BeacnDevice<K> {}

#[async_trait]
impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnDeviceInfo for BeacnDevice<K> {
    fn get_product_id(&self) -> u16 {
        self.handle.descriptor.product_id()
    }

    fn get_serial(&self) -> String {
        self.handle.serial.clone()
    }

    fn get_version(&self) -> VersionNumber {
        self.handle.version
    }
}

#[async_trait]
impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnAudioDeviceInternal for BeacnDevice<K> {
    async fn connect(definition: DeviceDefinition) -> BResult<Box<dyn BeacnAudioDevice>>
    where
        Self: Sized,
    {
        let (handle, endpoints) = crate::audio::common::open_beacn(definition, K::PID).await?;
        Ok(Box::new(Self {
            handle,
            endpoints: AsyncMutex::new(endpoints),

            _kind: PhantomData,
        }))
    }
}

impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnAudioDevice for BeacnDevice<K> {}
impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnAudioMessageLocal for BeacnDevice<K> {}
impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnAudioMessaging for BeacnDevice<K> {}
impl<K: BeacnDeviceKind + RefUnwindSafe> BeacnAudioMessageExecute for BeacnDevice<K> {
    fn get_device_type(&self) -> DeviceType {
        DeviceType::BeacnStudio
    }

    fn get_endpoints(&self) -> &AsyncMutex<AudioEndpoints> {
        &self.endpoints
    }
}

impl<K: BeacnDeviceKind> Drop for BeacnDevice<K> {
    fn drop(&mut self) {
        debug!("Dropping {}", K::NAME);
    }
}
