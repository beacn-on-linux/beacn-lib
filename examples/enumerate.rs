use crate::common::logging::configure_logging;
use beacn_lib::MaybeFuture;
use beacn_lib::manager::{
    get_beacn_mic_devices, get_beacn_mix_create_device, get_beacn_mix_device,
    get_beacn_studio_devices,
};
use log::info;

#[path = "common/mod.rs"]
mod common;

#[cfg(target_arch = "wasm32")]
compile_error!("Sync Examples are not supported under WASM, use the async variant instead.");

fn main() {
    configure_logging();

    // Simply enumerate all devices
    for device in get_beacn_mic_devices().wait() {
        info!("Mic Found At: {:?}", device);
    }

    for device in get_beacn_studio_devices().wait() {
        info!("Studio Found At: {:?}", device);
    }

    for device in get_beacn_mix_create_device().wait() {
        info!("Mix Create Found At: {:?}", device);
    }

    for device in get_beacn_mix_device().wait() {
        info!("Mix Found At: {:?}", device);
    }
}
