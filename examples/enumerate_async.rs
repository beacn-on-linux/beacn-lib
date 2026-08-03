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

    // Simply enumerate all devices
    for device in get_beacn_mic_devices().await {
        info!("Mic Found At: {:?}", device);
    }

    for device in get_beacn_studio_devices().await {
        info!("Studio Found At: {:?}", device);
    }

    for device in get_beacn_mix_create_device().await {
        info!("Mix Create Found At: {:?}", device);
    }

    for device in get_beacn_mix_device().await {
        info!("Mix Found At: {:?}", device);
    }
}
