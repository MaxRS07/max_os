// Manages a process

use alloc::{boxed::Box, vec::Vec};
use collections::hashmap::HashMap;
use mem::page_table::{address_space::AddressSpace, asid::ASID};

use crate::sched::{PROCESS_TABLE, SCHEDULER};

/// Process table for pid -> process lookup.
pub type ProcessTable = HashMap<u32, *mut Process>;

#[derive(Clone, Debug)]
pub struct Process {
    /// process identifier
    pid: u32,
    /// address space that this process operates in
    address_space: AddressSpace,
    /// list of thread IDs owned by this process
    threads: Vec<u32>,
}

impl Process {
    pub fn new(pid: u32, address_space: AddressSpace, threads: Vec<u32>) -> *mut Self {
        let raw_process = Box::into_raw(Box::new(Self {
            pid,
            address_space,
            threads,
        }));
        PROCESS_TABLE.wait().lock().insert(pid, raw_process);
        raw_process
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
