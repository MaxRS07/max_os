use core::sync::atomic::Ordering::{self, Acquire, SeqCst};
use core::sync::atomic::fence;

use log::{info, warn};

use sdt::fdt::{FDT, GLOBAL_FDT};

use crate::mm::heap::{page_table, setup_boot_mem};
use crate::println;

pub mod error;
pub mod heap;
pub mod pmm;

// declare boot stack addresses
unsafe extern "C" {
    pub unsafe static _boot_stack_top: usize;
    pub unsafe static _boot_stack_bottom: usize;
}

pub fn init(fdt_ptr: *const u8) {
    setup_boot_mem();
    match FDT::from_ptr(fdt_ptr) {
        Ok(mmap) => {
            println!("Intialized FDT");
            sdt::fdt::GLOBAL_FDT.init(|| mmap);
            if let Some(fdt) = GLOBAL_FDT.get() {
                println!("{:?}", fdt);
                page_table::init_root_table();
                heap::setup_system_mem(fdt.memory.size);
            } else {
                warn!("Failed to set global device tree");
            }
            fence(SeqCst);
            println!("Memory map initialized");
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
