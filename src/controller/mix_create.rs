use crate::common::DeviceDefinition;
use crate::controller::common::{open_beacn, BeacnControlDeviceAttach, BeacnControlInteraction};
use crate::controller::{BeacnControlDevice, ControlThreadManager, Interactions};
use crate::manager::PID_BEACN_MIX_CREATE;
use crate::version::VersionNumber;
use crossbeam::channel::{bounded, Sender};
use log::debug;
use std::sync::mpsc;
use std::thread;

#[derive(Debug)]
pub struct BeacnMixCreate {
    serial: String,
    version: VersionNumber,

    sender: Sender<ControlThreadManager>,
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
        if let Some(interaction) = interaction {
            thread::spawn(|| Self::spawn_event_handler(receiver, handle, interaction));
        }

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
}

impl BeacnControlDevice for BeacnMixCreate {}
impl BeacnControlInteraction for BeacnMixCreate {}

impl Drop for BeacnMixCreate {
    fn drop(&mut self) {
        debug!("Dropping BeacnMixCreate");
        let _ = self.sender.send(ControlThreadManager::STOP);
    }
}
