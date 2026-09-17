use core::cell::OnceCell;

use alloc::boxed::Box;
use block::device::BlockDevice;
use log::warn;
use sdt::fdt::FDT;

use crate::{drivers::block::virtio::VirtioBlock, println};

pub mod virtio;

pub static BLOCK_DEVICE: OnceCell<&'static mut dyn BlockDevice> = OnceCell::new();

pub fn init(fdt: &FDT, mmio_idx: usize) {
    if let Some(blk_drv) = VirtioBlock::from_mmio(fdt, mmio_idx) {
        let boxed = Box::new(blk_drv);
        let raw_box = Box::into_raw(boxed);
        unsafe {
            BLOCK_DEVICE.init(|| &mut *raw_box);
        }
    } else {
        warn!("Failed to intialize VirtIO BLK")
    }
}
