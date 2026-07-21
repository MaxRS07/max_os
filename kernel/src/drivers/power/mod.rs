use log::{info, warn};
use sdt::{fdt::GLOB_FDT, stream::FdtElement};

const POWEROFF: u32 = 0x5555;

/// Triggers a hardware shutdown. This will close QEMU
pub fn sys_poweroff() {
    if let Some(fdt) = GLOB_FDT.get()
        && let Some(e) = fdt.get_element("root/soc/test/reg")
        && let FdtElement::Property { value_ptr, len, .. } = e
    {
        let bytes = unsafe { core::slice::from_raw_parts(value_ptr, len) };
        let base = u64::from_be_bytes(bytes[0..8].try_into().unwrap()) as usize;

        let dev_ptr = base as *mut u32;
        unsafe { core::ptr::write_volatile(dev_ptr, POWEROFF) }
    } else {
        warn!("Something went wrong");
    }
    // wait for shutdown
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack, preserves_flags)) }
    }
}

#[macro_export]
macro_rules! off {
    () => {
        sys_poweroff();
    };
}
