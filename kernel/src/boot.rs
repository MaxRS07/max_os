use crate::{
    arch::riscv::{
        interrupt::{handler, machine::init_m_trap_handler},
        mode::enable_user_mode,
    },
    println,
};

#[unsafe(no_mangle)]
/// Pre kernel entry. Prepares M mode prologue, switches to S mode and calls `kernel_main`
pub extern "C" fn boot_entry(hart_id: usize, fdt_ptr: *const u8) {
    // boot init & sawp to s mode
    println!("Entering pre-kernel boot in M mode");
    init_m_trap_handler();
    enable_user_mode(hart_id, fdt_ptr);
}
