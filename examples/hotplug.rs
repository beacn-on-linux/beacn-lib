use beacn_lib::manager::{HotPlugMessage, HotPlugThreadManagement, run_hotplug_handler};
use flume::TryRecvError;
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};

fn main() {
    let (hotplug_tx, hotplug_rx) = flume::unbounded();
    let (mgmt_tx, mgmt_rx) = flume::unbounded();
    let start = Instant::now();

    // Spawn up a hotplug thread, this will announce all existing devices and watch for new ones.
    let handle = thread::spawn(|| run_hotplug_handler(hotplug_tx, mgmt_rx));

    // Listen for messages coming from the hotplug thread for 10 seconds, then exit.
    loop {
        match hotplug_rx.try_recv() {
            Ok(message) => match message {
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
                    println!("{:?} Device Attached: {:?}", device_type, location);
                }
                HotPlugMessage::DeviceRemoved(location) => {
                    println!("Device Removed: {:?}", location);
                }
                HotPlugMessage::ThreadStopped => {
                    break;
                }
            },
            Err(e) => {
                if e == TryRecvError::Empty {
                    if start.elapsed() > Duration::from_secs(10) {
                        // We should exit our loop at this point
                        break;
                    }

                    sleep(Duration::from_millis(10));
                    continue;
                }
            }
        }
    }

    let _ = mgmt_tx.send(HotPlugThreadManagement::Quit);
    let _ = handle.join().unwrap();
}
