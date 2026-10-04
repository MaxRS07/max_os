use log::error;
use mem::{error::MemoryError, page_table::permissions::PagePermissions};

use crate::{
    sched::{PROCESS_TABLE, SCHEDULER},
    syscall::{
        context::KernelContext,
        error::SyscallError,
        op::{SysOp, exit},
    },
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
    fn call(&self, ktx: &mut super::context::KernelContext, args: super::SyscallArgs) {
        match self {
            Self::Map => {
                let (virt_addr, size, phys_addr, flags, perms, ..) = args;
                match mmap(ktx, virt_addr, size, phys_addr, flags, perms) {
                    Ok(_) => unsafe {
                        // write ok
                        core::arch::asm!("mv a0, x0");
                    },
                    Err(err) => unsafe {
                        error!("{err}");
                        core::arch::asm!("li a0, -1");
                    },
                }
            }
            Self::Unmap => {
                let (virt_addr, size, ..) = args;
                match munmap(ktx, virt_addr, size) {
                    Ok(_) => {
                        // write ok
                        exit(0);
                    }
                    Err(err) => {
                        error!("{err}");
                        exit(-1)
                    }
                }
            }
            Self::Lock => {}
            Self::Unlock => {}
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
    phys_addr: usize,
    flags: usize,
    perms: usize,
) -> Result<(), MemoryError> {
    if let Some(process) = ktx.process() {
        let perms = PagePermissions::from(perms as u8);
        return process.map_size(virt_addr, size, phys_addr, flags, perms);
    }
    Err(MemoryError::AccessViolation(
        "Failed to retrive current process",
    ))
}

fn munmap(ktx: &mut KernelContext, virt_addr: usize, size: usize) -> Result<(), MemoryError> {
    if let Some(process) = ktx.process() {
        process.unmap_size(virt_addr, size);
        return Ok(());
    }
    Err(MemoryError::AccessViolation(
        "Failed to retrive current process",
    ))
}

/// Locks pages
fn mlock() {}

fn munlock() {}
