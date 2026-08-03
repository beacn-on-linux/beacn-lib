use crate::common::logging::configure_logging;
use beacn_lib::manager::{HotPlugMessage, HotPlugThreadManagement, watch_hotplug_devices};
use log::info;
use tokio::{join, select, task};
use web_time::Duration;
use crate::common::{sleep, spawn_local};

#[path = "common/mod.rs"]
mod common;

beacn_main!(flavor = "local", {
    app_main().await;
});

async fn app_main() {
    configure_logging();
    info!("Starting Hotplug Example");

    info!("Generating hotplug channels: (hotplug_tx, hotplug_rx) = (flume::unbounded(), flume::unbounded())");
    let (hotplug_tx, hotplug_rx) = flume::unbounded();
    let (mgmt_tx, mgmt_rx) = flume::unbounded();

    info!("Spawning hotplug thread");
    // Spawn up a hotplug thread, this will announce all existing devices and watch for new ones.
    let handle = spawn_local(watch_hotplug_devices(hotplug_tx, mgmt_rx));

    info!("Thread Spawned");
    // Listen for messages coming from the hotplug thread for 10 seconds, then exit.
    loop {
        select! {
            Ok(message) = hotplug_rx.recv_async() => {
                match message {
                    HotPlugMessage::DeviceAttached(location, device_type, _health) => {
                        // The health channel is used when opening control devices (Mix / Mix Create) and
                        // is triggered when there is some kind of failure to send / receive data from
                        // the devices (for example, if the device is unplugged).
                        //
                        // If the health channel passed into this enum is sent into open(), when a failure
                        // occurs it will defer back to the hotplug manager which will handle recovering
                        // the device.
                        //
                        // In this example, we're just going to ignore the health channel.
                        info!("{:?} Device Attached: {:?}", device_type, location);
                    }
                    HotPlugMessage::DeviceRemoved(location) => {
                        info!("Device Removed: {:?}", location);
                    }
                    HotPlugMessage::ThreadStopped => {
                        break;
                    }
                }
            },
            _ = sleep(Duration::from_secs(10)) => {
                break;
            }
        }
    }

    let _ = mgmt_tx.send(HotPlugThreadManagement::Quit);
    handle.join().await;
}
