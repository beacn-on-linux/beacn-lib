use crate::audio::common::open_beacn;
use crate::audio::{
    BeacnAudioDevice, BeacnAudioDeviceAttach, BeacnAudioMessageExecute, BeacnAudioMessaging,
    DeviceDefinition,
};
use crate::common::BeacnDeviceHandle;
use crate::manager::{DeviceType, PID_BEACN_MIC};
use anyhow::Result;
use rusb::{DeviceHandle, GlobalContext};

pub struct BeacnMic {
    handle: BeacnDeviceHandle,
}

impl BeacnAudioDeviceAttach for BeacnMic {
    fn connect(definition: DeviceDefinition) -> Result<Box<dyn BeacnAudioDevice>> {
        let handle = open_beacn(definition, PID_BEACN_MIC)?;
        Ok(Box::new(Self { handle }))
    }

    fn get_product_id(&self) -> u16 {
        PID_BEACN_MIC
    }

    fn get_serial(&self) -> String {
        self.handle.serial.clone()
    }

    fn get_version(&self) -> String {
        self.handle.version.to_string()
    }
}

impl BeacnAudioMessageExecute for BeacnMic {
    fn get_device_type(&self) -> DeviceType {
        DeviceType::BeacnMic
    }

    fn get_usb_handle(&self) -> &DeviceHandle<GlobalContext> {
        &self.handle.handle
    }
}

impl BeacnAudioMessaging for BeacnMic {}
impl BeacnAudioDevice for BeacnMic {}
