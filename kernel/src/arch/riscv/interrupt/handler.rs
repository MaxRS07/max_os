use core::{arch::naked_asm, ops::DerefMut, sync::atomic::Ordering};

use ::time::global_time::GlobalTimer;
use log::{debug, error, info, warn};
use mem::page_table::address_space::Addresser;
use sdt::fdt::{self, FDT, GLOBAL_FDT};

use crate::{
    arch::riscv::{
        csr::Csr::{SEPC, SIE, SIP, SSTATUS, STVAL, STVEC},
        interrupt::{
            notifier::{INTERRUPT_NOTIFIER, InterruptNotifier},
            route::{ExceptionCode, InterruptCode, Trap, parse_scause},
        },
    },
    drivers::time::GLOBAL_TIME,
    sched::SCHEDULER,
    syscall::handle_ecall,
};

pub const QTICK: u64 = 100_000;

pub fn init_s_trap_handler() {
    let sstatus_sie = 1 << 1;
    // Supervisor Software Interrupt Enable
    let sie_ssie = 1 << 1;
    // Supervisor External Interrupt Enable
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
        // Create 256-byte region for register saving (31 * 8 = 248 bytes, 16-byte aligned)
        "addi sp, sp, -256",
        "sd ra,   0(sp)",
        "sd tp,   8(sp)",
        "sd t0,  16(sp)",
        "sd t1,  24(sp)",
        "sd t2,  32(sp)",
        "sd a0,  40(sp)",
        "mv a0,  sp", // Pass TrapFrame pointer in a0
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
        // Save CSRs
        "csrr t0, sepc",
        "sd t0,  232(sp)",
        "csrr t0, sstatus",
        "sd t0,  240(sp)",
        // Call actual handler
        "call s_trap_handler",
        // Restore CSRs
        "ld t0,  232(sp)",
        "csrw sepc, t0",
        "ld t0,  240(sp)",
        "csrw sstatus, t0",
        // Load back saved registers
        "ld ra,   0(sp)",
        "ld tp,   8(sp)",
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
        // Reset stack pointer back
        "addi sp, sp, 256",
        // Jump to next instruction
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
                        let next = match SCHEDULER.wait().try_lock() {
                            Some(mut s) => s.handle_interrupt(),
                            None => return error!("Scheduler busy"),
                        };
                        match next {
                            Ok(context) => crate::sched::swap::switch_to(context),
                            Err(msg) => {} // error!("Failed to schedule next thread: {msg}"),
                        }
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
