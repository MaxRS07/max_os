#![no_std]

use core::time::Duration;

pub mod global_time;

unsafe extern "Rust" {
    static __GLOBAL_TIMER: &'static dyn global_time::GlobalTimer;
}

pub fn ticks() -> u64 {
    unsafe { __GLOBAL_TIMER.ticks() }
}

pub fn freq() -> u64 {
    unsafe { __GLOBAL_TIMER.freq() }
}

pub fn now_ms() -> u64 {
    unsafe { __GLOBAL_TIMER.now_ms() }
}

pub fn elapsed_ms(start: u64, duration: Duration) -> bool {
    unsafe { __GLOBAL_TIMER.elapsed_ms(start, duration) }
}
