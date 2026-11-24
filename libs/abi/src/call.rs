use crate::{error::Errno, op::*};

/// top 8 bits for identifing the call type
const SYSCALL_MASK: usize = 0xFF << (usize::BITS - 8);
/// lower 24 for function routing
const FN_MASK: usize = !SYSCALL_MASK;

/// Kernel operations that can be called with userspace interrupts.
///
/// # Types
/// ## 0 - Device
/// 0. Poweroff: Saves registers and performs poweroff sequence
/// ## 1 - Communication
/// ## 2 -
#[repr(usize)]
pub enum Syscall {
    /// physical device control
    Device(DevOp) = 0,
    /// allocation, mapping and physical memory locking
    Memory(MemOp) = 1,
    /// File IO & disk access
    FileSystem(FsOp) = 2,
    /// Threading operations
    Thread(ThreadOp) = 3,
    /// Process configuration and control
    Process(ProcessOp) = 4,
    /// network
    Communication(ComOp) = 5,
}

impl Syscall {
    pub fn from_call_fn(call_type: usize, fn_type: usize) -> Result<Self, Errno> {
        Ok(match call_type {
            0 => Syscall::Device(DevOp::try_from(fn_type)?),
            1 => Syscall::Memory(MemOp::try_from(fn_type)?),
            2 => Syscall::FileSystem(FsOp::try_from(fn_type)?),
            3 => Syscall::Thread(ThreadOp::try_from(fn_type)?),
            4 => Syscall::Process(ProcessOp::try_from(fn_type)?),
            5 => Syscall::Communication(ComOp::try_from(fn_type)?),
            _ => return Err(Errno::InvalidCall),
        })
    }
}
impl TryFrom<usize> for Syscall {
    type Error = Errno;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let call_type = (value & SYSCALL_MASK) >> (usize::BITS - 8);
        let fn_type = value & FN_MASK;

        Syscall::from_call_fn(call_type, fn_type)
    }
}
