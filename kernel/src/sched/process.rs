// Manages a process

use core::sync::atomic::{AtomicU32, Ordering};

use alloc::{boxed::Box, string::String, vec::Vec};
use collections::hashmap::HashMap;
use mem::page_table::{address_space::AddressSpace, asid::ASID, table::Table};

use crate::{
    drivers::time::GLOBAL_TIME,
    mm::heap::PAGE_ALLOCATOR,
    sched::{
        ASID_ALLOCATOR, PROCESS_TABLE, SCHEDULER,
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
    threads: Vec<u32>,
}

impl Process {
    fn new(name: String, pid: u32, address_space: AddressSpace, threads: Vec<u32>) -> *mut Self {
        let raw_process = Box::into_raw(Box::new(Self {
            name,
            pid,
            address_space,
            threads,
        }));
        PROCESS_TABLE.wait().lock().insert(pid, raw_process);
        raw_process
    }
    /// creates a new proces
    pub fn spawn(name: String, entry: fn()) -> u32 {
        let asid_num = ASID_ALLOCATOR.wait().lock().next_free().unwrap(); // TODO Make this not unwrap
        let asid = ASID::new(asid_num as u16);
        let table = Table::alloc_empty(PAGE_ALLOCATOR.wait());
        let addr_space = AddressSpace::new(PAGE_ALLOCATOR.wait(), table.unwrap(), asid, Vec::new());
        let pid = PID_ALLOCATOR.fetch_add(1, Ordering::Relaxed);
        let thread = Thread::spawn(pid, &name, Priority::Normal, entry as usize).unwrap();
        Self::new(name, pid, addr_space, alloc::vec![thread]);
        pid
    }
    pub fn address_space(&self) -> &AddressSpace {
        &self.address_space
    }
    pub fn contains_thread(&self, tid: u32) -> bool {
        self.threads.contains(&tid)
    }
}

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}
