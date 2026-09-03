use mmio::{mmio_read, mmio_write};

use crate::{
    VIRTIO_MMIO_DEV_FEATURES, VIRTIO_MMIO_DEV_FEATURES_SEL, VIRTIO_MMIO_DRV_FEATURES,
    VIRTIO_MMIO_DRV_FEATURES_SEL, VIRTIO_MMIO_STATUS,
};

/// Gets the GPU device ready for rw
pub fn init_state(mmio_addr: usize, select: u64) -> Result<(), &'static str> {
    // Reset
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_STATUS, 0); // Status = 0

    // ACKNOWLEDGE | DRIVER
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_STATUS, 1);
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_STATUS, 3);

    // Read device features
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_DEV_FEATURES_SEL, 0); // DeviceFeaturesSel = 0
    let features_low = mmio_read::<u32>(mmio_addr, VIRTIO_MMIO_DEV_FEATURES);
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_DEV_FEATURES_SEL, 1);
    let features_high = mmio_read::<u32>(mmio_addr, VIRTIO_MMIO_DEV_FEATURES);

    let features = ((features_high as u64) << 32) | features_low as u64;
    let selected_features = features & select;

    let selected_features_high = (selected_features >> 32) as u32;
    let selected_features_low = selected_features as u32;

    // Acknowledge VIRTIO_F_VERSION_1 (bit 32)
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_DRV_FEATURES_SEL, 0); // DriverFeaturesSel = 0 (low 32)
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_DRV_FEATURES, selected_features_low);
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_DRV_FEATURES_SEL, 1); // DriverFeaturesSel = 1 (high 32)
    mmio_write::<u32>(
        mmio_addr,
        VIRTIO_MMIO_DRV_FEATURES,
        selected_features_high | 1, // force VERSION 1
    ); // bit 32 = VIRTIO_F_VERSION_1

    // FEATURES_OK
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_STATUS, 11);

    // check FEATURES_OK stuck
    let status = mmio_read::<u32>(mmio_addr, VIRTIO_MMIO_STATUS);

    if status & 8 == 0 {
        return Err("Device rejected features");
    }
    Ok(())
}

/// Signals DRIVER_OK, telling the device the driver is ready to use the queues it just set up.
/// Must be called after all virtqueues for the device are initialized.
pub fn signal_driver_ok(mmio_addr: usize) {
    // ACKNOWLEDGE | DRIVER | FEATURES_OK | DRIVER_OK
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_STATUS, 1 | 2 | 8 | 4);
}
