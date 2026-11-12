use core::arch::naked_asm;

use crate::arch::riscv::csr::Csr::SATP;

#[repr(C)]
#[derive(Debug)]
pub struct ThreadContextSwitch {
    pub new_satp: Option<usize>,
    pub new_sp: *mut usize,
    pub old_sp_ptr: *mut *mut usize,
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
/// sditches context between threads by moving the stack pointer to the last executed instruction on the new thread
unsafe extern "C" fn thread_context_switch(old_sp: *mut *mut usize, new_sp: *mut usize) {
    naked_asm!(
        // shift stack pointer by 56 bytes, make room for registers
        "addi sp, sp, -56",
        // save thread's registers to its stack
        "sd ra,  52(sp)",
        "sd tp,  48(sp)",
        "sd s11, 44(sp)",
        "sd s10, 40(sp)",
        "sd s9,  36(sp)",
        "sd s8,  32(sp)",
        "sd s7,  28(sp)",
        "sd s6,  24(sp)",
        "sd s5,  20(sp)",
        "sd s4,  16(sp)",
        "sd s3,  12(sp)",
        "sd s2,  8(sp)",
        "sd s1,  4(sp)",
        "sd s0,  0(sp)",
        // save outgoing sp in thread
        // a0 contains old_thread.context.stack_pointer
        "sd sp, 0(a0)",
        // sditch stack pointer to new thread (a1)
        "mv sp, a1",
        // set satp and flush tlb to use new mappign :)
        "csrw satp, a2",
        "sfence.vma x0, x0",
        // load the new threads registers back
        "ld s0,  0(sp)",
        "ld s1,  4(sp)",
        "ld s2,  8(sp)",
        "ld s3,  12(sp)",
        "ld s4,  16(sp)",
        "ld s5,  20(sp)",
        "ld s6,  24(sp)",
        "ld s7,  28(sp)",
        "ld s8,  32(sp)",
        "ld s9,  36(sp)",
        "ld s10, 40(sp)",
        "ld s11, 44(sp)",
        "ld tp,  48(sp)",
        "ld ra,  52(sp)",
        // reset stack pointer back, can overwrite these registers
        "addi sp, sp, 56",
        // jump to next intruction
        "ret"
    );
}

fn switch_address_space(satp: usize) {
    unsafe {
        SATP.write(satp);
        core::arch::asm!("sfence.vma x0, x0", options(nostack));
    }
}

pub fn switch_to(context: ThreadContextSwitch) {
    if let Some(satp) = context.new_satp {
        switch_address_space(satp);
    }
    unsafe {
        thread_context_switch(context.old_sp_ptr, context.new_sp);
    }
}
