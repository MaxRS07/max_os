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
use mem::page_table::{
    address_space::{AddressSpace, Addresser},
    asid::{ASID, ASIDAllocator},
    permissions::PagePermissions,
    table::Table,
};
use sync::mutex::Mutex;

use crate::{
    drivers::time::GLOBAL_TIME,
    kernel_main,
    mm::heap::PAGE_ALLOCATOR,
    sched::{
        ASID_ALLOCATOR, KERNEL_ROOT, PROCESS_TABLE, SCHEDULER,
        error::{ProcessError, ThreadError},
        kernel_root,
        scheduler::CURRENT_PROCESS,
        thread::thread::{Priority, Thread, ThreadId},
    },
};

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
    pub fn thread_mut<'a>(&'a mut self, id: ThreadId) -> Result<&'a mut Thread, ThreadError> {
        self.process_mut(id.pid)?.thread_mut(id.tid)
    }
    /// drops and cleans up the thread with id: `thread_id`, removing it from it's parent process. Returns [`None`] if the thread was not found.
    pub fn drop_thread(&mut self, id: ThreadId) -> Result<(), ThreadError> {
        self.process_mut(id.pid)?
            .remove_thread(id.tid)
            .ok_or(ThreadError::BadTid(id.tid))
    }
    pub fn insert(&mut self, process: Process) -> Option<Process> {
        self.table.insert(process.pid, process)
    }
    pub fn process_mut(&mut self, pid: u32) -> Result<&mut Process, ThreadError> {
        self.table.get_mut(&pid).ok_or(ThreadError::BadPid(pid))
    }
    pub fn process(&mut self, pid: u32) -> Result<&Process, ThreadError> {
        self.table.get(&pid).ok_or(ThreadError::BadPid(pid))
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
    pub fn kernel(main_thread: Thread, address_space: AddressSpace) -> Result<u32, ThreadError> {
        let threads = alloc::vec![main_thread];
        let process = Self::new("Kernel", 0, address_space);
        PROCESS_TABLE.wait().lock().insert(process);
        Ok(0)
    }
    /// Forks this process with a new name and entry, copying the address space
    pub fn fork(&self, name: &str, entry: *mut u32) -> Result<u32, ProcessError> {
        let pid = alloc_pid();
        // Clone the address space
        let fork_asid = ASID_ALLOCATOR
            .wait()
            .lock()
            .alloc_id()
            .ok_or(ProcessError::OutOfResources(format!("ASIDs exhausted")))?;
        let address_space = self
            .address_space
            .fork(kernel_root(), fork_asid)
            .map_err(|err| ProcessError::OutOfResources(err.to_string()))?;
        let pid = alloc_pid();
        let process = Self::new(name, pid, address_space);
        PROCESS_TABLE.wait().lock().insert(process);
        Ok(pid)
    }
    /// creates a new proces
    pub fn spawn(name: &str, entry: usize) -> Result<u32, ThreadError> {
        let asid = ASID_ALLOCATOR
            .wait()
            .lock()
            .alloc_id()
            .ok_or(ThreadError::Other(format!("No free ASIDs")))?; // TODO Make this not unwrap
        let addr_space = AddressSpace::new(PAGE_ALLOCATOR.wait(), kernel_root(), asid, Vec::new())
            .map_err(|memerr| ThreadError::Other(format!("{memerr}")))?;

        let pid = alloc_pid();
        let mut process = Self::new(name, pid, addr_space);
        process.spawn_thread(name, entry);
        let thread = Thread::new(pid, 0, &name, Priority::Normal, entry as usize)?;

        PROCESS_TABLE
            .wait()
            .lock()
            .insert(process)
            .ok_or(ThreadError::Other(format!(
                "Failed to insert process with id {pid} into table, already exists"
            )));
        SCHEDULER.wait().lock().schedule(thread.id)?;
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
    pub fn spawn_thread(&mut self, name: &str, entry: usize) -> Option<()> {
        let tid = match self.next_tid() {
            Some(id) => id,
            None => {
                self.threads.push(None);
                self.threads.len() - 1
            }
        };
        let thread = Thread::new(self.pid, tid as u32, name, Priority::Normal, entry).ok();
        self.threads.insert(tid, thread);
        Some(())
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
        Some((*thread) = None)
    }

    pub fn thread_mut(&mut self, id: u32) -> Result<&mut Thread, ThreadError> {
        self.threads
            .get_mut(id as usize)
            .ok_or(ThreadError::BadTid(id))?
            .as_mut()
            .ok_or(ThreadError::BadTid(id))
    }
    pub fn thread(&mut self, id: u32) -> Result<&Thread, ThreadError> {
        self.threads
            .get(id as usize)
            .ok_or(ThreadError::BadTid(id))?
            .as_ref()
            .ok_or(ThreadError::BadTid(id))
    }

    pub fn destroy(&mut self) {
        let _ = self.address_space.destroy();
        drop(&self.fds)
    }

    /* Address Space Helpers */

    /// Maps `size` bytes to this process's address space. If `phys_addr` is greater than 0, the kernel will attempt to map `virt_addr` to it.
    pub fn map_size(
        &mut self,
        virt_addr: usize,
        size: usize,
        _phys_addr: usize,
        flags: usize,
        perms: PagePermissions,
    ) -> Result<(), mem::error::MemoryError> {
        let pages = size.div_ceil(0x1000);
        self.address_space.map_pages(virt_addr, pages, flags, perms)
    }
    /// Unmaps `size` from this process's address space beggining at `virt_addr`
    pub fn unmap_size(&mut self, virt_addr: usize, size: usize) {
        // convert bytes to pages here to reduce sysop overhead
        let pages = size.div_ceil(0x1000);
        let _ = self.address_space.unmap_pages(virt_addr, pages);
    }

    /* File Descriptor Helpers */

    pub fn alloc_fd(&mut self, file: FileObject) -> Result<u32, ProcessError> {
        let Some(fd) = self.fds.iter().position(|fo| fo.is_none()) else {
            return Err(ProcessError::FileDescriptorsFull(format!(
                "Failed to allocate file descriptor, free slots exhausted"
            )));
        };
        self.fds[fd] = Some(file);
        Ok(fd as u32)
    }
    pub fn file_mut(&mut self, fd: u32) -> Result<&mut FileObject, ProcessError> {
        if fd > MAX_FDS {
            return Err(ProcessError::InvalidFileDescriptor);
        }
        match self.fds.get_mut(fd as usize) {
            Some(fo) => Ok(fo.as_mut().unwrap()),
            None => Err(ProcessError::BadFileDescriptor(fd)),
        }
    }
    /// Returns the closed file object
    pub fn close_fd(&mut self, fd: u32) -> Result<(), ProcessError> {
        if fd > MAX_FDS {
            return Err(ProcessError::BadFileDescriptor(fd));
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
