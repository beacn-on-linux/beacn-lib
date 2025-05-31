use crate::common::{BeacnDeviceHandle, DeviceDefinition, get_device_info};
use crate::controller::BeacnControlDevice;
use anyhow::Result;
use anyhow::bail;
use log::debug;
use std::time::Duration;

pub trait BeacnControlDeviceAttach {
    // We're specifically allowing the DeviceDefinition to be a private interface, as it's
    // simply used internally for connection up a device, and shouldn't have any visibility
    // from the outside. This also prevents external code from attempting to call connect.
    #[allow(private_interfaces)]
    fn connect(definition: DeviceDefinition) -> Result<Box<dyn BeacnControlDevice>>
    where
        Self: Sized;

    fn get_product_id(&self) -> u16;
    fn get_serial(&self) -> String;
    fn get_version(&self) -> String;
}

/// Simple function to Open a libusb connection to a Beacn Audio device, do initial setup and
/// grab the firmware version from the device.
pub(crate) fn open_beacn(def: DeviceDefinition, product_id: u16) -> Result<BeacnDeviceHandle> {
    if def.descriptor.product_id() != product_id {
        bail!(
            "Expecting PID {} but got {}",
            product_id,
            def.descriptor.product_id()
        );
    }

    let handle = def.device.open()?;
    handle.claim_interface(0)?;
    handle.set_alternate_setting(0, 1)?;
    handle.clear_halt(0x83)?;

    let setup_timeout = Duration::from_millis(2000);

    // Unlike the Mic and Studio, we use an interrupt, rather a bulk read
    let mut input = [0; 64];
    handle.write_interrupt(0x03, &[00, 00, 00, 1], setup_timeout)?;
    handle.read_interrupt(0x83, &mut input, setup_timeout)?;

    let (version, serial) = get_device_info(&input)?;

    debug!(
        "Loaded Device, Location: {}.{}, Serial: {}, Version: {}",
        def.device.bus_number(),
        def.device.address(),
        serial.clone(),
        version
    );

    Ok(BeacnDeviceHandle {
        descriptor: def.descriptor,
        device: def.device,
        handle,
        version,
        serial,
    })
}
