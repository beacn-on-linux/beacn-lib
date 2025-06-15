use crate::common::DeviceDefinition;
use crate::controller::common::{BeacnControlDeviceAttach, BeacnControlInteraction, open_beacn};
use crate::controller::{BeacnControlDevice, ControlThreadSender, Interactions};
use crate::manager::PID_BEACN_MIX_CREATE;
use crate::version::VersionNumber;
use crossbeam::channel::{Sender, bounded};
use log::debug;
use std::sync::mpsc;
use std::thread;

#[derive(Debug)]
pub struct BeacnMixCreate {
    serial: String,
    version: VersionNumber,

    sender: Sender<ControlThreadSender>,
}

impl BeacnControlDeviceAttach for BeacnMixCreate {
    fn connect(
        definition: DeviceDefinition,
        interaction: Option<mpsc::Sender<Interactions>>,
    ) -> anyhow::Result<Box<dyn BeacnControlDevice>>
    where
        Self: Sized,
    {
        // This handle will get sent into the main processing thread which will monitor for
        // interactions, and handle commands.
        let handle = open_beacn(definition, PID_BEACN_MIX_CREATE)?;
        let serial = handle.serial.clone();
        let version = handle.version;

        let (sender, receiver) = bounded(64);

        let control_attach = Self {
            serial,
            version,
            sender,
        };

        // Only spawn the thread if the user is interested in Interactions
        thread::spawn(|| Self::spawn_event_handler(receiver, handle, interaction));
        Ok(Box::new(control_attach))
    }

    fn get_product_id(&self) -> u16 {
        PID_BEACN_MIX_CREATE
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

impl BeacnControlDevice for BeacnMixCreate {}
impl BeacnControlInteraction for BeacnMixCreate {}

impl Drop for BeacnMixCreate {
    fn drop(&mut self) {
        debug!("Dropping BeacnMixCreate");
        let _ = self.sender.send(ControlThreadSender::Stop);
    }
}
