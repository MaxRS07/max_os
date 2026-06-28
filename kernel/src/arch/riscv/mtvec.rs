use core::arch::naked_asm;

pub fn init_trap_handler() {
    unsafe {
        core::arch::asm!("csrw mtvec, {}", in(reg) trap_handler as usize);
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn trap_handler() {
    // save registers
    naked_asm!("");
}
