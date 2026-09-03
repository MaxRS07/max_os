use core::arch::naked_asm;

use log::{debug, info, warn};
use sdt::fdt::{self, FDT, GLOBAL_FDT};

use crate::{
    arch::riscv::{
        csr::Csr::{MIE, MSTATUS, MTVEC, SEPC, SIE, SSTATUS, STVAL, STVEC},
        interrupt::{
            notifier::{INTERRUPT_NOTIFIER, InterruptNotifier},
            route::{ExceptionCode, InterruptCode, Trap, parse_scause},
        },
    },
    drivers::{power::sys_poweroff, time},
    sched::THREAD_QUEUE,
    syscall::handle_ecall,
};

const QTICK: u64 = 100_000_000;

pub fn init_trap_handler() {
    let sstatus_mie = 1 << 3;
    let sie_stie = 1 << 7;
    let sie_meie = 1 << 11;
    let sie_mask = sie_stie | sie_meie;

    schedule_interrupt_timer(u64::MAX); // max out timer to prevent timer irq before handler

    let trap_target = (trap_entry as *const () as usize) & !0x3;

    unsafe {
        STVEC.write(trap_target);
        SSTATUS.read_set(sstatus_mie);
        SIE.read_set(sie_mask);
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
/// Execeuted on interuppt.
/// `handler_addr`: function address for trap handling
unsafe extern "C" fn trap_entry() {
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
        "call rust_trap_handler",
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
        "sret"
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_trap_handler() {
    unsafe {
        if let Some(notifier) = INTERRUPT_NOTIFIER.get_mut() {
            let cause = parse_scause();
            match cause {
                Trap::Interrupt(int) => {
                    match int {
                        InterruptCode::MachineExternal => handle_plic_interrupt(notifier),
                        InterruptCode::MachineTimer => {
                            THREAD_QUEUE.get_mut().unwrap().handle_interrupt();
                            schedule_interrupt_timer(QTICK);
                        }
                        // match int {
                        // }
                        _ => (),
                    };
                }
                Trap::Exception(exception) => match exception {
                    ExceptionCode::EnvCallFromUMode => handle_ecall(),
                    _ => {
                        // There's no fixup/demand-paging path for any other
                        // exception, so resuming at the same sepc can never
                        // make progress - sret would just re-trap on the same
                        // instruction forever. Fail loudly instead.
                        let sepc = SEPC.read();
                        let stval = STVAL.read();
                        panic!(
                            "unhandled exception {:?} at sepc={:#x} stval={:#x}",
                            exception, sepc, stval
                        );
                    }
                },
            }
        } else {
            warn!("Notifier unset")
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
        let cur = time::mtime_raw(fdt.clint.base_address);
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
