use core::arch::naked_asm;

use log::warn;

use crate::{
    arch::riscv::{
        csr::Csr::{MEPC, MIE, MIP, MSTATUS, MTVAL, MTVEC},
        interrupt::{
            handler::{QTICK, schedule_interrupt_timer},
            route::{ExceptionCode, InterruptCode, Trap, parse_mcause},
        },
    },
    sched::SCHEDULER,
    syscall::handle_ecall,
};

pub fn init_m_trap_handler() {
    let mie_stie = 1 << 7;
    let mie_mask = mie_stie;

    set_mtimecmp(u64::MAX);

    let trap_target = (m_trap_entry as *const () as usize) & !0x3;

    unsafe {
        MTVEC.write(trap_target);
        // MSTATUS.read_set(mstatus_mie);
        MIE.read_set(mie_mask);
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
/// Execeuted on interuppt.
/// `handler_addr`: function address for trap handling
unsafe extern "C" fn m_trap_entry() {
    // save registers
    naked_asm!(
        // Create 128 byte region for register saving
        "addi sp, sp, -128",
        "sw ra,  0(sp)",
        "sw tp,  4(sp)",
        "sw t0,  8(sp)",
        "sw t1,  12(sp)",
        "sw t2,  16(sp)",
        "sw a0,  20(sp)",
        "sw a1,  24(sp)",
        "sw a2,  28(sp)",
        "sw a3,  32(sp)",
        "sw a4,  36(sp)",
        "sw a5,  40(sp)",
        "sw a6,  44(sp)",
        "sw a7,  48(sp)",
        "sw t3,  52(sp)",
        "sw t4,  56(sp)",
        "sw t5,  60(sp)",
        "sw t6,  64(sp)",
        "sw s0,  68(sp)",
        "sw s1,  72(sp)",
        "sw s2,  76(sp)",
        "sw s3,  80(sp)",
        "sw s4,  84(sp)",
        "sw s5,  88(sp)",
        "sw s6,  92(sp)",
        "sw s7,  96(sp)",
        "sw s8,  100(sp)",
        "sw s9,  104(sp)",
        "sw s10, 108(sp)",
        "sw s11, 112(sp)",
        // Call actual handler
        "call m_trap_handler",
        // Load back saved registers
        "lw ra,  0(sp)",
        "lw tp,  4(sp)",
        "lw t0,  8(sp)",
        "lw t1,  12(sp)",
        "lw t2,  16(sp)",
        "lw a0,  20(sp)",
        "lw a1,  24(sp)",
        "lw a2,  28(sp)",
        "lw a3,  32(sp)",
        "lw a4,  36(sp)",
        "lw a5,  40(sp)",
        "lw a6,  44(sp)",
        "lw a7,  48(sp)",
        "lw t3,  52(sp)",
        "lw t4,  56(sp)",
        "lw t5,  60(sp)",
        "lw t6,  64(sp)",
        "lw s0,  68(sp)",
        "lw s1,  72(sp)",
        "lw s2,  76(sp)",
        "lw s3,  80(sp)",
        "lw s4,  84(sp)",
        "lw s5,  88(sp)",
        "lw s6,  92(sp)",
        "lw s7,  96(sp)",
        "lw s8,  100(sp)",
        "lw s9,  104(sp)",
        "lw s10, 108(sp)",
        "lw s11, 112(sp)",
        // reset stack pointer back, can overwrite these registers
        "addi sp, sp, 128",
        // jump to next intruction
        "mret"
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn m_trap_handler() {
    unsafe {
        let cause = parse_mcause();
        match cause {
            Trap::Interrupt(int) => {
                {
                    match int {
                        InterruptCode::MachineTimer => {
                            let now = (CLINT_BASE + 0xBFF8) as *const u64;
                            set_mtimecmp(now.read() + QTICK);
                            MIP.read_set(1 << 1);
                        }
                        // match int {
                        // }
                        _ => (),
                    };
                }
            }
            Trap::Exception(exception) => match exception {
                // ExceptionCode::EnvCallFromUMode => handle_ecall(),
                _ => {
                    let mepc = MEPC.read();
                    let mtval = MTVAL.read();
                    panic!(
                        "unhandled exception {:?} at mepc={:#x} mtval={:#x}",
                        exception, mepc, mtval
                    );
                }
            },
        }
    }
}
const CLINT_BASE: usize = 0x0200_0000;
const MTIMECMP: usize = CLINT_BASE + 0x4000; // hart 0

fn set_mtimecmp(value: u64) {
    let lo = MTIMECMP as *mut u32;
    let hi = (MTIMECMP + 4) as *mut u32;
    unsafe {
        hi.write_volatile(u32::MAX); // stop a spurious match while updating
        lo.write_volatile(value as u32);
        hi.write_volatile((value >> 32) as u32);
    }
}
