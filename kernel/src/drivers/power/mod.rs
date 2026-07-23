use log::{info, warn};
use sdt::{fdt::GLOBAL_FDT, stream::FDTElement};

use crate::println;

const POWEROFF: u32 = 0x5555;

/// Triggers a hardware shutdown. This will close QEMU
pub fn sys_poweroff() {
    if let Some(fdt) = GLOBAL_FDT.get()
        && let Some(e) = fdt.get_element("root/soc/test/reg")
        && let FDTElement::Property { value_ptr, len, .. } = e
    {
        let addr = value_ptr.addr();
        let bytes = unsafe { core::slice::from_raw_parts(value_ptr, len) };
        let base = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;

        let dev_ptr = base as *mut u32;

        unsafe { core::ptr::write_volatile(dev_ptr, POWEROFF) }
    } else {
        warn!("Failed to retrive poweroff address");
    }
    // wait for shutdown
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack, preserves_flags)) }
    }
}

#[macro_export]
macro_rules! shutdown {
    () => {
        sys_poweroff();
    };
}
