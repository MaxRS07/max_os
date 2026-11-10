use core::{
    cell::OnceCell,
    fmt::Error,
    ptr::{null, null_mut},
    sync::atomic::{AtomicPtr, Ordering, fence},
};

use alloc::{boxed::Box, fmt::format, format, vec::Vec};
use log::{debug, error, info, warn};
use mem::{
    error::MemoryError,
    page_table::{
        address_space::AddressSpace,
        asid::{ASID, ASIDBitmapSV32, MAX_ASID},
        table::Table,
        table_entry::TableEntry,
    },
};
use sync::{mutex::Mutex, mutex_guard::MutexGuard, oncelock::OnceLock};

use crate::{
    mm::{boot::map_boot_pages, heap::PAGE_ALLOCATOR},
    sched::{
        error::SchedError,
        process::{Process, ProcessTable},
        scheduler::Scheduler,
        thread::thread::{Thread, ThreadId},
    },
};

pub mod error;
pub mod process;
pub mod queue;
pub mod scheduler;
pub mod swap;
pub mod thread;

/// Global ASID allocator
pub static ASID_ALLOCATOR: OnceLock<Mutex<ASIDBitmapSV32>> = OnceLock::new();
/// This is the global thread scheduler
pub static SCHEDULER: OnceLock<Mutex<Scheduler>> = OnceLock::new();
/// Global process table
pub static PROCESS_TABLE: OnceLock<Mutex<ProcessTable>> = OnceLock::new();
/// The root kernel table, the base mapping will be copied onto all threads
pub static KERNEL_ROOT: OnceLock<usize> = OnceLock::new();

pub fn kernel_root() -> &'static Table {
    unsafe { &*(*KERNEL_ROOT.wait() as *const Table) }
}

/// intialize threading globals, setup main thread.
pub fn init_scheduler(stack_top: *const u8, stack_bottom: *const u8) -> Result<(), SchedError> {
    ASID_ALLOCATOR.set(Mutex::new(ASIDBitmapSV32::new([0; 8], MAX_ASID as usize)));
    SCHEDULER.set(Mutex::new(Scheduler::new()));
    PROCESS_TABLE.set(Mutex::new(ProcessTable::new()));
    let main_thread = Thread::main();

    let allocator = PAGE_ALLOCATOR.wait();
    let table =
        Table::alloc_empty(allocator).map_err(|error| SchedError::Other(format!("{error}")))?;
    if let Err(msg) = map_boot_pages(table) {
        error!("{msg}")
    }
    KERNEL_ROOT.set(table.addr());
    let kspace =
        AddressSpace::from_root(allocator, (*KERNEL_ROOT.wait()) as *mut Table, ASID::KERNEL);
    Process::kernel(main_thread, kspace)?;
    SCHEDULER.wait().lock().set_running(ThreadId::main())?;

    info!("Created main thread");
    Ok(())
}

pub fn spawn_thread(pid: u32, name: &str, entry: usize) -> Result<ThreadId, SchedError> {
    let id = PROCESS_TABLE
        .wait()
        .lock()
        .process_mut(pid)?
        .spawn_thread(name, entry)
        .ok_or(SchedError::Other(format!(
            "Failed to spawn thread:{name}@{entry:x} on process {pid}"
        )))?;
    let scheduled = SCHEDULER.wait().lock().schedule(id);
    if let Err(err) = scheduled {
        // cleanup on fail
        debug!("Failed to schedule thread, dropping {name}@{entry:x}");
        let _ = PROCESS_TABLE.wait().lock().drop_thread(id);
        return Err(err);
    }
    debug!("Scheduled thread {id:?}:{name}@{entry:x}");
    Ok(id)
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
/// Manually pauses the current thread and places it back in the queue as a ready thread
pub fn yield_thread() {
    let context = crate::sched::SCHEDULER.wait().lock().run_next();
    match context {
        Ok(c) => crate::sched::swap::switch_to(c),
        Err(err) => error!("Failed to yield thread: {err}"),
    }
}
