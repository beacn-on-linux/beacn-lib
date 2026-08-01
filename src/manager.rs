use anyhow::Result;
use flume::{Receiver, Sender, bounded};
use futures_lite::StreamExt;
use futures_lite::future::or;
use log::{debug, error, warn};
use nusb::hotplug::HotplugEvent;
use nusb::{DeviceId, DeviceInfo};
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use strum::Display;
use crate::timers::{sleep, Ticker};

pub(crate) const VENDOR_BEACN: u16 = 0x33ae;
pub(crate) const PID_BEACN_MIC: &[u16] = &[0x0001, 0x8001];
pub(crate) const PID_BEACN_STUDIO: &[u16] = &[0x0003];
pub(crate) const PID_BEACN_MIX: &[u16] = &[0x0004];
pub(crate) const PID_BEACN_MIX_CREATE: &[u16] = &[0x0007];

#[derive(Debug, Display, Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceType {
    #[default]
    BeacnMic,
    BeacnStudio,
    BeacnMixCreate,
    BeacnMix,
}

struct KnownDevice {
    location: DeviceLocation,
    device_type: DeviceType,
    health_rx: Receiver<()>,
}

struct HotPlugManager {
    known_devices: HashMap<DeviceId, KnownDevice>,
    sender: Sender<HotPlugMessage>,
}

impl HotPlugManager {
    fn thread_stopped(&self) {
        let _ = self.sender.send(HotPlugMessage::ThreadStopped);
    }

    async fn device_connected(&mut self, device: &DeviceInfo, device_type: DeviceType) {
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

        // We're actually going to wait on this for a quarter of a second because there
        // appears to be situations where if we run through this too quickly, the udev
        // rules may not have finished being setup when we attempt to connect to the
        // device. This results in a Permission Denied error, even if we have permission!
        //
        // Shoutout to Jordahn on Discord for helping diagnose this issue.
        sleep(Duration::from_millis(250)).await;

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

    async fn check_device_health(&mut self) {
        // Check the health reciever of all devices for pings, indicating that the device messaging
        // has failed.
        let failing: Vec<(DeviceLocation, DeviceType)> = self
            .known_devices
            .values_mut()
            .filter(|known| known.health_rx.try_recv().is_ok())
            .map(|known| (known.location.clone(), known.device_type))
            .collect();

        for (location, device_type) in failing {
            // We're going to do a fresh enumeration to see if the device is still here,
            // this makes sure that if a device is unplugged but the removal callback
            // hasn't fired yet, we don't double-up the removal messages.
            let still_present = crate::setup::list_devices()
                .await
                .ok()
                .map(|devices| {
                    devices
                        .into_iter()
                        .any(|d| DeviceLocation::from(&d) == location)
                })
                .unwrap_or(false);

            if !still_present {
                continue;
            }

            warn!(
                "Device {} health failed, but still present, sending faux reconnect",
                location
            );

            // The device is still present, so we'll 'fake' a disconnect / reconnect cycle
            // so that upstream code can recreate the connection to the device.
            let (health_tx, health_rx) = bounded(1);
            if let Some(known) = self
                .known_devices
                .values_mut()
                .find(|k| k.location == location)
            {
                known.health_rx = health_rx;
            }
            let _ = self
                .sender
                .send(HotPlugMessage::DeviceRemoved(location.clone()));

            // Wait a moment, just to give things time to settle
            sleep(Duration::from_millis(250)).await;
            let _ = self.sender.send(HotPlugMessage::DeviceAttached(
                location,
                device_type,
                health_tx,
            ));
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

enum HotplugLoopEvent {
    Management(Result<HotPlugThreadManagement, flume::RecvError>),
    Hotplug(Option<HotplugEvent>),
    HealthCheck,
}

/// Spawn an OS thread and Watch for Beacn device hot-plug events and report them on `sender`.
///
/// If you're running in an async context, instead take a look at `watch_hotplug_devices` which
/// can instead be used directly in your runtime.
///
/// Runs until `receiver` gets `HotPlugThreadManagement::Quit`, `sender`'s corresponding
/// receiver is dropped, or the underlying hotplug watch itself fails.
pub fn spawn_hotplug_handler(
    sender: Sender<HotPlugMessage>,
    receiver: Receiver<HotPlugThreadManagement>,
) -> JoinHandle<()> {
    debug!("Running Beacn Mic Hot Plug Handler");

    use crate::MaybeFuture;
    thread::spawn(|| watch_hotplug_devices(sender, receiver).wait())
}

/// Watch for Beacn device hot-plug events and report them on `sender`, without spawning
/// any OS thread of our own.
///
/// Await this directly from your own async runtime (or hand it to `tokio::spawn` /
/// `smol::spawn` / etc. to run it in the background), or drive it with `.wait()`
/// (`beacn_lib::MaybeFuture`) to block the current thread for as long as you want to
/// watch. `spawn_hotplug_handler` is a thin convenience wrapper around the latter, for
/// classic thread-per-task usage where you don't want to manage the thread yourself.
///
/// Runs until `receiver` gets `HotPlugThreadManagement::Quit`, `sender`'s corresponding
/// receiver is dropped, or the underlying hotplug watch itself fails.
pub async fn watch_hotplug_devices(
    sender: Sender<HotPlugMessage>,
    receiver: Receiver<HotPlugThreadManagement>,
) {
    let mut inner = HotPlugManager {
        sender: sender.clone(),
        known_devices: HashMap::new(),
    };

    // Create the nusb watcher, and start looking for device events. Unlike list_devices,
    // open, etc., watch_devices() itself doesn't need a blocking syscall to set up, so we
    // can call it directly without going through `crate::setup`.
    let mut watch = match nusb::watch_devices() {
        Ok(watch) => watch,
        Err(e) => {
            error!("Unable to start USB hotplug watch: {}", e);
            let _ = sender.send(HotPlugMessage::ThreadStopped);
            return;
        }
    };

    // watch_devices says to populate from list_devices after it's called, so we can
    // grab and handle devices which already exist.
    if let Ok(devices) = crate::setup::list_devices().await {
        // Locate all Beacn Devices
        let mut devices: Vec<_> = devices
            .filter_map(|info| identify_beacn_device(&info).map(|ty| (info, ty)))
            .collect();

        // Order them by startup order
        devices.sort_by_key(|(_, ty)| *ty);

        for (info, device_type) in devices {
            inner.device_connected(&info, device_type).await;
        }
    }

    // Periodic health-check tick, replacing the old poll-with-timeout loop -- this is
    // just another branch in the select below now that we're not restricted to blocking
    // primitives.
    let mut health_tick = Ticker::new(Duration::from_millis(100), false);

    loop {
        let event = or(
            or(
                async { HotplugLoopEvent::Management(receiver.recv_async().await) },
                async { HotplugLoopEvent::Hotplug(watch.next().await) },
            ),
            async {
                health_tick.tick().await;
                HotplugLoopEvent::HealthCheck
            },
        )
        .await;

        match event {
            HotplugLoopEvent::Management(Ok(HotPlugThreadManagement::Quit)) => break,
            HotplugLoopEvent::Management(Err(_)) => {
                error!("Receiver has Disconnected, terminating hot plug watcher");
                break;
            }
            HotplugLoopEvent::Hotplug(Some(HotplugEvent::Connected(info))) => {
                if let Some(device_type) = identify_beacn_device(&info) {
                    debug!("Found Beacn Device (type {:?})", device_type);
                    inner.device_connected(&info, device_type).await;
                }
            }
            HotplugLoopEvent::Hotplug(Some(HotplugEvent::Disconnected(info))) => {
                inner.device_removed(info);
            }
            HotplugLoopEvent::Hotplug(None) => {
                error!("Hotplug watch stream ended, terminating hot plug watcher");
                break;
            }
            HotplugLoopEvent::HealthCheck => {
                inner.check_device_health().await;
            }
        }
    }

    inner.thread_stopped();
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

/// This is a generic function that will just return a list of USB Locations of Beacn Mic devices
/// attached to your system for situations where you want to handle hot plugging yourself.
pub async fn get_beacn_mic_devices() -> Vec<DeviceLocation> {
    get_beacn_device(PID_BEACN_MIC).await
}

/// This is a generic function that will just return a list of USB Locations of Beacn Studio
/// devices attached to your system for situations where you want to handle hot plugging yourself.
pub async fn get_beacn_studio_devices() -> Vec<DeviceLocation> {
    get_beacn_device(PID_BEACN_STUDIO).await
}

/// This is a generic function that will just return a list USB Locations of Beacn Mix devices
/// attached to your system for situations where you want to handle hot plugging yourself.
pub async fn get_beacn_mix_device() -> Vec<DeviceLocation> {
    get_beacn_device(PID_BEACN_MIX).await
}

/// This is a generic function that will just return a list USB Locations of Beacn Mix Create
/// devices attached to your system for situations where you want to handle hot plugging yourself.
pub async fn get_beacn_mix_create_device() -> Vec<DeviceLocation> {
    get_beacn_device(PID_BEACN_MIX_CREATE).await
}

async fn get_beacn_device(pid: &[u16]) -> Vec<DeviceLocation> {
    let mut devices = vec![];
    if let Ok(devs) = crate::setup::list_devices().await {
        for info in devs {
            if info.vendor_id() == VENDOR_BEACN && pid.contains(&info.product_id()) {
                devices.push(DeviceLocation::from(&info));
            }
        }
    }
    devices
}
