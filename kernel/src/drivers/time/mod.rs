use core::arch::{asm, global_asm};

use sdt::fdt::GLOB_FDT;

global_asm!(include_str!("time.s"));

const TIMER_FREQUENCY: u64 = 10_000_000;

unsafe extern "C" {
    unsafe fn get_raw_time() -> u64;
}

pub fn get_uptime_ms() -> u64 {
    let raw_time = unsafe { get_raw_time() };
    raw_time * 1000 / TIMER_FREQUENCY
}

const MTIME_OFFSET: usize = 0xBFF8;

pub fn mtime_raw(base_clint: usize) -> u64 {
    let addr = base_clint + MTIME_OFFSET;
    let ptr = addr as *const u64;
    unsafe { core::ptr::read_volatile(ptr) }
}

/// return the kernel uptime in milliseconds
pub fn mtime_ms() -> u64 {
    if let Some(fdt) = GLOB_FDT.get() {
        let raw_time = mtime_raw(fdt.clint.base_address);
        return raw_time * 1000 / TIMER_FREQUENCY;
    }
    100
}
