//! Thin wrappers around nusb's *connection-setup* calls: `list_devices`, `DeviceInfo::open`,
//! `Device::claim_interface`, `Interface::set_alt_setting`, `Endpoint::clear_halt`.
use nusb::transfer::{BulkOrInterrupt, EndpointDirection};
use nusb::{Device, DeviceInfo, Interface};

pub(crate) async fn list_devices() -> Result<impl Iterator<Item = DeviceInfo>, nusb::Error> {
    #[cfg(any(feature = "tokio", feature = "smol", target_arch = "wasm32"))]
    {
        nusb::list_devices().await
    }
    #[cfg(not(any(feature = "tokio", feature = "smol", target_arch = "wasm32")))]
    {
        use nusb::MaybeFuture;
        nusb::list_devices().wait()
    }
}

pub(crate) async fn open(info: &DeviceInfo) -> Result<Device, nusb::Error> {
    #[cfg(any(feature = "tokio", feature = "smol", target_arch = "wasm32"))]
    {
        info.open().await
    }
    #[cfg(not(any(feature = "tokio", feature = "smol", target_arch = "wasm32")))]
    {
        use nusb::MaybeFuture;
        info.open().wait()
    }
}

pub(crate) async fn claim_interface(
    device: &Device,
    interface: u8,
) -> Result<Interface, nusb::Error> {
    #[cfg(any(feature = "tokio", feature = "smol", target_arch = "wasm32"))]
    {
        device.claim_interface(interface).await
    }
    #[cfg(not(any(feature = "tokio", feature = "smol", target_arch = "wasm32")))]
    {
        use nusb::MaybeFuture;
        device.claim_interface(interface).wait()
    }
}

pub(crate) async fn set_alt_setting(interface: &Interface, alt: u8) -> Result<(), nusb::Error> {
    #[cfg(any(feature = "tokio", feature = "smol", target_arch = "wasm32"))]
    {
        interface.set_alt_setting(alt).await
    }
    #[cfg(not(any(feature = "tokio", feature = "smol", target_arch = "wasm32")))]
    {
        use nusb::MaybeFuture;
        interface.set_alt_setting(alt).wait()
    }
}

pub(crate) async fn clear_halt<EpType, Dir>(
    endpoint: &mut nusb::Endpoint<EpType, Dir>,
) -> Result<(), nusb::Error>
where
    EpType: BulkOrInterrupt,
    Dir: EndpointDirection,
{
    #[cfg(any(feature = "tokio", feature = "smol", target_arch = "wasm32"))]
    {
        #[cfg(not(target_arch = "wasm32"))]
        endpoint.cancel_all();

        endpoint.clear_halt().await
    }
    #[cfg(not(any(feature = "tokio", feature = "smol", target_arch = "wasm32")))]
    {
        use nusb::MaybeFuture;
        endpoint.cancel_all();
        endpoint.clear_halt().wait()
    }
}
