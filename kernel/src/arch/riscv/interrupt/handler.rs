use core::{
    arch::{global_asm, naked_asm},
    ops::DerefMut,
    sync::atomic::Ordering,
};

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

global_asm!(include_str!("s_trap_entry.s"));

unsafe extern "C" {
    unsafe fn s_trap_entry();
}

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
