use anyhow::Result;
use log::{debug, error, warn};
use rusb::{Device, GlobalContext, Hotplug, HotplugBuilder, UsbContext, has_hotplug};
use std::cmp::PartialEq;
use std::fmt::{Display, Formatter};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

pub(crate) const VENDOR_BEACN: u16 = 0x33ae;
pub(crate) const PID_BEACN_MIC: u16 = 0x0001;
pub(crate) const PID_BEACN_STUDIO: u16 = 0x0003;

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum DeviceType {
    #[default]
    BeacnMic,
    BeacnStudio,
}

pub fn spawn_mic_hotplug_handler(
    sender: Sender<HotPlugMessage>,
    receiver: Receiver<HotPlugThreadManagement>,
) -> Result<()> {
    debug!("Spawning Beacn Mic Hot Plug Handler");

    // Create the object for managing devices
    let manager = Box::new(BeacnMicManager::new(sender.clone()));

    // Create a libusb context
    let context = GlobalContext::default();

    // Work out which type of hot plug handler we need to create
    if has_hotplug() {
        thread::spawn(move || hotplug_notify(context, manager, receiver, sender));
    } else {
        thread::spawn(move || hotplug_poll(context, *manager, receiver));
    }

    Ok(())
}

struct BeacnMicManager {
    known_devices: Vec<DeviceLocation>,
    sender: Sender<HotPlugMessage>,
}

impl BeacnMicManager {
    fn new(sender: Sender<HotPlugMessage>) -> Self {
        Self {
            sender,
            known_devices: vec![],
        }
    }

    fn thread_stopped(&self) {
        let _ = self.sender.send(HotPlugMessage::ThreadStopped);
    }

    fn device_connected(&mut self, device: DeviceLocation, device_type: DeviceType) {
        if self.known_devices.contains(&device) {
            warn!("Received 'Arrived' Message for already present device!");
            return;
        }
        debug!("Device Connected at {}", device);
        self.known_devices.push(device);
        let _ = self.sender.send(HotPlugMessage::DeviceAttached(device, device_type));
    }

    fn device_removed(&mut self, device: DeviceLocation) {
        debug!("Device Removed from {}", device);
        self.known_devices.retain(|e| e != &device);
        let _ = self.sender.send(HotPlugMessage::DeviceRemoved(device));
    }
}

impl Hotplug<GlobalContext> for BeacnMicManager {
    fn device_arrived(&mut self, device: Device<GlobalContext>) {
        let location = DeviceLocation::from(device.clone());

        // We need to work out what kind of device this is
        if let Ok(desc) = device.device_descriptor() {
            if desc.product_id() == PID_BEACN_MIC {
                debug!("Found Beacn Mic!");
                self.device_connected(location, DeviceType::BeacnMic);
            }
            if desc.product_id() == PID_BEACN_STUDIO {
                debug!("Found Beacn Studio!");
                self.device_connected(location, DeviceType::BeacnStudio);
            }
        }
    }

    fn device_left(&mut self, device: Device<GlobalContext>) {
        let device = DeviceLocation::from(device);
        self.device_removed(device);
    }
}

fn hotplug_notify(
    context: GlobalContext,
    manager: Box<BeacnMicManager>,
    receiver: Receiver<HotPlugThreadManagement>,
    sender: Sender<HotPlugMessage>,
) {
    let _handler = HotplugBuilder::new()
        .vendor_id(VENDOR_BEACN)
        .enumerate(true)
        .register(context, manager)
        .expect("Cannot Register hot plug Handler");

    let loop_duration = Some(Duration::from_millis(500));
    loop {
        let message = receiver.try_recv();
        if should_stop(message) {
            break;
        }
        context.handle_events(loop_duration).unwrap();
    }

    // We need to send this ourselves, manager has been moved into the handler
    let _ = sender.send(HotPlugMessage::ThreadStopped);
}

fn hotplug_poll(
    context: GlobalContext,
    mut manager: BeacnMicManager,
    receiver: Receiver<HotPlugThreadManagement>,
) {
    loop {
        let message = receiver.try_recv();
        if should_stop(message) {
            break;
        }

        let mut found_devices = vec![];
        if let Ok(devices) = context.devices() {
            for dev in devices.iter() {
                if let Ok(desc) = dev.device_descriptor() {
                    if desc.vendor_id() == VENDOR_BEACN {
                        let device = DeviceLocation::from(dev);

                        if desc.product_id() == PID_BEACN_MIC {
                            if !&manager.known_devices.contains(&device) {
                                found_devices.push(device);
                                manager.device_connected(device, DeviceType::BeacnMic);
                            }
                        }

                        if desc.product_id() == PID_BEACN_STUDIO {
                            if !&manager.known_devices.contains(&device) {
                                found_devices.push(device);
                                manager.device_connected(device, DeviceType::BeacnStudio);
                            }
                        }
                    }
                }
            }
        }

        // Finally, check for any device removals
        for dev in manager.known_devices.clone() {
            if !found_devices.contains(&dev) {
                manager.device_removed(dev);
            }
        }

        // We're done, sleep for now
        sleep(Duration::from_millis(500));
    }
    manager.thread_stopped();
}

fn should_stop(message: Result<HotPlugThreadManagement, TryRecvError>) -> bool {
    match message {
        Ok(message) => match message {
            HotPlugThreadManagement::Quit => true,
        },
        Err(error) => match error {
            TryRecvError::Empty => false,
            TryRecvError::Disconnected => {
                error!("Receiver has Disconnected, terminating hot plug Thread");
                true
            }
        },
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum HotPlugMessage {
    DeviceAttached(DeviceLocation, DeviceType),
    DeviceRemoved(DeviceLocation),
    ThreadStopped,
}

#[derive(PartialEq)]
pub enum HotPlugThreadManagement {
    Quit,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct DeviceLocation {
    pub bus_number: u8,
    pub address: u8,
}

impl Display for DeviceLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.bus_number, self.address)
    }
}

impl<T: UsbContext> From<Device<T>> for DeviceLocation {
    fn from(value: Device<T>) -> Self {
        Self {
            bus_number: value.bus_number(),
            address: value.address(),
        }
    }
}

/// This is a generic function that will just return a list of Beacn Mic's attached to your
/// system for situations where you want to handle hot plugging yourself.
///
/// This function is useful during prototyping, but shouldn't be used long term, instead
/// use the regular hot plug thread.
pub fn get_beacn_mic_devices() -> Vec<DeviceLocation> {
    get_beacn_device(PID_BEACN_MIC)
}

pub fn get_beacn_studio_devices() -> Vec<DeviceLocation> {
    get_beacn_device(PID_BEACN_STUDIO)
}

fn get_beacn_device(pid: u16) -> Vec<DeviceLocation> {
    let mut devices = vec![];
    if let Ok(devs) = rusb::devices() {
        for dev in devs.iter() {
            if let Ok(desc) = dev.device_descriptor() {
                if desc.vendor_id() == VENDOR_BEACN && desc.product_id() == pid {
                    devices.push(DeviceLocation::from(dev));
                }
            }
        }
    }
    devices
}