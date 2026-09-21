// Manages a process

use alloc::vec::Vec;

use crate::{mm::page_table::address_space::AddressSpace, sched::thread};

struct Process {
    /// process identifier
    pid: u32,
    /// address space that this process operates in
    asid: AddressSpace,
    /// list of thread IDs owned by this process
    threads: Vec<u32>,
}
