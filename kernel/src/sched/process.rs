// Manages a process

use alloc::vec::Vec;

use crate::sched::thread;

struct Process {
    threads: Vec,
}
