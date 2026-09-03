// pub trait VirtQueuedDevice {
//     fn set_
// }
use crate::{VIRTIO_MMIO_MAGIC, VIRTIO_MMIO_MAGIC_VALUE};

pub fn verify_virtio_magic(mmio_addr: usize) -> Result<(), &'static str> {
    let magic = mmio::mmio_read::<u32>(mmio_addr, VIRTIO_MMIO_MAGIC);
    if magic != VIRTIO_MMIO_MAGIC_VALUE {
        return Err("Failed");
    }
    Ok(())
}
