use abi::op::MemOp;
use alloc::{format, string::ToString};
use log::error;
use mem::{error::MemoryError, page_table::permissions::PagePermissions};

use crate::{
    sched::{PROCESS_TABLE, SCHEDULER},
    syscall::{Syscall, context::KernelContext, error::SyscallError, op::SysOp},
    write_return,
};

impl SysOp for MemOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), SyscallError> {
        match self {
            Self::Map => {
                let (virt_addr, size, perms, ..) = args;
                let vaddr = mmap(ktx, virt_addr, size, perms)?;
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

/// Allocates `size` bytes of memory and maps
fn mmap(
    ktx: &mut KernelContext,
    virt_addr: usize,
    size: usize,
    perms: usize,
) -> Result<usize, SyscallError> {
    let mut process = ktx.process()?;
    let perms = PagePermissions::from(perms as u8 & 0b111);
    process
        .map_size(virt_addr, size, perms)
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
