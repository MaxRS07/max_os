use core::{arch::naked_asm, ops::DerefMut, sync::atomic::Ordering};

use ::time::global_time::GlobalTimer;
use log::{debug, error, info, warn};
use sdt::fdt::{self, FDT, GLOBAL_FDT};

use crate::{
    arch::riscv::{
        csr::Csr::{MIE, MSTATUS, MTVEC, SEPC, SIE, SIP, SSTATUS, STVAL, STVEC},
        interrupt::{
            notifier::{INTERRUPT_NOTIFIER, InterruptNotifier},
            route::{ExceptionCode, InterruptCode, Trap, parse_cause, parse_scause},
        },
    },
    drivers::time::GLOBAL_TIME,
    sched::{SCHEDULER, context::switch_context_impl},
    syscall::handle_ecall,
};

pub const QTICK: u64 = 100_000;

pub fn init_s_trap_handler() {
    let sstatus_sie = 1 << 1;
    let sie_ssie = 1 << 1;
    let sie_meie = 1 << 9;
    let sie_mask = sie_meie | sie_ssie;

    let trap_target = (s_trap_entry as *const () as usize) & !0x3;

    unsafe {
        STVEC.write(trap_target);
        SSTATUS.read_set(sstatus_sie);
        SIE.read_set(sie_mask);
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
/// Execeuted on interuppt.
/// `handler_addr`: function address for trap handling
unsafe extern "C" fn s_trap_entry() {
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
        // move stack pointer starting at 20(sp) into register a0 for frame write access in syscall
        "mv a0,  sp",
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
        "csrr t0, sepc",
        "sw t0, 116(sp)",
        "csrr t0, sstatus",
        "sw t0, 120(sp)",
        "call s_trap_handler",
        // Load back saved registers
        "lw t0, 116(sp)",
        "csrw sepc, t0",
        "lw t0, 120(sp)",
        "csrw sstatus, t0",
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
        "sret"
    );
}

/// Frame is the address of the bottom of the stack pointer.
#[unsafe(no_mangle)]
pub extern "C" fn s_trap_handler(frame: *mut usize) {
    unsafe {
        let cause = parse_scause();
        match cause {
            Trap::Interrupt(int) => {
                match int {
                    InterruptCode::SupervisorExternal => {
                        let mut notifier = INTERRUPT_NOTIFIER.wait().lock();
                        handle_plic_interrupt(notifier.deref_mut())
                    }
                    InterruptCode::SupervisorSoftware => {
                        SIP.read_clear(1 << 1);
                        let run_next = SCHEDULER
                            .wait()
                            .try_lock()
                            .and_then(|mut sched| sched.handle_interrupt().ok());
                        if let Some((old_sp, new_sp, new_satp)) = run_next {
                            switch_context_impl(old_sp, new_sp, new_satp);
                        };
                    }
                    // match int {
                    // }
                    _ => (),
                };
            }
            Trap::Exception(exception) => match exception {
                ExceptionCode::EnvCallFromUMode => handle_ecall(frame),
                _ => {
                    let sepc = SEPC.read();
                    let stval = STVAL.read();
                    panic!(
                        "unhandled exception {:?} at sepc={:#x} stval={:#x}",
                        exception, sepc, stval
                    );
                }
            },
        }
    }
}

fn claim_plic(fdt: &FDT) -> u32 {
    let ptr = (fdt.plic.base_address + 0x200004) as *mut u32;
    unsafe { core::ptr::read(ptr) }
}

fn set_plic(fdt: &FDT, irq_id: u32) {
    let ptr = (fdt.plic.base_address + 0x200004) as *mut u32;
    unsafe { core::ptr::write_volatile(ptr, irq_id) }
}

fn handle_plic_interrupt(notifier: &mut InterruptNotifier) {
    if let Some(fdt) = GLOBAL_FDT.get() {
        let irq_id = claim_plic(fdt);
        if irq_id == 0 {
            return;
        }

        notifier.notify(irq_id);

        set_plic(fdt, irq_id);
    }
}
/// Time interrupt compare on hart 0
const MTIME_CMP_OFFSET: usize = 0x4000;

/// sets an interrupt to occur `time` cpu cycles from now
pub fn schedule_interrupt_timer(interval: u64) {
    if let Some(fdt) = GLOBAL_FDT.get() {
        let addr = fdt.clint.base_address + MTIME_CMP_OFFSET;
        let cur = GLOBAL_TIME.wait().ticks();
        let irq_time = interval + cur;

        let interval_low = (irq_time >> 32) as u32;
        let interval_high = irq_time as u32;

        let ptr_low = (addr + 4) as *mut u32;
        let ptr_high = addr as *mut u32;

        unsafe {
            core::ptr::write_volatile(ptr_low, 0xFFFFFFFF);
            core::ptr::write_volatile(ptr_low, interval_low);
            core::ptr::write_volatile(ptr_high, interval_high);
        }
    }
}
