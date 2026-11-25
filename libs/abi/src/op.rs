use crate::error::Errno;

pub enum MemOp {
    /// Maps a physical memory region into the virtual address space of the current process.
    ///
    /// ## Args
    ///
    /// * `virt_addr` - Target virtual memory address to map.
    /// * `size` - Size of the mapping in bytes.
    /// * `phys_addr` - Source physical address to map, or `0` to let the kernel allocate physical pages.
    /// * `flags` - Page flags
    /// * `perms` - Page permissions; `0`: Read, `1`: Write, `2`: Execute, 3: `User`
    Map,
    /// Unmaps a virtual memory region from this process, freeing the physical addresses
    ///
    /// ## Args
    ///
    /// * `virt_addr` - Target virtual memory address to unmap.
    /// * `size` - The number of bytes to unmap
    Unmap,
    Lock,
    Unlock,
}

impl TryFrom<usize> for MemOp {
    type Error = Errno;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Map,
            1 => Self::Unmap,
            2 => Self::Lock,
            3 => Self::Unlock,
            _ => return Err(Errno::InvalidOp),
        })
    }
}

/// Operations for creating, querying, and managing processes
pub enum ProcessOp {
    /// Returns the current PID
    CurrentPid = 0,
    /// Creates and registers a new blank process
    /// ## Args:
    /// 0. `name`: The pointer to the name str \
    /// 1. `name_len`: The length of the name in chars
    /// 2. ``
    Spawn = 1,
    /// Clones the current process into the process table, returning the clone's PID
    Fork = 2,
    /// Blocks this process until a child exits
    /// `pid>0`: waits for child with `pid` to exit
    /// `pid=0`: waits for any process to exit
    /// `pid=-1`:
    Wait = 3,
    /// Kills a process and terminates child threads
    Kill = 4,
}
impl TryFrom<usize> for ProcessOp {
    type Error = Errno;
    fn try_from(value: usize) -> Result<Self, Errno> {
        Ok(match value {
            0 => Self::CurrentPid,
            1 => Self::Spawn,
            2 => Self::Fork,
            3 => Self::Wait,
            4 => Self::Kill,
            _ => return Err(Errno::InvalidOp),
        })
    }
}
/// Threading and process control system call operations
pub enum ThreadOp {
    /// Spawns a thread
    /// ## args:
    /// `pid`: Parent process id, \
    /// `name`: address of string pointer \
    /// `name_len`: length of name string \
    /// `priority`: runqueue priority of the thread \
    /// `entry`: address of entry function for this thread
    Spawn,
    /// calls exit_current_thread!
    Exit,
    // /// Causes the
    // Sleep,
    /// Forces the current thread to pause execution and requeue, giving CPU time to the next ready thread
    Yield,
    /// Cancels a thread, deallocating the used resources and space. If the specified thread is currently running the next ready thread will replace it
    Cancel,
}
impl TryFrom<usize> for ThreadOp {
    type Error = Errno;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Spawn,
            1 => Self::Yield,
            2 => Self::Cancel,
            3 => Self::Exit,
            _ => return Err(Errno::InvalidOp),
        })
    }
}

/// File system operations
pub enum FsOp {
    Open,
    Read,
    Write,
    Close,
    Seek,
}
impl TryFrom<usize> for FsOp {
    type Error = Errno;
    fn try_from(value: usize) -> Result<Self, Errno> {
        Ok(match value {
            0 => Self::Open,
            1 => Self::Read,
            2 => Self::Write,
            3 => Self::Close,
            4 => Self::Seek,
            _ => return Err(Errno::InvalidOp),
        })
    }
}

/// Device operation syscalls
pub enum DevOp {
    Poweroff,
    Test,
}

impl TryFrom<usize> for DevOp {
    type Error = Errno;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Poweroff,
            1 => Self::Test,
            _ => return Err(Errno::InvalidOp),
        })
    }
}

pub enum ComOp {
    Pipe,
    Socket,
    Shmget,
    Semget,
    Msgget,
}

impl TryFrom<usize> for ComOp {
    type Error = Errno;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Pipe,
            1 => Self::Socket,
            2 => Self::Msgget,
            4 => Self::Semget,
            5 => Self::Msgget,
            _ => return Err(Errno::InvalidOp),
        })
    }
}
