use core::hint::spin_loop;
use core::sync::atomic::Ordering::{self, Acquire, SeqCst};
use core::sync::atomic::{compiler_fence, fence};

use log::{debug, info, warn};

use mem::page_table::table::Table;
use sdt::fdt::{FDT, GLOBAL_FDT};

use crate::drivers::time::start_system_clock;
use crate::mm::boot::map_boot_pages;
use crate::mm::heap::{PAGE_ALLOCATOR, setup_boot_mem};
use crate::{console, println};

pub mod boot;
pub mod heap;

// declare boot stack addresses
unsafe extern "C" {
    pub unsafe static _boot_stack_top: usize;
    pub unsafe static _boot_stack_bottom: usize;
}

pub const BOOT_HEAP_SIZE: usize = 0x20_000;

pub fn init(fdt_ptr: *const u8) {
    setup_boot_mem();
    match FDT::from_ptr(fdt_ptr) {
        Ok(mmap) => {
            start_system_clock();
            console::init(mmap.get_arg("debug"));
            debug!("Intialized FDT");
            sdt::fdt::GLOBAL_FDT.set(mmap);
            if let Some(fdt) = GLOBAL_FDT.get() {
                heap::setup_system_mem(fdt.memory.size);
            } else {
                warn!("Failed to set global device tree");
            }
            fence(SeqCst);
            debug!("Memory map initialized");
        }
        Err(msg) => warn!("Failed to initialize memory map: {}", msg),
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
