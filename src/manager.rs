use anyhow::Result;
use crossbeam::channel::{Receiver, Sender, TryRecvError, bounded};
use log::{debug, error, warn};
use rusb::{Device, GlobalContext, Hotplug, HotplugBuilder, UsbContext, has_hotplug};
use std::cmp::PartialEq;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

pub(crate) const VENDOR_BEACN: u16 = 0x33ae;
pub(crate) const PID_BEACN_MIC: &[u16] = &[0x0001, 0x8001];
pub(crate) const PID_BEACN_STUDIO: &[u16] = &[0x0003];
pub(crate) const PID_BEACN_MIX: &[u16] = &[0x0004];
pub(crate) const PID_BEACN_MIX_CREATE: &[u16] = &[0x0007];

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceType {
    #[default]
    BeacnMic,
    BeacnStudio,
    BeacnMix,
    BeacnMixCreate,
}

struct KnownDevice {
    location: DeviceLocation,
    device_type: DeviceType,
    health_rx: Receiver<()>,
}

pub fn spawn_hotplug_handler(
    sender: Sender<HotPlugMessage>,
    receiver: Receiver<HotPlugThreadManagement>,
) -> Result<()> {
    debug!("Spawning Beacn Mic Hot Plug Handler");

    // Create the object for managing devices
    let manager = BeacnMicManager::new(sender.clone());

    // Create a libusb context
    let context = GlobalContext::default();

    // Work out which type of hot plug handler we need to create
    if has_hotplug() {
        thread::spawn(move || hotplug_notify(context, manager, receiver, sender));
    } else {
        thread::spawn(move || hotplug_poll(context, manager, receiver));
    }

    Ok(())
}

struct BeacnMicManager {
    inner: Arc<Mutex<BeacnMicManagerInner>>,
}

struct BeacnMicManagerInner {
    known_devices: Vec<KnownDevice>,
    sender: Sender<HotPlugMessage>,
}

impl BeacnMicManager {
    fn new(sender: Sender<HotPlugMessage>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BeacnMicManagerInner {
                sender,
                known_devices: vec![],
            })),
        }
    }
}

impl BeacnMicManagerInner {
    fn thread_stopped(&self) {
        let _ = self.sender.send(HotPlugMessage::ThreadStopped);
    }

    fn device_connected(&mut self, device: DeviceLocation, device_type: DeviceType) {
        //let mut inner = self.inner.lock().unwrap();

        if self.known_devices.iter().any(|k| k.location == device) {
            warn!("Received 'Arrived' Message for already present device!");
            return;
        }

        debug!("Device Connected at {}", device);

        // Create a health channel, this will be triggered if something goes wrong
        let (health_tx, health_rx) = bounded(1);
        self.known_devices.push(KnownDevice {
            location: device,
            device_type,
            health_rx,
        });

        // We're actually going to sleep on this for a quarter of a second because there appears
        // to be situations where if we run through this too quickly, the udev rules may not have
        // finished being setup when we attempt to connect to the device. This results in a
        // Permission Denied error, even if we have permission!
        //
        // Shoutout to Jordahn on Discord for helping diagnose this issue.
        sleep(Duration::from_millis(250));

        let _ = self.sender.send(HotPlugMessage::DeviceAttached(
            device,
            device_type,
            health_tx,
        ));
    }

    fn device_removed(&mut self, device: DeviceLocation) {
        debug!("Device Removed from {}", device);
        self.known_devices.retain(|e| e.location != device);
        let _ = self.sender.send(HotPlugMessage::DeviceRemoved(device));
    }

    fn check_device_health(&mut self) {
        for known in &mut self.known_devices {
            if known.health_rx.try_recv().is_ok() {

                // We're going to do a rusb iteration to see if the device is still here, this
                // makes sure that if a device is unplugged but the removal callback hasn't fired
                // yet, we don't double-up the removal messages.
                let still_present = rusb::devices().ok()
                    .map(|devices| devices.iter().any(|d| DeviceLocation::from(d) == known.location))
                    .unwrap_or(false);

                if still_present {
                    // The device is still present, so we'll 'fake' a disconnect / reconnect cycle
                    // so that upstream code can recreate the connection to the device.
                    let (health_tx, health_rx) = bounded(1);
                    known.health_rx = health_rx;
                    let _ = self.sender.send(HotPlugMessage::DeviceRemoved(known.location));

                    // Sleep for a moment, just to give things time to settle
                    sleep(Duration::from_millis(250));
                    let _ = self.sender.send(HotPlugMessage::DeviceAttached(
                        known.location,
                        known.device_type,
                        health_tx,
                    ));
                }
            }
        }
    }
}

impl Hotplug<GlobalContext> for BeacnMicManager {
    fn device_arrived(&mut self, device: Device<GlobalContext>) {
        let location = DeviceLocation::from(device.clone());

        let mut inner = self.inner.lock().unwrap();

        // We need to work out what kind of device this is
        if let Ok(desc) = device.device_descriptor() {
            if PID_BEACN_MIC.contains(&desc.product_id()) {
                debug!("Found Beacn Mic!");
                inner.device_connected(location, DeviceType::BeacnMic);
            }
            if PID_BEACN_STUDIO.contains(&desc.product_id()) {
                debug!("Found Beacn Studio!");
                inner.device_connected(location, DeviceType::BeacnStudio);
            }
            if PID_BEACN_MIX.contains(&desc.product_id()) {
                debug!("Found Beacn Mix!");
                inner.device_connected(location, DeviceType::BeacnMix)
            }
            if PID_BEACN_MIX_CREATE.contains(&desc.product_id()) {
                debug!("Found Beacn Mix Create!");
                inner.device_connected(location, DeviceType::BeacnMixCreate)
            }
        }
    }

    #[allow(clippy::collapsible_if)]
    fn device_left(&mut self, device: Device<GlobalContext>) {
        // Only flag a device removal if it's a Mic or Studio
        if let Ok(desc) = device.device_descriptor() {
            if PID_BEACN_MIC.contains(&desc.product_id())
                || PID_BEACN_STUDIO.contains(&desc.product_id())
                || PID_BEACN_MIX.contains(&desc.product_id())
                || PID_BEACN_MIX_CREATE.contains(&desc.product_id())
            {
                let location = DeviceLocation::from(device.clone());
                self.inner.lock().unwrap().device_removed(location);
            }
        }
    }
}

fn hotplug_notify(
    context: GlobalContext,
    manager: BeacnMicManager,
    receiver: Receiver<HotPlugThreadManagement>,
    sender: Sender<HotPlugMessage>,
) {
    let inner = manager.inner.clone();

    let _handler = HotplugBuilder::new()
        .vendor_id(VENDOR_BEACN)
        .enumerate(true)
        .register(context, Box::new(manager))
        .expect("Cannot Register hot plug Handler");

    let loop_duration = Some(Duration::from_millis(100));
    loop {
        let message = receiver.try_recv();
        if should_stop(message) {
            break;
        }

        inner.lock().unwrap().check_device_health();
        context.handle_events(loop_duration).unwrap();
    }

    // We need to send this ourselves, manager has been moved into the handler
    let _ = sender.send(HotPlugMessage::ThreadStopped);
}

fn hotplug_poll(
    context: GlobalContext,
    manager: BeacnMicManager,
    receiver: Receiver<HotPlugThreadManagement>,
) {
    loop {
        let message = receiver.try_recv();
        if should_stop(message) {
            break;
        }

        let mut inner = manager.inner.lock().unwrap();

        let mut found_devices = vec![];
        if let Ok(devices) = context.devices() {
            for dev in devices.iter() {
                #[allow(clippy::collapsible_if)]
                if let Ok(desc) = dev.device_descriptor() {
                    if desc.vendor_id() == VENDOR_BEACN {
                        let device = DeviceLocation::from(dev);

                        #[allow(clippy::collapsible_if)]
                        if PID_BEACN_MIC.contains(&desc.product_id()) {
                            if !inner.known_devices.iter().any(|k| k.location == device) {
                                found_devices.push(device);
                                inner.device_connected(device, DeviceType::BeacnMic);
                            }
                        }

                        #[allow(clippy::collapsible_if)]
                        if PID_BEACN_STUDIO.contains(&desc.product_id()) {
                            if !inner.known_devices.iter().any(|k| k.location == device) {
                                found_devices.push(device);
                                inner.device_connected(device, DeviceType::BeacnStudio);
                            }
                        }

                        #[allow(clippy::collapsible_if)]
                        if PID_BEACN_MIX.contains(&desc.product_id()) {
                            if !inner.known_devices.iter().any(|k| k.location == device) {
                                found_devices.push(device);
                                inner.device_connected(device, DeviceType::BeacnMix);
                            }
                        }

                        #[allow(clippy::collapsible_if)]
                        if PID_BEACN_MIX_CREATE.contains(&desc.product_id()) {
                            if !inner.known_devices.iter().any(|k| k.location == device) {
                                found_devices.push(device);
                                inner.device_connected(device, DeviceType::BeacnMixCreate);
                            }
                        }
                    }
                }
            }
        }

        let known_locations: Vec<DeviceLocation> =
            inner.known_devices.iter().map(|k| k.location).collect();
        for location in known_locations {
            if !found_devices.contains(&location) {
                inner.device_removed(location);
            }
        }

        // We're done, sleep for now
        inner.check_device_health();
        sleep(Duration::from_millis(100));
    }

    let inner = manager.inner.lock().unwrap();
    inner.thread_stopped();
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

#[derive(Debug, Clone)]
pub enum HotPlugMessage {
    DeviceAttached(DeviceLocation, DeviceType, Sender<()>),
    DeviceRemoved(DeviceLocation),
    ThreadStopped,
}

#[derive(PartialEq)]
pub enum HotPlugThreadManagement {
    Quit,
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
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

pub fn get_beacn_mix_device() -> Vec<DeviceLocation> {
    get_beacn_device(PID_BEACN_MIX)
}

pub fn get_beacn_mix_create_device() -> Vec<DeviceLocation> {
    get_beacn_device(PID_BEACN_MIX_CREATE)
}

#[allow(clippy::collapsible_if)]
fn get_beacn_device(pid: &[u16]) -> Vec<DeviceLocation> {
    let mut devices = vec![];
    if let Ok(devs) = rusb::devices() {
        for dev in devs.iter() {
            if let Ok(desc) = dev.device_descriptor() {
                if desc.vendor_id() == VENDOR_BEACN && pid.contains(&desc.product_id()) {
                    devices.push(DeviceLocation::from(dev));
                }
            }
        }
    }
    devices
}
