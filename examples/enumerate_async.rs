use beacn_lib::manager::{
    get_beacn_mic_devices, get_beacn_mix_create_device, get_beacn_mix_device,
    get_beacn_studio_devices,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Simply enumerate all devices
    for device in get_beacn_mic_devices().await {
        println!("Mic Found At: {:?}", device);
    }

    for device in get_beacn_studio_devices().await {
        println!("Studio Found At: {:?}", device);
    }

    for device in get_beacn_mix_create_device().await {
        println!("Mix Create Found At: {:?}", device);
    }

    for device in get_beacn_mix_device().await {
        println!("Mix Found At: {:?}", device);
    }
}
