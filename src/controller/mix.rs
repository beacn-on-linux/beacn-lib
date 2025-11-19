use crate::BResult;
use crate::common::DeviceDefinition;
use crate::controller::common::{BeacnControlDeviceAttach, BeacnControlInteraction, open_beacn};
use crate::controller::{BeacnControlDevice, ControlThreadSender, Interactions};
use crate::manager::PID_BEACN_MIX;
use crate::version::VersionNumber;
use crossbeam::channel::{Sender, bounded};
use log::debug;
use std::thread;

pub struct BeacnMix {
    pid: u16,

    serial: String,
    version: VersionNumber,

    sender: Sender<ControlThreadSender>,
}

impl BeacnControlDeviceAttach for BeacnMix {
    fn connect(
        definition: DeviceDefinition,
        interaction: Option<Sender<Interactions>>,
    ) -> BResult<Box<dyn BeacnControlDevice>>
    where
        Self: Sized,
    {
        // This handle will get sent into the main processing thread which will monitor for
        // interactions, and handle commands.
        let handle = open_beacn(definition, PID_BEACN_MIX)?;
        let serial = handle.serial.clone();
        let version = handle.version;
        let pid = handle.descriptor.product_id();

        let (sender, receiver) = bounded(64);

        let control_attach = Self {
            pid,
            serial,
            version,
            sender,
        };

        thread::spawn(|| Self::spawn_event_handler(receiver, handle, interaction));
        Ok(Box::new(control_attach))
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

    fn get_sender(&self) -> &Sender<ControlThreadSender> {
        &self.sender
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
