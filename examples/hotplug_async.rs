use beacn_lib::manager::{HotPlugMessage, HotPlugThreadManagement, watch_hotplug_devices};
use std::time::Duration;
use env_logger::Env;
use log::info;
use tokio::time::sleep;
use tokio::{join, select, task};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    let (hotplug_tx, hotplug_rx) = flume::unbounded();
    let (mgmt_tx, mgmt_rx) = flume::unbounded();

    // Spawn up a hotplug thread, this will announce all existing devices and watch for new ones.
    let handle = task::spawn_local(watch_hotplug_devices(hotplug_tx, mgmt_rx));

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
    let _ = join!(handle);
}
