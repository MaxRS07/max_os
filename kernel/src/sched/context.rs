use core::arch::naked_asm;

use crate::arch::riscv::csr::Csr::SATP;

#[unsafe(naked)]
#[unsafe(no_mangle)]
/// Switches context between threads by moving the stack pointer to the last executed instruction on the new thread
pub unsafe extern "C" fn thread_context_switch(old_sp: *mut *mut usize, new_sp: *mut usize) {
    naked_asm!(
        // shift stack pointer by 56 bytes, make room for registers
        "addi sp, sp, -56",
        // save thread's registers to its stack
        "sw ra,  52(sp)",
        "sw tp,  48(sp)",
        "sw s11, 44(sp)",
        "sw s10, 40(sp)",
        "sw s9,  36(sp)",
        "sw s8,  32(sp)",
        "sw s7,  28(sp)",
        "sw s6,  24(sp)",
        "sw s5,  20(sp)",
        "sw s4,  16(sp)",
        "sw s3,  12(sp)",
        "sw s2,  8(sp)",
        "sw s1,  4(sp)",
        "sw s0,  0(sp)",
        // save outgoing sp in thread
        // a0 contains old_thread.context.stack_pointer
        "sw sp, 0(a0)",
        // switch stack pointer to new thread (a1)
        "mv sp, a1",
        // set satp and flush tlb to use new mappign :)
        "csrw satp, a2",
        "sfence.vma x0, x0",
        // load the new threads registers back
        "lw s0,  0(sp)",
        "lw s1,  4(sp)",
        "lw s2,  8(sp)",
        "lw s3,  12(sp)",
        "lw s4,  16(sp)",
        "lw s5,  20(sp)",
        "lw s6,  24(sp)",
        "lw s7,  28(sp)",
        "lw s8,  32(sp)",
        "lw s9,  36(sp)",
        "lw s10, 40(sp)",
        "lw s11, 44(sp)",
        "lw tp,  48(sp)",
        "lw ra,  52(sp)",
        // reset stack pointer back, can overwrite these registers
        "addi sp, sp, 56",
        // jump to next intruction
        "ret"
    );
}

pub fn switch_address_space(satp: usize) {
    unsafe {
        SATP.write(satp);
        core::arch::asm!("sfence.vma x0, x0", options(nostack));
    }
}

pub fn switch_to(satp: Option<usize>, old_sp: *mut *mut usize, new_sp: *mut usize) {
    if let Some(satp) = satp {
        switch_address_space(satp);
    }
    unsafe {
        thread_context_switch(old_sp, new_sp);
    }
}
