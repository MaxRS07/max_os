use core::{arch::asm, ops::DerefMut};

use alloc::string::ToString;
use log::warn;

use crate::{
    drivers::time::GLOBAL_TIME,
    fs::{GLOBAL_FS, call},
    syscall::{
        com::ComOp,
        context::{KernelContext, TrapFrame},
        device::DevOp,
        error::SyscallError,
        fs::FsOp,
        mem::MemOp,
        op::SysOp,
        process::ProcessOp,
        thread::ThreadOp,
    },
    write_return,
};

mod com;
mod context;
mod device;
mod error;
mod fs;
mod mem;
mod op;
mod process;
mod thread;

/// top 8 bits for identifing the call type
const SYSCALL_MASK: usize = 0xFF << (usize::BITS - 8);
/// lower 24 for function routing
const FN_MASK: usize = !SYSCALL_MASK;

/// arguments 1-6. these come from the a0-5 registers.
type SyscallArgs = (usize, usize, usize, usize, usize, usize);

/// Kernel operations that can be called with userspace interrupts.
///
/// # Types
/// ## 0 - Device
/// 0. Poweroff: Saves registers and performs poweroff sequence
/// ## 1 - Communication
/// ## 2 -
#[repr(usize)]
enum Syscall {
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

impl SysOp for Syscall {
    fn call(&self, ktx: &mut KernelContext, args: SyscallArgs) -> Result<(), SyscallError> {
        if let Err(err) = match self {
            Self::Device(op) => op.call(ktx, args),
            Self::Memory(op) => op.call(ktx, args),
            Self::FileSystem(op) => op.call(ktx, args),
            Self::Thread(op) => op.call(ktx, args),
            Self::Process(op) => op.call(ktx, args),
            Self::Communication(op) => op.call(ktx, args),
        } {
            // Propagate a0 write to here, write Syscall err pointer to following args
            let msg_ptr = err.to_string().as_ptr() as usize;
            write_return!(ktx, -1, msg_ptr);
            return Err(err);
        }
        Ok(())
    }
}
impl TryFrom<usize> for Syscall {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let call_type = (value & SYSCALL_MASK) >> (usize::BITS - 8);
        let fn_type = value & FN_MASK;

        Syscall::from_call_fn(call_type, fn_type)
    }
}
impl Syscall {
    pub fn from_call_fn(call_type: usize, fn_type: usize) -> Result<Self, SyscallError> {
        Ok(match call_type {
            0 => Syscall::Device(DevOp::try_from(fn_type)?),
            1 => Syscall::Memory(MemOp::try_from(fn_type)?),
            2 => Syscall::FileSystem(FsOp::try_from(fn_type)?),
            3 => Syscall::Thread(ThreadOp::try_from(fn_type)?),
            4 => Syscall::Process(ProcessOp::try_from(fn_type)?),
            5 => Syscall::Communication(ComOp::try_from(fn_type)?),
            _ => return Err(SyscallError::InvalidCall(call_type)),
        })
    }
}

pub fn handle_ecall(frame: *mut usize) {
    let ktx = &mut KernelContext::new(frame);
    let mut id = 0usize;
    let args = load_args(&mut id, ktx);
    match Syscall::try_from(id) {
        Ok(op) => op.call(ktx, args),
        Err(msg) => warn!("{msg}"),
    }
}
/// Loads the syscall registers from frame and returns them
pub fn load_args(id: &mut usize, ktx: &mut KernelContext) -> SyscallArgs {
    let frame = ktx.trap_frame();
    let [a0, a1, a2, a3, a4, a5, a6, ..] = frame[5..12];
    *id = frame[12];
    (a0, a1, a2, a3, a4, a5)
}
