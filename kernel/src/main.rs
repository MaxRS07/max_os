#![no_std]
#![no_main]

use core::panic::PanicInfo;

use max_os::println;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("Panic at: {}", _info);
    loop {}
}
