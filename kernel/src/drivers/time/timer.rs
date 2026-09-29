use core::arch::{asm, global_asm};

use sdt::fdt::GLOBAL_FDT;

global_asm!(include_str!("time.s"));

const TIMER_FREQUENCY: u64 = 10_000_000;

unsafe extern "C" {
    unsafe fn get_raw_time() -> u64;
}

pub(crate) fn get_upticks() -> u64 {
    unsafe { get_raw_time() }
}

pub(crate) fn get_uptime_ms() -> u64 {
    let raw_time = unsafe { get_raw_time() };
    raw_time * 1000 / TIMER_FREQUENCY
}

const MTIME_OFFSET: usize = 0xBFF8;

pub(crate) fn mtime_raw(base_clint: usize) -> u64 {
    let addr = base_clint + MTIME_OFFSET;
    let ptr = addr as *const u64;
    unsafe { core::ptr::read_volatile(ptr) }
}

pub(crate) fn mtime_ticks() -> u64 {
    let fdt = GLOBAL_FDT.wait();
    mtime_raw(fdt.clint.base_address)
}

/// return the kernel uptime in milliseconds
pub(crate) fn mtime_ms() -> u64 {
    let raw_time = mtime_ticks();
    raw_time * 1000 / TIMER_FREQUENCY
}
