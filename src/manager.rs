use anyhow::Result;
use flume::{Receiver, RecvTimeoutError, Sender, TryRecvError, bounded};
use futures_lite::stream::block_on;
use log::{debug, error, warn};
use nusb::hotplug::HotplugEvent;
use nusb::{DeviceId, DeviceInfo, MaybeFuture};
use std::cmp::PartialEq;
use std::collections::HashMap;
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

    // nusb's watch_devices() is a single cross-platform (Linux / macOS / Windows) hotplug
    // API, so unlike the old rusb-based implementation we no longer need a separate
    // libusb-hotplug-callback path and a polling fallback for platforms without it.
    thread::spawn(move || hotplug_watch(manager, receiver, sender));

    Ok(())
}

struct BeacnMicManager {
    inner: Arc<Mutex<BeacnMicManagerInner>>,
}

struct BeacnMicManagerInner {
    known_devices: HashMap<DeviceId, KnownDevice>,
    sender: Sender<HotPlugMessage>,
}

impl BeacnMicManager {
    fn new(sender: Sender<HotPlugMessage>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BeacnMicManagerInner {
                sender,
                known_devices: HashMap::new(),
            })),
        }
    }
}

impl BeacnMicManagerInner {
    fn thread_stopped(&self) {
        let _ = self.sender.send(HotPlugMessage::ThreadStopped);
    }

    fn device_connected(&mut self, device: &DeviceInfo, device_type: DeviceType) {
        let location = DeviceLocation::from(device);
        if self.known_devices.values().any(|k| k.location == location) {
            warn!("Received 'Arrived' Message for already present device!");
            return;
        }

        debug!("Device Connected at {}", location);

        // Create a health channel, this will be triggered if something goes wrong
        let (health_tx, health_rx) = bounded(1);
        self.known_devices.insert(
            device.id(),
            KnownDevice {
                location: location.clone(),
                device_type,
                health_rx,
            },
        );

        // We're actually going to sleep on this for a quarter of a second because there appears
        // to be situations where if we run through this too quickly, the udev rules may not have
        // finished being setup when we attempt to connect to the device. This results in a
        // Permission Denied error, even if we have permission!
        //
        // Shoutout to Jordahn on Discord for helping diagnose this issue.
        sleep(Duration::from_millis(250));

        let _ = self.sender.send(HotPlugMessage::DeviceAttached(
            location,
            device_type,
            health_tx,
        ));
    }

    fn device_removed(&mut self, id: DeviceId) {
        if let Some(dev) = self.known_devices.remove(&id) {
            debug!("Device Removed from {}", dev.location);
            let _ = self
                .sender
                .send(HotPlugMessage::DeviceRemoved(dev.location));
        }
    }

    fn check_device_health(&mut self) {
        for known in &mut self.known_devices.values_mut() {
            if known.health_rx.try_recv().is_ok() {
                // We're going to do a fresh enumeration to see if the device is still here,
                // this makes sure that if a device is unplugged but the removal callback
                // hasn't fired yet, we don't double-up the removal messages.
                let still_present = nusb::list_devices()
                    .wait()
                    .ok()
                    .map(|devices| {
                        devices
                            .into_iter()
                            .any(|d| DeviceLocation::from(&d) == known.location)
                    })
                    .unwrap_or(false);

                if still_present {
                    warn!(
                        "Device {} health failed, but still present, sending faux reconnect",
                        known.location
                    );

                    // The device is still present, so we'll 'fake' a disconnect / reconnect cycle
                    // so that upstream code can recreate the connection to the device.
                    let (health_tx, health_rx) = bounded(1);
                    known.health_rx = health_rx;
                    let _ = self
                        .sender
                        .send(HotPlugMessage::DeviceRemoved(known.location.clone()));

                    // Sleep for a moment, just to give things time to settle
                    sleep(Duration::from_millis(250));
                    let _ = self.sender.send(HotPlugMessage::DeviceAttached(
                        known.location.clone(),
                        known.device_type,
                        health_tx,
                    ));
                }
            }
        }
    }
}

/// Work out if a device is a Beacn device we care about, and if so what type it is.
fn identify_beacn_device(info: &DeviceInfo) -> Option<DeviceType> {
    if info.vendor_id() != VENDOR_BEACN {
        return None;
    }
    if PID_BEACN_MIC.contains(&info.product_id()) {
        Some(DeviceType::BeacnMic)
    } else if PID_BEACN_STUDIO.contains(&info.product_id()) {
        Some(DeviceType::BeacnStudio)
    } else if PID_BEACN_MIX.contains(&info.product_id()) {
        Some(DeviceType::BeacnMix)
    } else if PID_BEACN_MIX_CREATE.contains(&info.product_id()) {
        Some(DeviceType::BeacnMixCreate)
    } else {
        None
    }
}

fn hotplug_watch(
    manager: BeacnMicManager,
    receiver: Receiver<HotPlugThreadManagement>,
    sender: Sender<HotPlugMessage>,
) {
    let inner = manager.inner.clone();

    // Create the nusb watcher, and start looking for device events..
    let watch = match nusb::watch_devices() {
        Ok(watch) => watch,
        Err(e) => {
            error!("Unable to start USB hotplug watch: {}", e);
            let _ = sender.send(HotPlugMessage::ThreadStopped);
            return;
        }
    };

    // watch_devices says to populate from list_devices after it's called, so we can
    // grab and handle devices which already exist.
    if let Ok(devices) = nusb::list_devices().wait() {
        for info in devices {
            if let Some(device_type) = identify_beacn_device(&info) {
                inner.lock().unwrap().device_connected(&info, device_type);
            }
        }
    }

    // watch_devices() gives us a stream, given that we're blocking, we need a thread which can
    // pull out events and send them up to our general handler.
    let (event_tx, event_rx) = bounded(16);
    thread::spawn(move || {
        for event in block_on(watch) {
            if event_tx.send(event).is_err() {
                warn!("Hotplug Watcher: Channel Closed, stopping");
                break;
            }
        }
    });

    loop {
        let message = receiver.try_recv();
        if should_stop(message) {
            break;
        }

        match event_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(HotplugEvent::Connected(info)) => {
                if let Some(device_type) = identify_beacn_device(&info) {
                    debug!("Found Beacn Device (type {:?})", device_type);
                    inner.lock().unwrap().device_connected(&info, device_type);
                }
            }
            Ok(HotplugEvent::Disconnected(info)) => {
                inner.lock().unwrap().device_removed(info);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                error!("Hotplug watch thread has gone away, terminating hot plug Thread");
                break;
            }
        }

        inner.lock().unwrap().check_device_health();
    }

    let inner = inner.lock().unwrap();
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

#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct DeviceLocation {
    pub bus_id: String,
    pub device_address: u8,
}

impl Display for DeviceLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.bus_id, self.device_address)
    }
}

impl From<&DeviceInfo> for DeviceLocation {
    fn from(value: &DeviceInfo) -> Self {
        Self {
            bus_id: value.bus_id().to_string(),
            device_address: value.device_address(),
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

fn get_beacn_device(pid: &[u16]) -> Vec<DeviceLocation> {
    let mut devices = vec![];
    if let Ok(devs) = nusb::list_devices().wait() {
        for info in devs {
            if info.vendor_id() == VENDOR_BEACN && pid.contains(&info.product_id()) {
                devices.push(DeviceLocation::from(&info));
            }
        }
    }
    devices
}
