use env_logger::Env;
use log::info;
use beacn_lib::manager::{
    get_beacn_mic_devices, get_beacn_mix_create_device, get_beacn_mix_device,
    get_beacn_studio_devices,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
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
