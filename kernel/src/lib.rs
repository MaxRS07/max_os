#![no_std]

extern crate alloc;

pub mod arch;
pub mod boot;
pub mod console;
pub mod drivers;
pub mod mm;
pub mod sched;

use core::arch::global_asm;

use log::info;
use sdt::fdt::GLOB_FDT;

use crate::{
    arch::riscv::{self, interrupt::handler::schedule_interrupt_timer},
    console::writer::print,
    drivers::uart,
    mm::{_boot_stack_bottom, _boot_stack_top},
    sched::{
        queue::THREAD_QUEUE,
        thread::{self, Priority, Thread},
    },
};

global_asm!(include_str!("boot.s"));

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(hart_id: usize, fdt_ptr: *const u8) -> ! {
    println!("Entered kernel in S mode. Hardware thread {}.", hart_id);
    mm::init(fdt_ptr); // global allocator & fdt
    if GLOB_FDT.get().is_none() {
        panic!("Failed to parse device tree");
    }
    let fdt = GLOB_FDT.get().unwrap();

    riscv::setup();
    console::init(fdt.debug_mode);
    info!("Entry on hart id: {}", hart_id);
    info!("Detected RAM: {} bytes", fdt.memory.size);

    mmio::probe_mmio_devices(fdt, |id, mmio_idx| match id {
        1 => { /* TODO: Implement Blk driver */ }
        2 => { /* TODO: Implement net driver */ }
        16 => drivers::gpu::init(fdt, mmio_idx),
        18 => drivers::input::init(fdt, mmio_idx),
        _ => {}
    });
    schedule_interrupt_timer(10_000_000);
    info!("Kernel initialized successfully, entering main loop");

    unsafe {
        let stack_top = _boot_stack_top as *const u8;
        let stack_bottom = _boot_stack_bottom as *const u8;
        sched::init_run_queue(stack_top, stack_bottom);

        Thread::spawn("thread a", Priority::Normal, thread_entry as usize);
    }
    // enable hardware timer
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

fn thread_entry() {
    info!("Hello from Thread A");
}
