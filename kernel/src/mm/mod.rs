use core::sync::atomic::Ordering::{Acquire, SeqCst};
use core::sync::atomic::fence;

use log::{info, warn};

use crate::mm::map::{MEMORY_MAP, init_static_map};
use crate::println;

pub mod error;
pub mod heap;
pub mod map;
pub mod pmm;
pub mod vmm;

pub fn init(fdt_ptr: *const u8) {
    if let Ok(mmap) = init_static_map(fdt_ptr) {
        map::MEMORY_MAP.init(|| mmap);
        heap::init(mmap.memory.size);
        fence(SeqCst);
        info!("Memory map initialized");
    } else {
        warn!("Failed to initialize memory map")
    }
}
/// Writes a byte at an address
/// # Safety
/// `addr` must be a valid MMIO address. See `memory::map` for valid ranges.
pub unsafe fn mem_write(addr: u32, value: u8) {
    let pointer = addr as *mut u8;
    unsafe {
        pointer.write_volatile(value);
    }
}

/// Reads a byte at an address
/// # Safety
/// `addr` must be a valid MMIO address. See `memory::map` for valid ranges.
pub unsafe fn mem_read(addr: u32) -> u8 {
    let pointer = addr as *const u8;
    unsafe { pointer.read_volatile() }
}
