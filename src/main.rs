#![no_std]
#![no_main]

mod utils;
mod vga;

use core::panic::PanicInfo;
use core::ptr::slice_from_raw_parts;
use core::str;
use mem_utils::Strable;
use utils::*;
use vga::*;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    loop {
        // Main OS loop
    }
}
