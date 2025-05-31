use crate::audio::common::{open_beacn, BeacnDeviceHandle};
use crate::audio::{
    BeacnAudioDevice, BeacnAudioDeviceAttach, BeacnAudioMessageExecute, BeacnAudioMessaging,
    DeviceDefinition,
};
use crate::manager::{DeviceType, PID_BEACN_STUDIO};
use anyhow::Result;
use rusb::{DeviceHandle, GlobalContext};

pub struct BeacnStudio {
    handle: BeacnDeviceHandle,
}

impl BeacnAudioDeviceAttach for BeacnStudio {
    fn connect(definition: DeviceDefinition) -> Result<Box<dyn BeacnAudioDevice>> {
        let handle = open_beacn(definition, PID_BEACN_STUDIO)?;

        // TODO: Spawn Thread to manage inputs
        Ok(Box::new(Self { handle }))
    }

    fn get_product_id(&self) -> u16 {
        PID_BEACN_STUDIO
    }

    fn get_serial(&self) -> String {
        self.handle.serial.clone()
    }

    fn get_version(&self) -> String {
        self.handle.version.to_string()
    }
}

impl BeacnAudioMessageExecute for BeacnStudio {
    fn get_device_type(&self) -> DeviceType {
        DeviceType::BeacnStudio
    }

    fn get_usb_handle(&self) -> &DeviceHandle<GlobalContext> {
        &self.handle.handle
    }
}

impl BeacnAudioMessaging for BeacnStudio {}
impl BeacnAudioDevice for BeacnStudio {}
