#![no_std]

extern crate alloc;

pub mod arch;
pub mod boot;
pub mod console;
pub mod drivers;
pub mod fs;
pub mod mm;
pub mod sched;
pub mod syscall;

use core::{arch::global_asm, panic};

use log::{Level::Debug, debug, info, warn};
use sdt::fdt::GLOBAL_FDT;

use crate::{
    arch::riscv::{self, interrupt::handler::schedule_interrupt_timer},
    drivers::block::BLOCK_DEVICE,
    mm::{_boot_stack_bottom, _boot_stack_top},
};

global_asm!(include_str!("boot.s"));

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(hart_id: usize, fdt_ptr: *const u8) -> ! {
    println!("Entered kernel in S mode. Hardware thread {}.", hart_id);
    mm::init(fdt_ptr); // global allocator & fdt

    let Some(fdt) = GLOBAL_FDT.get() else {
        panic!("Failed to parse device tree");
    };
    riscv::setup();

    info!("Entry on hart id: {}", hart_id);
    info!("Detected RAM: {} bytes", fdt.memory.size);

    mmio::probe_mmio_devices(fdt, |id, mmio_idx| match id {
        1 => drivers::net::ini,
        2 => drivers::block::init(fdt, mmio_idx),
        16 => drivers::gpu::init(fdt, mmio_idx),
        18 => drivers::input::init(fdt, mmio_idx),
        _ => {}
    });

    if let Some(blk_dev) = unsafe { BLOCK_DEVICE.get_mut() } {
        debug!("Initializing file system");
        match fs::fs_init(&mut **blk_dev, fdt.get_arg("remount")) {
            Ok(_) => debug!("Initialized file system"),
            Err(msg) => warn!("Failed to initialize file system: {msg:?}"),
        }
    }

    schedule_interrupt_timer(10_000_000);

    info!("Kernel initialized successfully, entering main loop");
    unsafe {
        let stack_top = _boot_stack_top as *const u8;
        let stack_bottom = _boot_stack_bottom as *const u8;
        info!("Loaded stack");
        sched::init_run_queue(stack_top, stack_bottom);
        info!("Run queue initialized");
    }
    // enable hardware timer
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
