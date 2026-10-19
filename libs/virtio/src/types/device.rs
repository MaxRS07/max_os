use crate::{VIRTIO_MMIO_MAGIC, VIRTIO_MMIO_MAGIC_VALUE, error::VirtioError};

pub fn verify_virtio_magic(mmio_addr: usize) -> Result<(), VirtioError> {
    let magic = mmio::mmio_read::<u32>(mmio_addr, VIRTIO_MMIO_MAGIC);
    if magic != VIRTIO_MMIO_MAGIC_VALUE {
        return Err(VirtioError::BadMagic(magic));
    }
    Ok(())
}
