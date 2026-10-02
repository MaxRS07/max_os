use crate::{
    sched::{PROCESS_TABLE, SCHEDULER},
    syscall::op::SysOp,
};

pub enum MemOp {
    /// Maps a physical memory region into the virtual address space of the current process.
    ///
    /// ## Args
    ///
    /// * `virt_addr` - Target virtual memory address to map.
    /// * `size` - Size of the mapping in bytes.
    /// * `phys_addr` - Source physical address to map, or `0` / `null` to let the kernel allocate physical pages.
    Map,
    /// Unmaps a virtual memory region from this process, freeing the physical addresses
    ///
    /// ## Args
    ///
    /// * `virt_addr` - Target virtual memory address to map.
    Unmap,
    Lock,
    Unlock,
}

impl SysOp for MemOp {
    fn call(&self, ktx: &mut super::context::KernelContext, args: super::SyscallArgs) {
        match self {
            Self::Map => {
                if let Some(process) = ktx.process() {
                    process
                }

            }
        }
        
    }
}

/// Allocates `size` bytes of memory and maps
fn mmap(virt_addr: usize, size: usize, phys_addr: usize) {
    if let Ok(process) = 
}

fn munmap() {}

/// Locks pages
fn mlock() {}

fn munlock() {}
