#![no_std]

extern crate alloc;

pub mod arch;
pub mod console;
pub mod drivers;
pub mod mm;

use core::arch::global_asm;

use log::{debug, info, warn};

use crate::{
    console::terminal::println,
    drivers::{gpu, mmio::probe_virtio_mmio_gpu},
    mm::map,
};

global_asm!(include_str!("boot.s"));

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text._start")]
pub extern "C" fn kernel(hart_id: usize, fdt_ptr: *const u8) -> ! {
    arch::riscv::setup();
    // main thread, do init stuff
    console::init();
    info!("entry with hart_id: {}", hart_id);
    mm::init(fdt_ptr);
    if let Some(map) = map::MEMORY_MAP.get() {
        debug!("Initialized memory map");
        if let Some(reg) = probe_virtio_mmio_gpu(map) {
            gpu::initialize_virtio_gpu(reg.base_address);
        }
    } else {
        warn!("Memory map failed to load")
    }

    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
