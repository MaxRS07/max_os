#![no_std]

use alloc::string::ToString;
use log::{Level::Info, info, warn};
use sdt::{fdt::FDT, region::FDTRegion, stream::FdtElement};

pub fn mmio_write<T>(base: usize, offset: u32, val: T) {
    let ptr = (base + offset as usize) as *mut T;
    unsafe { ptr.write_volatile(val) }
}

pub fn mmio_read<T>(base: usize, offset: u32) -> T {
    let ptr = (base + offset as usize) as *mut T;
    unsafe { ptr.read_volatile() }
}

pub fn probe_mmio_devices<F>(fdt: &FDT, callback: F)
where
    F: Fn(u32, usize),
{
    for i in 0..fdt.virtio_mmio.len() {
        let reg = &fdt.virtio_mmio[i];
        let device_id = unsafe { ((reg.base_address + 0x8) as *const u32).read_volatile() };
        callback(device_id, i);
    }
}
extern crate alloc;

pub fn get_interrupt(fdt: &FDT, mmio_idx: usize) -> Option<u32> {
    let path = alloc::format!("root/soc/virtio_mmio/{}/interrupts", mmio_idx);

    if let Some(e) = fdt.get_element_string(&path)
        && let FdtElement::Property { value_ptr, .. } = e
    {
        unsafe {
            return Some((value_ptr as *const u32).read_volatile().swap_bytes());
        }
    }
    warn!("Element at {} not found", path);
    None
}
