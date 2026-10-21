use core::cell::OnceCell;

use alloc::boxed::Box;
use block::device::BlockDevice;
use fdt::tree::FDT;
use log::warn;
use sync::oncelock::OnceLock;

use crate::{drivers::block::virtio::VirtioBlock, println};

pub mod virtio;

pub static BLOCK_DEVICE: OnceLock<&'static mut dyn BlockDevice> = OnceLock::new();

pub fn init(fdt: &FDT, mmio_idx: usize) {
    match VirtioBlock::new(fdt, mmio_idx) {
        Ok(blk_drv) => {
            let boxed = Box::new(blk_drv);
            let raw_box = Box::into_raw(boxed);
            unsafe {
                BLOCK_DEVICE.set(&mut *raw_box);
            }
        }
        Err(error) => warn!("Failed to intialize VirtIO BLK: {error}"),
    }
}
