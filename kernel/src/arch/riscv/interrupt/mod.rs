use log::{Level::Info, info};
use sdt::fdt::FDT;

pub mod handler;
pub mod notifier;
pub mod route;

pub fn setup() {
    handler::init_trap_handler();
    notifier::init_notifier();
}

pub fn set_interrupt_priority(fdt: &FDT, device_id: u32, priority: u32) {
    unsafe {
        let plic_base = fdt.plic.base_address;

        let priority_reg = (plic_base + (device_id as usize * 4)) as *mut u32;
        priority_reg.write_volatile(priority);

        let enable_reg_offset = 0x2000 + ((device_id as usize / 32) * 4);
        let enable_reg = (plic_base + enable_reg_offset) as *mut u32;

        let current_enable = enable_reg.read_volatile();
        enable_reg.write_volatile(current_enable | (1 << (device_id % 32)));

        let threshold_reg = (plic_base + 0x200000) as *mut u32;
        threshold_reg.write_volatile(0);
    }
}
