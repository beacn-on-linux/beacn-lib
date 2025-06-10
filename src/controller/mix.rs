use std::sync::mpsc;
use crate::common::{DeviceDefinition};
use crate::controller::common::{BeacnControlDeviceAttach, open_beacn, BeacnControlInteraction};
use crate::controller::{BeacnControlDevice, ControlThreadSender, Interactions};
use crate::manager::{PID_BEACN_MIX};
use anyhow::Result;
use std::thread;
use crossbeam::channel::{bounded, Sender};
use log::debug;
use crate::version::VersionNumber;

pub struct BeacnMix {
    serial: String,
    version: VersionNumber,

    sender: Sender<ControlThreadSender>,
}

impl BeacnControlDeviceAttach for BeacnMix {
    fn connect(
        definition: DeviceDefinition,
        interaction: Option<mpsc::Sender<Interactions>>,
    ) -> Result<Box<dyn BeacnControlDevice>>
    where
        Self: Sized,
    {
        // This handle will get sent into the main processing thread which will monitor for
        // interactions, and handle commands.
        let handle = open_beacn(definition, PID_BEACN_MIX)?;
        let serial = handle.serial.clone();
        let version = handle.version;

        let (sender, receiver) = bounded(64);

        let control_attach = Self {
            serial,
            version,
            sender,
        };

        // Only spawn the Thread if the caller is interested in Handling Interactions
        if let Some(interaction) = interaction {
            thread::spawn(|| Self::spawn_event_handler(receiver, handle, interaction));
        }
        Ok(Box::new(control_attach))
    }

    fn get_product_id(&self) -> u16 {
        PID_BEACN_MIX
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
        debug!("Dropping BeacnMixCreate");
        let _ = self.sender.send(ControlThreadSender::Stop);
    }
}
