use core::arch::naked_asm;

// unsafe extern "C" {
//     /// Swaps hardware context, allowing the new thread to run. Must pass stack pointers directly to load them into a0, a1 registers
//     /// 1. `old_sp`: `*mut *mut usize` of the old threads context.stack_pointer (`a0`).
//     /// 2. `new_sp`: The raw address of the incoming thread's saved stack pointer (`a1`).
//     pub unsafe fn switch_context(old_sp: *mut *mut usize, new_sp: *mut usize);
// }

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn switch_context_impl(old_sp: *mut *mut usize, new_sp: *mut usize) {
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
        // a0 contains `old_thread.context.stack_pointer``
        "sw sp, 0(a0)",
        // switch stack pointer to new thread (a1)
        "mv sp, a1",
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
