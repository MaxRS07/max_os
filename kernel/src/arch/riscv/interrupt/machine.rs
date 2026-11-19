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
        "addi sp, sp, -256",
        "sd ra,  0(sp)",
        "sd tp,  8(sp)",
        "sd t0,  16(sp)",
        "sd t1,  24(sp)",
        "sd t2,  32(sp)",
        "sd a0,  40(sp)",
        "sd a1,  48(sp)",
        "sd a2,  56(sp)",
        "sd a3,  64(sp)",
        "sd a4,  72(sp)",
        "sd a5,  80(sp)",
        "sd a6,  88(sp)",
        "sd a7,  96(sp)",
        "sd t3,  104(sp)",
        "sd t4,  112(sp)",
        "sd t5,  120(sp)",
        "sd t6,  128(sp)",
        "sd s0,  136(sp)",
        "sd s1,  144(sp)",
        "sd s2,  152(sp)",
        "sd s3,  160(sp)",
        "sd s4,  168(sp)",
        "sd s5,  176(sp)",
        "sd s6,  184(sp)",
        "sd s7,  192(sp)",
        "sd s8,  200(sp)",
        "sd s9,  208(sp)",
        "sd s10, 216(sp)",
        "sd s11, 224(sp)",
        // Call actual handler
        "call m_trap_handler",
        // Load back saved registers
        "ld ra,  0(sp)",
        "ld tp,  8(sp)",
        "ld t0,  16(sp)",
        "ld t1,  24(sp)",
        "ld t2,  32(sp)",
        "ld a0,  40(sp)",
        "ld a1,  48(sp)",
        "ld a2,  56(sp)",
        "ld a3,  64(sp)",
        "ld a4,  72(sp)",
        "ld a5,  80(sp)",
        "ld a6,  88(sp)",
        "ld a7,  96(sp)",
        "ld t3,  104(sp)",
        "ld t4,  112(sp)",
        "ld t5,  120(sp)",
        "ld t6,  128(sp)",
        "ld s0,  136(sp)",
        "ld s1,  144(sp)",
        "ld s2,  152(sp)",
        "ld s3,  160(sp)",
        "ld s4,  168(sp)",
        "ld s5,  176(sp)",
        "ld s6,  184(sp)",
        "ld s7,  192(sp)",
        "ld s8,  200(sp)",
        "ld s9,  208(sp)",
        "ld s10, 216(sp)",
        "ld s11, 224(sp)",
        // reset stack pointer back, can overwrite these registers
        "addi sp, sp, 256",
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
