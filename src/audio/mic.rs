use crate::BResult;
use crate::audio::common::{AudioEndpoints, BeacnAudioMessageLocal, open_beacn};
use crate::audio::{
    BeacnAudioDevice, BeacnAudioDeviceAttach, BeacnAudioMessageExecute, BeacnAudioMessaging,
    DeviceDefinition,
};
use crate::common::BeacnDeviceHandle;
use crate::manager::{DeviceType, PID_BEACN_MIC};
use crate::version::VersionNumber;
use std::sync::Mutex;

pub struct BeacnMic {
    handle: BeacnDeviceHandle,
    endpoints: Mutex<AudioEndpoints>,
}

impl BeacnAudioDeviceAttach for BeacnMic {
    fn connect(definition: DeviceDefinition) -> BResult<Box<dyn BeacnAudioDevice>> {
        let (handle, endpoints) = open_beacn(definition, PID_BEACN_MIC)?;
        Ok(Box::new(Self {
            handle,
            endpoints: Mutex::new(endpoints),
        }))
    }

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

impl BeacnAudioMessageExecute for BeacnMic {
    fn get_device_type(&self) -> DeviceType {
        DeviceType::BeacnMic
    }

    fn get_endpoints(&self) -> &Mutex<AudioEndpoints> {
        &self.endpoints
    }
}

impl BeacnAudioMessaging for BeacnMic {}
impl BeacnAudioMessageLocal for BeacnMic {}
impl BeacnAudioDevice for BeacnMic {}
