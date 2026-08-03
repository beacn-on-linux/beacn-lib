use env_logger::Env;
use beacn_lib::MaybeFuture;
use beacn_lib::manager::{
    get_beacn_mic_devices, get_beacn_mix_create_device, get_beacn_mix_device,
    get_beacn_studio_devices,
};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    // Simply enumerate all devices
    for device in get_beacn_mic_devices().wait() {
        println!("Mic Found At: {:?}", device);
    }

    for device in get_beacn_studio_devices().wait() {
        println!("Studio Found At: {:?}", device);
    }

    for device in get_beacn_mix_create_device().wait() {
        println!("Mix Create Found At: {:?}", device);
    }

    for device in get_beacn_mix_device().wait() {
        println!("Mix Found At: {:?}", device);
    }
}
