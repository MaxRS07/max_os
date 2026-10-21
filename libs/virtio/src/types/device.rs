use alloc::boxed::Box;
use fdt::tree::FDT;
use mmio::{mmio_read, mmio_write};

use crate::{
    error::VirtioError,
    types::queue::{VirtQueue, VirtqAvail, VirtqDesc, VirtqUsed},
    *,
};
/// Shared basic functionality for all virtio devices
#[derive(Clone, Debug)]
pub struct VirtioDevice {
    pub mmio: usize,
    pub features: u64,
}

const QUEUE_DESC_HIGH: u32 = 0x84;
const QUEUE_DRIVER_HIGH: u32 = 0x94;
const QUEUE_DEVICE_HIGH: u32 = 0xA4;
const QUEUE_READY: u32 = 0x44;

impl VirtioDevice {
    /// Creates a device by probing MMIO port `idx`.
    /// Returns [`Ok`] containing an instance of self if the device is located on the given port and it's id matches `device_id`.
    pub fn probe(fdt: &FDT, idx: usize, device_id: u32) -> Result<Self, VirtioError> {
        let mmio = fdt.virtio_mmio[idx].base_address;
        let id = mmio_read::<u32>(mmio, VIRTIO_MMIO_DEVICE_ID);
        if id != device_id {
            return Err(VirtioError::BadId(id, device_id));
        }
        let magic = mmio_read::<u32>(mmio, VIRTIO_MMIO_MAGIC);
        if magic != VIRTIO_MMIO_MAGIC_VALUE {
            return Err(VirtioError::BadMagic(magic));
        }
        Ok(Self { mmio, features: 0 })
    }
    /// Negotiates `features` with the device. Returns [`Ok`] containing the agreed feature bits from the device.
    pub fn select_features(&mut self, features: u64) -> Result<u64, VirtioError> {
        // Reset status, causing a hardware reset
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_STATUS, 0); // Status = 0

        // write status 1 = OS acknowledgement
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
        // write status driver ready + OS acknowledge
        mmio_write::<u32>(
            self.mmio,
            VIRTIO_MMIO_STATUS,
            VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_ACKNOWLEDGE,
        );

        // Read device features
        // select + read lower 32 bits
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_DEV_FEATURES_SEL, 0);
        let features_low = mmio_read::<u32>(self.mmio, VIRTIO_MMIO_DEV_FEATURES);
        // select and read upper 32 bits
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_DEV_FEATURES_SEL, 1);
        let features_high = mmio_read::<u32>(self.mmio, VIRTIO_MMIO_DEV_FEATURES);

        let device_features = ((features_high as u64) << 32) | features_low as u64;
        let selected_features = device_features & features;

        let selected_features_high = (selected_features >> 32) as u32;
        let selected_features_low = selected_features as u32;

        // select and write lower features
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_DRV_FEATURES_SEL, 0); // DriverFeaturesSel = 0 (low 32)
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_DRV_FEATURES, selected_features_low);
        // select and write higher features
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_DRV_FEATURES_SEL, 1); // DriverFeaturesSel = 1 (high 32)
        mmio_write::<u32>(
            self.mmio,
            VIRTIO_MMIO_DRV_FEATURES,
            selected_features_high | 1, // force VERSION 1
        ); // bit 32 = VIRTIO_F_VERSION_1

        // tell device features are set
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_STATUS, 11);

        // verify device acknowledges features
        let status = mmio_read::<u32>(self.mmio, VIRTIO_MMIO_STATUS);

        if status & VIRTIO_STATUS_FEATURES_OK == 0 {
            return Err(VirtioError::FeaturesRejected);
        }
        self.features = selected_features;
        Ok(selected_features)
    }
    /// Intitializes and allocates a virtqueue with the given idx on this device
    pub fn setup_queue(&self, idx: u32) -> Result<VirtQueue, VirtioError> {
        // heap allocate queues
        let mut queue = VirtQueue::default();
        queue.desc = Box::new([VirtqDesc::default(); QUEUE_SIZE]);
        queue.avail = Box::new(VirtqAvail::default());
        queue.used = Box::new(VirtqUsed::default());
        queue.queue_idx = idx;
        queue.last_used_idx = 0;

        // set the selected queue to idx, the following register access will apply to idx
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_QUEUE_SEL, idx);

        // read max number of queues
        let max = mmio_read::<u32>(self.mmio, VIRTIO_MMIO_QUEUE_MAX); // QueueNumMax
        if max == 0 {
            return Err(VirtioError::QueueUnavailable(idx));
        }

        // set the queue's size to QUEUE_SIZE (16), must be 2^n and < QueueNumMax
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_QUEUE_NUM, QUEUE_SIZE as u32); // QueueNum

        // set the descriptor table physical address
        let desc_addr = &mut *queue.desc as *mut VirtqDesc as u32;
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_QUEUE_DESC, desc_addr);

        mmio_write::<u32>(self.mmio, QUEUE_DESC_HIGH, 0);

        // set the avail queue address
        let avail_addr = &mut *queue.avail as *mut VirtqAvail as u32;
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_QUEUE_DRIVER, avail_addr);

        mmio_write::<u32>(self.mmio, QUEUE_DRIVER_HIGH, 0);

        // set used address
        let used_addr = &mut *queue.used as *mut VirtqUsed as u32;
        mmio_write::<u32>(self.mmio, VIRTIO_MMIO_QUEUE_DEVICE, used_addr);

        mmio_write::<u32>(self.mmio, QUEUE_DEVICE_HIGH, 0);

        // set queue ready to true

        mmio_write::<u32>(self.mmio, QUEUE_READY, 1);

        Ok(queue)
    }
    /// Returns configuration information about this device
    pub fn read_config<T>(&self) -> T {
        mmio::mmio_read(self.mmio, VIRTIO_MMIO_CONFIG)
    }
    /// Sets the driver status to DRIVER_OK, enabling regular device operation.
    pub fn finish(&self) {
        mmio_write::<u32>(
            self.mmio,
            VIRTIO_MMIO_STATUS,
            VIRTIO_STATUS_ACKNOWLEDGE
                | VIRTIO_STATUS_DRIVER
                | VIRTIO_STATUS_FEATURES_OK
                | VIRTIO_STATUS_DRIVER_OK,
        );
    }
}
