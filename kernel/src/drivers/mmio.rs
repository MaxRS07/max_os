use log::info;
use sdt::mreg::MemoryRegion;

use crate::{
    drivers::pci::config::{VIRTIO_MMIO_DEVICE_ID, VIRTIO_MMIO_MAGIC_VALUE},
    mm::map::MemoryMap,
};

pub fn mmio_write<T>(base: usize, offset: u32, val: T) {
    let ptr = (base + offset as usize) as *mut T;
    unsafe { ptr.write_volatile(val) }
}

pub fn mmio_read<T>(base: usize, offset: u32) -> T {
    let ptr = (base + offset as usize) as *mut T;
    unsafe { ptr.read_volatile() }
}

pub fn probe_virtio_mmio_gpu(map: &MemoryMap) -> Option<MemoryRegion> {
    for reg in map.virtio_mmio {
        let magic = mmio_read::<u32>(reg.base_address, 0);
        if magic != VIRTIO_MMIO_MAGIC_VALUE {
            continue;
        }
        let device_id = unsafe { ((reg.base_address + 0x8) as *const u32).read_volatile() };
        if device_id == VIRTIO_MMIO_DEVICE_ID {
            return Some(reg);
        }
    }
    None
}
