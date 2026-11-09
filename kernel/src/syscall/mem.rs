use alloc::{format, string::ToString};
use log::error;
use mem::{error::MemoryError, page_table::permissions::PagePermissions};

use crate::{
    sched::{PROCESS_TABLE, SCHEDULER},
    syscall::{Syscall, context::KernelContext, error::SyscallError, op::SysOp},
    write_return,
};

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

impl SysOp for MemOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), SyscallError> {
        match self {
            Self::Map => {
                let (virt_addr, size, flags, perms, ..) = args;
                let vaddr = mmap(ktx, virt_addr, size, flags, perms)?;
                write_return!(ktx, vaddr);
                Ok(())
            }
            Self::Unmap => {
                let (virt_addr, size, ..) = args;
                munmap(ktx, virt_addr, size)
            }
            Self::Lock | Self::Unlock => Err(SyscallError::InvalidOperation("MemOp", 0)),
        }
    }
}

impl TryFrom<usize> for MemOp {
    type Error = SyscallError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Map,
            1 => Self::Unmap,
            2 => Self::Lock,
            3 => Self::Unlock,
            _ => return Err(SyscallError::InvalidOperation("MemOp", value)),
        })
    }
}

/// Allocates `size` bytes of memory and maps
fn mmap(
    ktx: &mut KernelContext,
    virt_addr: usize,
    size: usize,
    flags: usize,
    perms: usize,
) -> Result<usize, SyscallError> {
    let mut process = ktx.process()?;
    let perms = PagePermissions::from(perms as u8);
    process
        .map_size(virt_addr, size, flags, perms)
        .map_err(|err| SyscallError::Failed(err.to_string()))
}

fn munmap(ktx: &mut KernelContext, virt_addr: usize, size: usize) -> Result<(), SyscallError> {
    let mut process = ktx.process()?;
    process
        .unmap_size(virt_addr, size)
        .map_err(|memerr| SyscallError::Failed(format!("Memory error: {memerr}")))
}

/// Locks pages
fn mlock() {}

fn munlock() {}
