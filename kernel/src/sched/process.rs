// Manages a process

use core::{
    ops::Div,
    ptr::{null, null_mut},
    sync::atomic::{AtomicU32, Ordering},
};

use alloc::{
    borrow::ToOwned,
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    fmt::format,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::{self, Vec},
};
use collections::hashmap::HashMap;
use fs::{collections::error::FSError, storage::fileobject::FileObject};
use mem::{
    error::MemoryError,
    page_table::{
        address_space::{AddressSpace, Addresser},
        asid::{ASID, ASIDAllocator},
        permissions::PagePermissions,
        table::Table,
    },
};
use sync::mutex::Mutex;

use crate::{
    drivers::time::GLOBAL_TIME,
    kernel_main,
    mm::heap::PAGE_ALLOCATOR,
    sched::{
        ASID_ALLOCATOR, KERNEL_ROOT, PROCESS_TABLE, SCHEDULER,
        error::SchedError,
        kernel_root,
        scheduler::CURRENT_PROCESS,
        spawn_thread,
        thread::{
            thread::{Priority, Thread, ThreadId},
            ustack::{PROCESS_STACK_TOP, UserStack},
        },
    },
};

pub static PROCESS_MAP_BASE: usize = 0x4000_0000;
pub static PID_ALLOCATOR: AtomicU32 = AtomicU32::new(0);
/// Process table for pid -> process lookup.
pub struct ProcessTable {
    table: BTreeMap<u32, Process>,
}

impl ProcessTable {
    pub fn new() -> Self {
        Self {
            table: BTreeMap::new(),
        }
    }
    pub fn thread_mut(&mut self, id: ThreadId) -> Result<&mut Thread, SchedError> {
        self.process_mut(id.pid)?.thread_mut(id.tid)
    }
    /// removes the process with pid `pid` from the table and deallocates all fields
    pub fn remove_process(&mut self, pid: u32) -> Result<(), SchedError> {
        self.table
            .remove(&pid)
            .map(|mut process| process.destroy())
            .ok_or(SchedError::BadPid(pid))
    }
    /// drops and cleans up the thread with id: `thread_id`, removing it from it's parent process. Returns [`None`] if the thread was not found.
    pub fn drop_thread(&mut self, id: ThreadId) -> Result<(), SchedError> {
        self.process_mut(id.pid)?
            .remove_thread(id.tid)
            .ok_or(SchedError::BadTid(id))
    }
    pub fn insert(&mut self, process: Process) -> Option<Process> {
        self.table.insert(process.pid, process)
    }
    pub fn process_mut(&mut self, pid: u32) -> Result<&mut Process, SchedError> {
        self.table.get_mut(&pid).ok_or(SchedError::BadPid(pid))
    }
    pub fn process(&mut self, pid: u32) -> Result<&Process, SchedError> {
        self.table.get(&pid).ok_or(SchedError::BadPid(pid))
    }
}

pub const MAX_FDS: u32 = 64;

#[derive(Debug)]
pub struct Process {
    name: String,
    /// process identifier
    pid: u32,
    /// address space that this process operates in
    address_space: AddressSpace,
    /// list of thread IDs owned by this process
    threads: Vec<Option<Thread>>,
    fds: Vec<Option<FileObject>>,
}

impl Process {
    fn new(name: &str, pid: u32, address_space: AddressSpace) -> Self {
        Self {
            name: name.to_owned(),
            pid,
            address_space,
            threads: Vec::new(),
            fds: Vec::new(),
        }
    }
    /// returns the next
    /// Creates the kernel process and adds it to the
    pub fn kernel(main_thread: Thread, address_space: AddressSpace) -> Result<u32, SchedError> {
        let threads = alloc::vec![main_thread];
        let process = Self::new("Kernel", 0, address_space);
        PROCESS_TABLE.wait().lock().insert(process);
        Ok(0)
    }
    /// Forks this process with a new name and entry, copying the address space
    pub fn fork(&self, name: &str, entry: usize) -> Result<u32, SchedError> {
        // Clone the address space
        let fork_asid = ASID_ALLOCATOR
            .wait()
            .lock()
            .alloc_id()
            .ok_or(SchedError::OutOfResources(format!("ASIDs exhausted")))?;
        let address_space = self
            .address_space
            .fork(kernel_root(), fork_asid)
            .map_err(|err| SchedError::OutOfResources(err.to_string()))?;
        let pid = alloc_pid();
        let process = Self::new(name, pid, address_space);
        spawn_thread(pid, name, entry)?;
        PROCESS_TABLE.wait().lock().insert(process);
        Ok(pid)
    }
    /// creates a new process
    pub fn spawn(name: &str, entry: usize) -> Result<u32, SchedError> {
        let asid = ASID_ALLOCATOR
            .wait()
            .lock()
            .alloc_id()
            .ok_or(SchedError::Other(format!("No free ASIDs")))?; // TODO Make this not unwrap
        let addr_space = AddressSpace::new(PAGE_ALLOCATOR.wait(), kernel_root(), asid, Vec::new())
            .map_err(|memerr| SchedError::Other(format!("{memerr}")))?;

        let pid = alloc_pid();
        let mut process = Self::new(name, pid, addr_space);
        let id = process
            .spawn_thread(name, entry)
            .ok_or(SchedError::Other(format!(
                "Failed to create entry thread for process"
            )))?;

        PROCESS_TABLE
            .wait()
            .lock()
            .insert(process)
            .ok_or(SchedError::Other(format!(
                "Failed to insert process with id {pid} into table, already exists"
            )))?;
        SCHEDULER.wait().lock().schedule(id)?;
        Ok(pid)
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn address_space(&self) -> &AddressSpace {
        &self.address_space
    }
    pub fn contains_thread(&self, tid: u32) -> bool {
        self.threads.iter().any(|thread| {
            if let Some(t) = thread {
                t.id.tid == tid
            } else {
                false
            }
        })
    }
    /// adds a new thread id to this thread's owned TIDs. If the thread ID is already present, returns [`Some`], returns [`None`] if the assignment was successful
    pub fn spawn_thread(&mut self, name: &str, entry: usize) -> Option<ThreadId> {
        let tid = match self.next_tid() {
            Some(id) => id,
            None => {
                self.threads.push(None);
                self.threads.len() - 1
            }
        };
        let user_stack = UserStack::alloc(tid, &mut self.address_space).ok()?;
        let thread = Thread::new(
            self.pid,
            tid as u32,
            name,
            Priority::Normal,
            user_stack,
            entry,
        )
        .ok()?;
        let id: ThreadId = thread.id;
        self.threads[tid] = Some(thread);
        Some(id)
    }
    pub fn next_tid(&self) -> Option<usize> {
        self.threads.iter().position(|t| t.is_none())
    }
    /// Removes a thread with the id `tid` from this process. Returns [`Some`] if the removal was successful or [`None`] if the process doesnt own `tid`
    pub fn remove_thread(&mut self, tid: u32) -> Option<()> {
        let thread = self
            .threads
            .iter_mut()
            .find(|t| t.as_ref().is_some_and(|t| t.id.tid == tid))?;
        if let Some(thread) = thread.as_mut() {
            thread.free_ustack(&mut self.address_space);
        }
        Some((*thread) = None)
    }

    pub fn thread_mut(&mut self, tid: u32) -> Result<&mut Thread, SchedError> {
        let id = ThreadId::new(self.pid, tid);
        self.threads
            .get_mut(tid as usize)
            .ok_or(SchedError::BadTid(id))?
            .as_mut()
            .ok_or(SchedError::BadTid(id))
    }
    pub fn thread(&mut self, tid: u32) -> Result<&Thread, SchedError> {
        let id = ThreadId::new(self.pid, tid);
        self.threads
            .get(tid as usize)
            .ok_or(SchedError::BadTid(id))?
            .as_ref()
            .ok_or(SchedError::BadTid(id))
    }

    pub fn destroy(&mut self) {
        let _ = self.address_space.destroy();
        ASID_ALLOCATOR
            .wait()
            .lock()
            .set_free(self.address_space.asid().value() as usize);
    }

    /* Address Space Helpers */

    /// Maps `size` bytes to this process's address space. If `phys_addr` is greater than 0, the kernel will attempt to map `virt_addr` to it.
    pub fn map_size(
        &mut self,
        virt_addr: usize,
        size: usize,
        flags: usize,
        perms: PagePermissions,
    ) -> Result<(), mem::error::MemoryError> {
        let pages = size.div_ceil(0x1000);
        let mut virt_addr = virt_addr;
        if virt_addr == 0 {
            virt_addr = self
                .address_space
                .find_free(pages, PROCESS_MAP_BASE, PROCESS_STACK_TOP)
                .ok_or(MemoryError::OutOfMemory(
                    "Failed to map pages to process, address space full",
                ))?;
        }
        self.address_space.map_pages(virt_addr, pages, flags, perms)
    }
    /// Unmaps `size` from this process's address space beggining at `virt_addr`
    pub fn unmap_size(&mut self, virt_addr: usize, size: usize) {
        // convert bytes to pages here to reduce sysop overhead
        let pages = size.div_ceil(0x1000);
        let _ = self.address_space.unmap_pages(virt_addr, pages);
    }

    /* File Descriptor Helpers */

    pub fn alloc_fd(&mut self, file: FileObject) -> Result<u32, SchedError> {
        let Some(fd) = self.fds.iter().position(|fo| fo.is_none()) else {
            return Err(SchedError::FileDescriptorsFull(format!(
                "Failed to allocate file descriptor, free slots exhausted"
            )));
        };
        self.fds[fd] = Some(file);
        Ok(fd as u32)
    }
    pub fn file_mut(&mut self, fd: u32) -> Result<&mut FileObject, SchedError> {
        if fd > MAX_FDS {
            return Err(SchedError::InvalidFileDescriptor);
        }
        match self.fds.get_mut(fd as usize) {
            Some(fo) => Ok(fo.as_mut().unwrap()),
            None => Err(SchedError::BadFileDescriptor(fd)),
        }
    }
    /// Returns the closed file object
    pub fn close_fd(&mut self, fd: u32) -> Result<(), SchedError> {
        if fd > MAX_FDS {
            return Err(SchedError::BadFileDescriptor(fd));
        }
        let fo = self.file_mut(fd)?;
        self.fds[fd as usize] = None;
        Ok(())
    }
}

fn alloc_pid() -> u32 {
    PID_ALLOCATOR.fetch_add(1, Ordering::Acquire)
}

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}
