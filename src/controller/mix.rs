use crate::common::{BeacnDeviceHandle, DeviceDefinition};
use crate::controller::BeacnControlDevice;
use crate::controller::common::{BeacnControlDeviceAttach, open_beacn};
use crate::manager::PID_BEACN_MIX;
use anyhow::Result;

pub struct BeacnMix {
    handle: BeacnDeviceHandle,
}

impl BeacnControlDeviceAttach for BeacnMix {
    fn connect(definition: DeviceDefinition) -> Result<Box<dyn BeacnControlDevice>>
    where
        Self: Sized,
    {
        let handle = open_beacn(definition, PID_BEACN_MIX)?;
        // TODO: Spawn Thread to manage inputs
        Ok(Box::new(Self { handle }))
    }

    fn get_product_id(&self) -> u16 {
        PID_BEACN_MIX
    }

    fn get_serial(&self) -> String {
        self.handle.serial.clone()
    }

    fn get_version(&self) -> String {
        self.handle.version.to_string()
    }
}

impl BeacnControlDevice for BeacnMix {}
