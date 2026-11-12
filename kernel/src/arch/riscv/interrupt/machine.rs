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
        "sd ra,  0(sp)",
        "sd tp,  4(sp)",
        "sd t0,  8(sp)",
        "sd t1,  12(sp)",
        "sd t2,  16(sp)",
        "sd a0,  20(sp)",
        "sd a1,  24(sp)",
        "sd a2,  28(sp)",
        "sd a3,  32(sp)",
        "sd a4,  36(sp)",
        "sd a5,  40(sp)",
        "sd a6,  44(sp)",
        "sd a7,  48(sp)",
        "sd t3,  52(sp)",
        "sd t4,  56(sp)",
        "sd t5,  60(sp)",
        "sd t6,  64(sp)",
        "sd s0,  68(sp)",
        "sd s1,  72(sp)",
        "sd s2,  76(sp)",
        "sd s3,  80(sp)",
        "sd s4,  84(sp)",
        "sd s5,  88(sp)",
        "sd s6,  92(sp)",
        "sd s7,  96(sp)",
        "sd s8,  100(sp)",
        "sd s9,  104(sp)",
        "sd s10, 108(sp)",
        "sd s11, 112(sp)",
        // Call actual handler
        "call m_trap_handler",
        // Load back saved registers
        "ld ra,  0(sp)",
        "ld tp,  4(sp)",
        "ld t0,  8(sp)",
        "ld t1,  12(sp)",
        "ld t2,  16(sp)",
        "ld a0,  20(sp)",
        "ld a1,  24(sp)",
        "ld a2,  28(sp)",
        "ld a3,  32(sp)",
        "ld a4,  36(sp)",
        "ld a5,  40(sp)",
        "ld a6,  44(sp)",
        "ld a7,  48(sp)",
        "ld t3,  52(sp)",
        "ld t4,  56(sp)",
        "ld t5,  60(sp)",
        "ld t6,  64(sp)",
        "ld s0,  68(sp)",
        "ld s1,  72(sp)",
        "ld s2,  76(sp)",
        "ld s3,  80(sp)",
        "ld s4,  84(sp)",
        "ld s5,  88(sp)",
        "ld s6,  92(sp)",
        "ld s7,  96(sp)",
        "ld s8,  100(sp)",
        "ld s9,  104(sp)",
        "ld s10, 108(sp)",
        "ld s11, 112(sp)",
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
