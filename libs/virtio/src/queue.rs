use alloc::boxed::Box;
use mmio::{mmio_read, mmio_write};

use crate::{
    QUEUE_SIZE, VIRTIO_MMIO_QUEUE_DESC, VIRTIO_MMIO_QUEUE_DEVICE, VIRTIO_MMIO_QUEUE_DRIVER,
    VIRTIO_MMIO_QUEUE_MAX, VIRTIO_MMIO_QUEUE_NUM, VIRTIO_MMIO_QUEUE_SEL,
    types::queue::{VirtQueue, VirtqAvail, VirtqDesc, VirtqUsed},
};

fn setup_queue(
    mmio_addr: usize,
    queue: &mut VirtQueue,
    queue_idx: u32,
) -> Result<(), &'static str> {
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_QUEUE_SEL, queue_idx); // QueueSel — must be first

    let max = mmio_read::<u32>(mmio_addr, VIRTIO_MMIO_QUEUE_MAX); // QueueNumMax
    if max == 0 {
        return Err("Queue not available");
    }

    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_QUEUE_NUM, QUEUE_SIZE as u32); // QueueNum

    let desc_addr = &mut *queue.desc as *mut VirtqDesc as u32;
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_QUEUE_DESC, desc_addr);
    mmio_write::<u32>(mmio_addr, 0x84, 0);

    let avail_addr = &mut *queue.avail as *mut VirtqAvail as u32;
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_QUEUE_DRIVER, avail_addr);
    mmio_write::<u32>(mmio_addr, 0x94, 0);

    let used_addr = &mut *queue.used as *mut VirtqUsed as u32;
    mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_QUEUE_DEVICE, used_addr);
    mmio_write::<u32>(mmio_addr, 0xA4, 0);
    Ok(())
}

pub fn init_virtqueue(
    queue: &mut VirtQueue,
    mmio_addr: usize,
    queue_idx: u32,
) -> Result<(), &'static str> {
    queue.desc = Box::new([VirtqDesc::default(); QUEUE_SIZE]);
    queue.avail = Box::new(VirtqAvail::default());
    queue.used = Box::new(VirtqUsed::default());

    // select queue and set addresses
    setup_queue(mmio_addr, queue, queue_idx)?;

    mmio_write::<u32>(mmio_addr, 0x44, 1);

    Ok(())
}
