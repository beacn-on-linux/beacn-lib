use crate::common::logging::configure_logging;
use beacn_lib::manager::{
    get_beacn_mic_devices, get_beacn_mix_create_device, get_beacn_mix_device,
    get_beacn_studio_devices,
};
use log::info;

#[path = "common/mod.rs"]
mod common;

beacn_main!(flavor = "current_thread", {
    app_main().await;
});

async fn app_main() {
    configure_logging();

    let mut count = 0;
    info!("Looking for Devices");

    // Simply enumerate all devices
    for device in get_beacn_mic_devices().await {
        count += 1;
        info!("Mic Found At: {:?}", device);
    }

    for device in get_beacn_studio_devices().await {
        count += 1;
        info!("Studio Found At: {:?}", device);
    }

    for device in get_beacn_mix_create_device().await {
        count += 1;
        info!("Mix Create Found At: {:?}", device);
    }

    for device in get_beacn_mix_device().await {
        count += 1;
        info!("Mix Found At: {:?}", device);
    }

    let s = if count == 1 { "" } else { "s" };
    info!("Found {} device{s}", count);
}
