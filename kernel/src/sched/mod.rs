use core::{
    cell::OnceCell,
    fmt::Error,
    sync::atomic::{Ordering, fence},
};

use alloc::{boxed::Box, format, vec::Vec};
use log::{debug, error, info, warn};
use mem::{
    error::MemoryError,
    page_table::{
        address_space::AddressSpace,
        asid::{ASID, ASIDBitmapSV32, MAX_ASID},
        table::Table,
    },
};
use sync::{mutex::Mutex, oncelock::OnceLock};

use crate::{
    mm::{boot::map_boot_pages, heap::PAGE_ALLOCATOR},
    sched::{
        error::ThreadError,
        process::{Process, ProcessTable},
        scheduler::Scheduler,
        thread::Thread,
    },
};

pub mod context;
pub mod error;
pub mod process;
pub mod queue;
pub mod scheduler;
pub mod thread;

/// Global ASID allocator
pub static ASID_ALLOCATOR: OnceLock<Mutex<ASIDBitmapSV32>> = OnceLock::new();
/// This is the global thread scheduler
pub static SCHEDULER: OnceLock<Mutex<Scheduler>> = OnceLock::new();
/// Global process table
pub static PROCESS_TABLE: OnceLock<Mutex<ProcessTable>> = OnceLock::new();

/// intialize threading globals, setup main thread.
pub fn init_scheduler(stack_top: *const u8, stack_bottom: *const u8) -> Result<(), ThreadError> {
    ASID_ALLOCATOR.set(Mutex::new(ASIDBitmapSV32::new([0; 8], MAX_ASID as usize)));
    SCHEDULER.set(Mutex::new(Scheduler::new()));
    PROCESS_TABLE.set(Mutex::new(ProcessTable::new()));
    match Thread::main(stack_top, stack_bottom) {
        Ok(tid) => {
            let allocator = PAGE_ALLOCATOR.wait();
            let table = Table::alloc_empty(allocator)
                .map_err(|error| ThreadError::Other(format!("{error}")))?;
            if let Err(msg) = map_boot_pages(table) {
                error!("{msg}")
            }
            let kspace = AddressSpace::new(allocator, table, ASID::MAIN, Vec::new());
            Process::kernel(tid, kspace).inspect_err(|err| error!("{err}"))?;
        }
        Err(error) => warn!("Failed to setup main thread: {}", error),
    }
    info!("Created main thread");
    Ok(())
}
/// Manually terminates the current thread
///
/// Panics if the global scheduler has not been initialized
#[macro_export]
macro_rules! terminate {
    () => {
        crate::sched::SCHEDULER.wait().lock().terminate_running();
    };
}

/// Manually pauses the
#[macro_export]
macro_rules! yield_thread {
    () => {
        crate::sched::SCHEDULER.wait().lock().yeild_thread();
    };
}
