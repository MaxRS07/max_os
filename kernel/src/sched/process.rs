// Manages a process

use core::{
    ops::Div,
    ptr::{null, null_mut},
    sync::atomic::{AtomicU32, Ordering},
};

use alloc::{
    borrow::ToOwned, boxed::Box, collections::BTreeSet, fmt::format, format, string::String,
    vec::Vec,
};
use collections::hashmap::HashMap;
use mem::page_table::{
    address_space::{AddressSpace, Addresser},
    asid::{ASID, ASIDAllocator},
    permissions::PagePermissions,
    table::Table,
};

use crate::{
    drivers::time::GLOBAL_TIME,
    kernel_main,
    mm::heap::PAGE_ALLOCATOR,
    sched::{
        ASID_ALLOCATOR, PROCESS_TABLE, SCHEDULER,
        error::ThreadError,
        thread::{Priority, Thread},
    },
};

pub static PID_ALLOCATOR: AtomicU32 = AtomicU32::new(0);
/// Process table for pid -> process lookup.
pub type ProcessTable = HashMap<u32, *mut Process>;

#[derive(Clone, Debug)]
pub struct Process {
    name: String,
    /// process identifier
    pid: u32,
    /// address space that this process operates in
    address_space: AddressSpace,
    /// list of thread IDs owned by this process
    threads: BTreeSet<u32>,
}

impl Process {
    fn new(name: &str, pid: u32, address_space: AddressSpace, threads: BTreeSet<u32>) -> *mut Self {
        Box::into_raw(Box::new(Self {
            name: name.to_owned(),
            pid,
            address_space,
            threads,
        }))
    }
    /// Creates the kernel process and adds it to the
    pub fn kernel(main_tid: u32, address_space: AddressSpace) -> Result<u32, ThreadError> {
        let mut thread_set = BTreeSet::new();
        thread_set.insert(main_tid);
        let process = Self::new("Kernel", 0, address_space, thread_set);
        PROCESS_TABLE.wait().lock().insert(0, process);
        Ok(0)
    }
    /// creates a new proces
    pub fn spawn(name: &str, entry: *const fn()) -> Result<u32, ThreadError> {
        let asid = ASID_ALLOCATOR
            .wait()
            .lock()
            .alloc_id()
            .ok_or(ThreadError::Other(format!("No free ASIDs")))?; // TODO Make this not unwrap
        let table = Table::alloc_empty(PAGE_ALLOCATOR.wait())
            .map_err(|err| ThreadError::Other(format!("{err}")))?;
        let addr_space = AddressSpace::new(PAGE_ALLOCATOR.wait(), table, asid, Vec::new());

        let pid = PID_ALLOCATOR.fetch_add(1, Ordering::Relaxed);

        let thread = Thread::new(pid, &name, Priority::Normal, entry as usize);
        let tid = thread.id();
        let thread_ptr = Box::into_raw(Box::new(thread));

        let mut thread_set = BTreeSet::new();
        thread_set.insert(tid);
        let process = Self::new(name, pid, addr_space, thread_set);

        PROCESS_TABLE
            .wait()
            .lock()
            .insert(pid, process)
            .ok_or(ThreadError::Other(format!(
                "Failed to insert process with id {pid} into table, already exists"
            )));
        SCHEDULER.wait().lock().schedule(thread_ptr)?;
        Ok(pid)
    }
    pub fn address_space(&self) -> &AddressSpace {
        &self.address_space
    }
    pub fn contains_thread(&self, tid: u32) -> bool {
        self.threads.contains(&tid)
    }
    /// adds a new thread id to this thread's owned TIDs. If the thread ID is already present, returns [`Some`], returns [`None`] if the assignment was successful
    pub fn assign_thread(&mut self, tid: u32) -> Option<()> {
        if self.threads.contains(&tid) {
            return None;
        }
        self.threads.insert(tid);
        Some(())
    }
    /// Removes a thread with the id `tid` from this process. Returns [`Some`] if the removal was successful or [`None`] if the process doesnt own `tid`
    pub fn remove_thread(&mut self, tid: u32) -> Option<()> {
        if !self.threads.contains(&tid) {
            return None;
        }
        if self.threads.remove(&tid) {
            return Some(());
        }
        None
    }

    pub fn destroy(&mut self) {
        self.address_space.destroy();
    }

    /* Address Space Helpers */

    /// Maps `size` bytes to this process's address space.
    pub fn map_size(
        &mut self,
        virt_addr: usize,
        size: usize,
        _phys_addr: usize,
        flags: usize,
        perms: PagePermissions,
    ) -> Result<(), mem::error::MemoryError> {
        let pages = size.div_ceil(0x1000);
        self.address_space.map_pages(virt_addr, size, pages, perms)
    }
    /// Unmaps `size` from this process's address space beggining at `virt_addr`
    pub fn unmap_size(&mut self, virt_addr: usize, size: usize) {
        // convert bytes to pages here to reduce sysop overhead
        let pages = size.div_ceil(0x1000);
        self.address_space.unmap_pages(virt_addr, pages);
    }
}

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}
